---
rfc: 0011
title: Multi-resource binding for the on-device governance gate
status: Accepted
date: 2026-06-24
author: Mirko Bozzetto
amends: RFC 0010 (agent behavior contract / governance)
---

# RFC 0011 - Multi-resource binding for the on-device governance gate

## 1. Problem

An installed agent can be armed to exactly one resource. `bound_resource` is
`{"spreadsheet_id": "X"}` and the gate requires every governed call to carry
that exact id. The user wants to arm an agent to **several** spreadsheets and
have the agent act on any of them, with each armed sheet shown by its real name
in the UI.

This crosses a boundary: the on-device governance gate (`domain/governance.rs`)
is a pure, security-critical module mirrored by the backend proxy. Widening the
bound from "one resource" to "a set of resources" changes the per-call check and
the `read_before_write` floor, so it is specified here before implementation.

## 2. Goals / Non-goals

Goals:
- Arm an agent to N resources; the gate allows a call targeting any armed one.
- Keep the safety floor honest: `read_before_write` must hold **per resource**,
  not "any read unlocks any write".
- Single-bind behavior stays byte-identical (pure backward compatibility).
- Names shown in the UI, never inside `bound_resource`.

Non-goals:
- No per-run session state on the backend. The backend proxy stays stateless.
- No change to the agent run shape: one chain run still targets one task; the
  gate simply permits any armed resource.
- Cross-resource transactions (read X, write Y atomically) are out of scope.

## 3. Current state (verified against code)

- `args_match_bound` (`src/domain/governance.rs:378`) does exact equality
  `a.get(k) == Some(v)`. Array binding is unimplemented.
- `RunState.read_bound: bool` (`governance.rs:319`), set globally on any `Read`
  (`governance.rs:542`), checked globally before a write (`governance.rs:502`).
- `validate_governance` gates write-grant-without-bound on
  `bound_resource.is_none()` only (`governance.rs:266`) - it does not inspect the
  bound's structure.
- FSM guard `chain.rs:86` reads `base.read_bound()` -> `RunState.read_bound`.
- App layer (`connector_module.rs`) is scalar-only: `bind_spreadsheet`,
  `unbind_spreadsheet`, `current_binding`, setting `armed_sheet_name` (singular).
- **Backend (decisive, verified):** `marketplace-flowflow/src/proxy.rs:121`
  calls only `gate_call_stateless` (`governance.rs:584`), the run-stateless
  subset. The backend does **not** enforce `read_before_write` today
  (`governance.rs:508`, `:580`: "the proxy is stateless across calls"). It has
  its own `args_match_bound` (`governance.rs:412`).

## 4. Proposed design - locked gate semantics

### 4.1 Representation

`bound_resource` stays a JSON object (or absent). Each pinned field's value is
either:
- a string (scalar) -> exact match, as today; or
- a non-empty array of strings -> membership ("one of").

`{"spreadsheet_id": ["X", "Y", "Z"]}` arms three sheets. Display names live in a
new setting `armed_sheet_names` = JSON map `id -> name`. Names never enter
`bound_resource`: a name key would become a pinned field the gate requires in
every call's args, denying everything.

Storage: `installed_agents.bound_json` already holds arbitrary JSON, so the array
form needs no migration. `NULL` still means unbound -> the manifest placeholder
stands -> off-bound denied (fail-closed).

### 4.2 `args_match_bound` (per-call bound check)

```rust
Some(a) => pinned.iter().all(|(k, v)| match (a.get(k), v) {
    (Some(av), serde_json::Value::Array(allowed)) => allowed.contains(av),
    (Some(av), scalar) => av == scalar,
    (None, _) => false,
}),
```

A non-object bound still pins nothing (`true`), but validation (4.4) now prevents
a scalar/non-object bound from reaching a write grant.

### 4.3 Per-resource `read_before_write`

`RunState.read_bound: bool` becomes `read_resources: BTreeSet<String>`.

The **resource key** of a call is built only from the bound field *names* (the
keys of `bound_resource`), read out of `call.args`, sorted, serialized compact.
A `BTreeMap` gives the canonical sorted order, which closes the read-skip attack
(reordered keys must not produce a different key):

```rust
fn resource_key(call_args: &serde_json::Value, bound: &serde_json::Value) -> String {
    let (Some(args), Some(b)) = (call_args.as_object(), bound.as_object())
        else { return String::new() };
    let mut k = std::collections::BTreeMap::new();
    for name in b.keys() {
        if let Some(val) = args.get(name) { k.insert(name.clone(), val.clone()); }
    }
    serde_json::to_string(&k).unwrap_or_default()
}
```

In `gate`:
- on Allow of an `Action::Read`: `run.read_resources.insert(resource_key(...))`;
- on a write under `read_before_write`: deny `ReadBeforeWrite` unless
  `run.read_resources.contains(&resource_key(...))`.

`check_bound` runs before tracking (unchanged order: allowlist -> mode/columns ->
bound -> read_before_write -> destructive -> limits), so an off-bound read is
denied and never poisons the read set. Single-bind degenerates to a one-element
set: identical behavior to today.

### 4.4 Validation (`validate_governance`)

Add a structural pass over `bound_resource`. It must be an object; each field
value must be a string or a non-empty array of strings. Reject (new
`GovernanceError::MalformedBoundResource { field, reason }`): empty array,
numbers/nulls/booleans, nested objects, mixed-type arrays. The write-grant check
changes from `bound_resource.is_none()` to "bound is not a well-formed non-empty
object", so a scalar or malformed bound under a write grant fails at install.

### 4.5 FSM guard stays coarse

`chain.rs:86` `base.read_bound()` -> `base.read_any()` (`!read_resources
.is_empty()`), with a comment: this is a best-effort early block ("some bound
resource was read"); the gate enforces the precise per-resource rule and will
still deny a write to an unread sibling even after FSM entry.

### 4.6 `merge_bound` - additive union

Multi-bind is cumulative (the user adds sheets). When both the manifest bound and
the arm-time binding are arrays, `merge_bound` UNIONs them and drops the
`"bound-at-install"` placeholder. Scalar/array mixes replace wholesale. The
merged result is validated before `build_agent`.

## 5. Backend enforcement split (decisive)

Per-resource `read_before_write` is **device-only**, and this is **not new
drift**: the backend proxy already enforces only the stateless subset and has
never enforced `read_before_write` or budgets (verified, section 3). Multi-bind
does not widen what the backend is responsible for.

Verified further: the backend's agent catalog seed (`marketplace-flowflow/src/db.rs`,
`agent-crm-sync` config) ships its governance with NO `bound_resource`, so the proxy
does not enforce a bound for this agent at all today (a `None` bound matches every
call). Bound enforcement is already entirely device-side; multi-bind changes nothing
there. The array-membership upgrade to the backend `args_match_bound`
(`marketplace-flowflow/src/domain/governance.rs:412`) is therefore forward-safety
(applied to keep the two copies byte-identical per the "agreed once" invariant) and
not a load-bearing fix today. It becomes load-bearing only if a future catalog agent
ships an array-valued `bound_resource`. No per-run session token, no backend state.

The contract comment block on both copies (`governance.rs:5-9`) is amended to
state explicitly: the stateless subset (allowlist, mode, column_roles,
bound_resource, destructive) is dual-enforced and must not drift;
`read_before_write` and budgets are device-only and the proxy does not enforce
them.

## 6. Security review (adversarial pass, resolved)

Four independent reviewers stressed the design. Resolved holes:
- Read-skip via key reordering -> canonical sorted resource key (4.3).
- Scalar/empty/mixed-type bound bypassing the bound floor -> structural
  validation at install (4.4).
- "Backend can't do per-resource read_before_write -> drift" -> framing error:
  read_before_write is already device-only; only `args_match_bound` parity is
  owed (section 5).
- Coarse FSM guard permitting write-state entry before the target is read -> kept
  coarse by design; the gate is the precise enforcer (4.5).

## 7. Implementation plan

| id | title | files | deps |
|----|-------|-------|------|
| T1 | `args_match_bound`: array membership + scalar equality | `src/domain/governance.rs` | - |
| T2 | `validate_governance`: structural validation of `bound_resource`; new `MalformedBoundResource`; write-grant check requires well-formed non-empty object | `src/domain/governance.rs` | - |
| T3 | `RunState.read_bound` -> `read_resources: BTreeSet<String>`; add `resource_key()`; gate inserts on Read, checks membership on write | `src/domain/governance.rs` | T1 |
| T4 | ContractHook `read_bound()` -> `read_any()` | `src/application/tools/mod.rs` | T3 |
| T5 | FSM guard -> `read_any()` + doc comment | `src/application/chain.rs` | T4 |
| T6 | `merge_bound`: union arrays, drop placeholder, validate merged result | `src/application/agent_builder.rs` | T2 |
| T7 | App layer multi-bind: `current_bindings -> Vec<(id,name)>`, `bind_add`, `unbind_remove`, `armed_sheet_names` map; keep single-id shims for the current UI | `src/application/connector_module.rs` | T6 |
| T8 | Backend parity: array-membership in backend `args_match_bound`; amend contract comment on both copies | `marketplace-flowflow/src/domain/governance.rs`, `src/domain/governance.rs` | T1 |
| T9 | UI: accordion, list of armed sheets (name + remove + open), add via list or URL | `src/ui/settings/connections.rs` | T7 |
| T10 | Tests (section 8) | `tests/governance_test.rs`, `tests/agent_builder_test.rs`, `tests/connector_module_test.rs` | T1-T7 |

## 8. Backward-compat + tests

Backward-compat (must hold):
- Scalar bound + scalar args: unchanged Allow/Deny. Existing governance tests
  pass with no semantic change.
- The `RunState` field rename touches the tests that set `read_bound: true` or
  assert it; update them to seed/inspect `read_resources`.
- DB: `bound_json` needs no migration; `armed_sheet_name` -> `armed_sheet_names`
  is additive (display-only, no migration).
- `FIXTURE_PACKAGE` stays scalar `"bound-at-install"`; no re-sign. Multi-bind is
  exercised by arm-time binding, not the shipped manifest.

Tests to add:
- gate: `array_bound_allows_member`, `array_bound_denies_non_member`,
  `per_resource_read_does_not_satisfy_sibling`, `resource_key_is_order_independent`,
  `single_bind_unchanged` (regression), `validate_rejects_empty_array` /
  `_mixed_types` / `_scalar_bound_under_write`.
- agent_builder: `merge_bound` unions two arrays and drops the placeholder; merged
  result passes validation.
- connector_module: `bind_add` scalar->array append+dedup; `unbind_remove` clears
  field to NULL when last id removed; `current_bindings` returns plural pairs with
  names from `armed_sheet_names`.

## 9. Open questions

- Name fetch for URL-pasted sheets: the real title can be fetched via a direct
  `get_spreadsheet` call (bypasses the gate exactly like `arm_list_spreadsheets`).
  In scope for the UI (T9) so a URL-armed sheet shows its name instead of its id.
- Backend parity (T8) lives in a separate repo and must merge before, or with,
  the device change to avoid the bound-check drift in section 5.
