# Issue 101 - Block A trace

User approved grouped block A (2a + 2b), sequential execution with tested commits
and pushes on dev. No intermediate approval required. B/C design and native
consent decisions, provider configuration and deployment remain separate gates.

## Completed

- 2a: shared chat/note menu queries the existing backend connector list on mount.
  Google account status is not represented as tool/resource readiness.
  Loading, disconnected, unavailable and connected labels; check only on confirmed
  connection. Four library tests passed, including FR/EN label coverage.
  Command: `cargo test --lib ui::chat::connector_status::tests`.
  No device/browser or live account check performed.

## Block A implementation complete

- 2b: generic chat now uses a pool of connector-specific peers, filtered per
  owner, and supplies the same peer map to approval execution. Pool lifetime
  encloses the prompt. Closed/absent approval channels mount no connectors.
- A failed service, ambiguous names/prefixes or missing advertised tools falls
  back to native-only with a visible warning in the answer. The fallback does
  not retry through the weaker legacy alias.
- Backend route delivered in marketplace commit 0aba6e8:
  `/v1/chat/connectors/{slug}/mcp`; premium/device and connector access checks,
  mandatory active manifest, rejected batches/malformed calls. Existing agent
  route still requires x-agent-id; legacy route unchanged.
- Checks: 14 chat_surface tests, 10 agent_builder tests, 21 contract_hook tests,
  7 governance_approval tests passed. Backend: 3 proxy HTTP tests passed.
  Four 2a UI-state tests passed separately. No live provider/device/LLM test.
- First 2b run had 9 chat tests; final run added 5 ownership/routing regressions.
  Compiler emitted existing block future-compatibility and large unwind-table
  warnings, not failures. The test binaries actually ran.
- Source review and GitNexus scope agree on chat/MCP changes; graph process
  discovery is incomplete and its cached freshness hint disagrees with the
  current local index metadata. Do not interpret missing edges as safety proof.

## Impact and release constraints

- 2b: inspected chat surface, MCP pool and backend proxy. GitNexus impact of
  prompt_chat_agent is HIGH: two direct callers, three process groups (chat,
  notes, RAG). Preserve per-tool routing, approval channel and session lifetime.
- Backend connector slug route requires x-agent-id. Generic chat must not supply
  a fake agent or weaken that route. Legacy /v1/mcp is Google-only. Determine a
  separate authenticated generic route with explicit connector authorization.

## Preserve

Unrelated docs/proposals/0010-account-home-redesign/ remains untracked and untouched.
No main merge, live OAuth/service change, deployment or native consent change.

## Release follow-up requested by user

Before deployment, locate the `flowflow` connector linked to RMS and its
associated documentation, skill and MCP integration. Access/location are not yet
confirmed. Review and update affected contracts, examples and skill guidance for
the shipped changes; verify consistency before release. Do not assume current
MCP access includes RMS. This checklist item does not authorize deployment or
external service changes and does not pause block A.

## Delivery gate

After committing block A, open PR(s) from dev to main and inspect available CI.
Do not merge or deploy. Wait for the user's signal before block B, with binding
compatibility/migration and native-write consent decisions still required.
Deploy the new backend route before releasing this client; against an old
backend, connector setup fails visibly and chat is native-only. The old client
can continue using the unchanged legacy backend alias.

Manual release checks still needed: chat, note actions and RAG with a real test
account; approve/reject/cancel an external write; disconnect/reconnect; one failed
connector; return from settings and reopen both menus. No device installation
or successful live service run is claimed by unit/integration tests.

## Block B resumed - first reader protection

User approved resuming block B and preserving existing assistants/direct native
note/reminder actions. New native-write confirmation applies to new assistants.
Marketplace draft contract: 656ed39, BLOCK-B-CONTRACT.md.

FlowFlow now admits and executes only agent schema_version "1". Unknown versions
are rejected after package integrity verification, on stored manifest loading and
at build_agent_multi even if callers deserialize directly. Original canonical
JSON, unknown metadata fields and signed bytes are not rewritten.

Checks: agent_manifest_test 7 passed / 1 ignored (signature fixture generator),
agent_builder_test 10 passed, agent_activation_test 10 passed,
connector_module_test 37 passed. Total 64 passed. No production mutation.

This is only the first fail-closed reader protection, not schema-2 support.
Server distribution guard, precise new wire contract, scoped resource adapters
and multi-owner/native validation remain. No deployment or new PR authorized
for this incomplete block yet. Keep progress as small tested commits on dev.

Reader protections delivered: FlowFlow 33dde01 and marketplace e7f342c, both dev.
64 client tests and 23 backend packaging tests passed. New formats remain off.
Next contract question: new bindings per-device versus shared across linked
account devices. Current backend bind mutates global catalog config; the new
format must isolate identity and package digest before persistence changes.

## Block B - isolated resource adapter

Implemented `application::agent_bindings::scoped_resource_contracts` as a pure,
reader-first adapter. Its internal Rust types do not define or enable schema 2.
It joins device + agent + package digest + requirement key + resolved owner,
rejects shared legacy bounds and ambiguous routes, and gives each owner only its
own tools/resource. All returned entries must enter one ContractHook so limits
remain shared. Authenticated identity must come from the session/verified pin;
this pure adapter is not an authentication or persistence layer.

Eight targeted tests passed, including the real PromptHook seam proving scoped
resources and a shared call budget. External connectors claiming any of the five
native tool names are refused before routing (native tools bypass legacy gates).
Existing linker unwind-size and dependency future-compatibility warnings remain;
no failing check or runtime activation is concealed.

Legacy builder, stored bindings, signed bytes and direct native actions are
unchanged. Adapter is not yet wired to installation or execution. No schema-2
publication or migration enabled. Next: native descriptors and admission wiring.

User explicitly requires autonomous continuation, including while a separate
Prime diagnosis runs. Do not stop at RESUME.md or ask for a new window. Native
persistent goal is active; use checkpoints and compact.run() as needed.

## Block B - native capability admission

`application::agent_native` defines native descriptors from the five actual Tool
names, not fake connector/OAuth entries. A private-field plan admits only known,
mounted capabilities with compatible modes and no spreadsheet key semantics.
Create-note and schedule-reminder grants must declare require_approval and have
an available decision channel. Native-only reads need no external connector.
Four focused tests passed. This checks admission, not that an actual decision
was awaited; per-call native approval wiring and schema-2 execution remain off.
The earlier isolated resource adapter was pushed in c30077e (8 tests passed).

## Block B - opt-in native confirmation execution

Added `execute_native_with_confirmation`: invokes an admitted typed native tool
only after argument validation and, for writes, a real approval-registry decision.
Approved edits are re-deserialized and only the edited payload executes. Rejection,
expiry, invalid edits, dropped runs and a closed decision surface cause no call.
No background mutation task is detached. Existing direct native tools are intact.

Four targeted tests passed through the real approval registry and a recording
Tool implementation: exact original/edited payload executes once; no mutation
before approval or after refusal/expiry/cancel; pending decisions are removed;
closed surface and malformed arguments fail closed. No database/calendar write
was used to validate this seam.

This is not yet mounted in the app. The schema-2 integration must enforce shared
run budgets and route every declared native call through this path, never mount
its raw tool alongside it. Wire-format admission and per-device persistence
remain incomplete; schema-2 publication and deployment remain disabled.

## Block B - shared accounting and action stop

ScopedAgentRun now shares the existing RunState with external ContractHook
entries. Native calls check shared/per-tool limits before proposing and recheck
atomically before mutation after approval. Step/time accounting also works for
native-only runs. Rejection, cancellation and failed native execution stop later
external and native calls. Existing ContractHook constructors remain unchanged;
with_shared_run and abort_run are opt-in additions.

Targeted checks: agent_run_test 4, agent_native_execution_test 4 and
contract_hook_test 21 passed (29 total). Includes a budget consumed by an
external call while native approval is pending: the native call does not execute
and its card resolves Rejected. Legacy direct-note bypass tests still pass.

These seams remain unmounted. Runtime integration must not expose raw native
tools beside the confirmed path. Already-started provider mutations cannot be
rolled back by an abort; no transactional cancellation guarantee is claimed.

## Block B - combined owner validation

Added a pure capability_contract validator with matching backend/client test
cases. Every logical requirement must resolve exactly once with the declared
type and capabilities; all tool owners and routing prefixes are checked. Native
and mixed declarations are supported without fake connector records. Unknown,
ambiguous or ungranted tools and native writes lacking approval are refused.
Shared legacy resources and ownerless column policies are rejected.

Five targeted tests passed in each repository. An initial backend compile failure
exposed a client-only GovernanceError variant; fixed by deferring the resource
check on an ephemeral structural-validation copy, not matching enum variants.
The original runtime policy is not modified; binding/availability validation
remains mandatory before execution. Matching tests pass on both native builds.
No network adapter or schema-2 publication is enabled by this pure validator.

## Block B - combined run assembly

assemble_scoped_run now composes complete owner validation, native availability
and consent admission, isolated resource contracts and shared run accounting.
It does not create a partial run when any requirement fails. Two targeted tests
passed: native-only construction without fake connectors, and mixed construction
requiring every resource, the correct device identity and a live write-decision
surface. No network or storage mutation is performed by assembly.

This remains an internal integration seam, not installed-package dispatch.
Remaining: wire-format acceptance, versioned admission/dispatch and device-scoped
persistence before publication/runtime activation. Existing signed bytes and
legacy direct actions remain untouched.

## Block B - accepted schema-2 reader and local installed pin

User explicitly accepted separate schema 2, /v2 routes and device/package-scoped
bindings with no automatic legacy migration. Persistent goal resumed and verified
active. Publication, deployment and production mutations remain excluded.

Added a strict schema-2 reader using the existing signed-envelope verifier.
Unknown execution policy fields fail closed; original canonical metadata/bytes
remain pinned. Explicit local installation checks the requested id and refuses
to replace a legacy row. Reload checks the stored digest, id, version and active
state. Legacy verify_package and install_agent behavior remains unchanged.

Checks: scoped_agent_install_test 4 + agent_directory_test 6 passed. Native-only
signed fixture installed into an isolated SQLite file, database reopened, exact
canonical bytes and digest checked. Tampered input/row and automatic legacy
replacement refused. Initial test compilation failed due to passing &PathBuf to
open_at; corrected to path.clone(). No application/user database was modified.

Next: connect a loaded native-only pin to guarded execution, then versioned
network installation and isolated owner persistence. Local APIs do not imply
published schema-2 availability or completed application dispatch.

## Block B - native tools mounted through the actual Rig interface

Added ScopedNativeTool<T> and allowlisted mounting of real native implementations.
Every mounted native tool, including reads, routes through ScopedAgentRun.
The separate LlmClient::run_scoped_native surface supports existing configured
providers without modifying legacy run_agent or mounting raw tools alongside it.

Checks: scoped_native_tool_test 1 + agent_run_test 4 passed. The erased ToolDyn
interface used by Rig waits for a real approval, executes the edited typed args
once, and refuses a second call when the shared quota is spent. No provider API
or real note/calendar mutation was called by these tests. Installed-agent chain
dispatch and UI activation remain the next wiring step.

## Block B - installed native chain dispatch

Connected schema-2 installed native-only chains to run_agent_chain and the real
LlmClient native surface. The schema-1 branch is unchanged. External schema-2
requirements fail closed pending isolated owner connections; no legacy bound
resource or fabricated device identity is used. Native tools must use the same
database as the installed pin. A single verified row supplies the manifest and
a read-only recheck of canonical bytes, digest, version and active state before
proposals and immediately before invocation after approval.

Native time accounting excludes the union of overlapping approval waits and
resumes on cancellation. Actual tool execution remains charged. Native results
and refusals enter the chain trace so completed calls cannot become a false no-op.
The external read-before-write chain guard cannot be satisfied by a native read.

Checks: 13 existing focused tests passed (native execution 4, shared run 4,
scoped installation 4, Rig wrapper 1). Initial compilation found a missing
ChainError-to-String conversion in the new chain; fixed before rerunning.
Two disposable isolated-DB probes passed: deactivation during pending approval
caused zero mutations; overlapping waits were excluded once, followed by a real
compute-time budget refusal. Probe source retained in private session artifacts,
not added as permanent tests. No live provider call or real user note mutation.

Remaining: palette/activation support for installed schema 2, model-driven full
flow validation, /v2 network installation and isolated external owners. No
publication or deployment performed. UI/live end-to-end completion is not claimed.

## Block B - installed native activation and palettes

Installed schema-2 native-only agents now enter the existing chat/note palettes
and alias/id activation resolver through a metadata/trigger view. No schema-2
manifest is converted into a schema-1 executable manifest. The legacy public
deterministic resolver remains compatible. Unsupported external schema-2 owners,
chainless packages and invalid/inactive pins are excluded from activation.

Checks: all 10 existing activation tests passed. A disposable isolated-DB probe
also passed: signed schema-2 installation resolves alias/id to its chain, appears
in the palette without changing canonical bytes, and disappears after either
deactivation or digest tampering. No model/provider call or user data mutation.
Full model-driven chain validation and /v2 installation remain outstanding.

## Block B - local model-driven native flow verified

Verified a complete native execution core with a loopback-only OpenAI Responses
fixture: signed schema-2 package -> install -> database reopen -> alias activation
-> actual Rig model/tool exchange -> live edited approval -> real CreateNote tool
-> one note containing exactly the approved text -> terminal chain synthesis.
The first model request exposed only the declared create_note tool. The database
contained zero notes before confirmation. Native tool results now enter terminal
context directly, not only through the intermediate model's narrative.

The production entry retains LlmClient::from_db and its consent checks. A narrow
internal client factory lets the same execution core run against a local fixture.
The temporary probe used a fresh FLOWFLOW_DATA_DIR, a fictional key and disabled
embedding consent; no real provider, backend, user store or published package was
used. The single targeted probe passed. Source retained privately as
native-flow-probe.rs and removed from the repository after the check.

This proves the local execution core, not live /v2 download, entitlement checks,
production signing, rendered UI interaction or external owner isolation. Those
remain separate integration/release work. Publication and deployment stay off.

## Block B - mixed client execution and read admission

Connected package/device-local owner selections to v2 binding requests and scoped
MCP sessions. Native wrappers and external owner tools share the run budget.
Package/selection snapshots are rechecked after native and external approvals.
Resource-free owners require explicit null selections, not missing selections.

Schema-2 FSM read admission now counts gate-admitted external reads even without
a bound resource. Legacy read_any and per-resource write enforcement are unchanged.
Admission accounting is not a claim that a provider returned successful data.
Terminal synthesis errors propagate rather than silently returning an earlier reply.

Two disposable loopback probes passed: external search -> edited native approval
-> exact note creation, and the same flow with rejection -> zero notes. Nineteen
binding/budget/approval tests and 38 installation, directory, native execution,
assembly and governance regressions passed. No real provider or user store used.
Temporary probe sources are retained privately and removed from the repository.
GitNexus refreshed but returned missing symbols and implausible cross-language
callers; its critical shared-hook scope warning was checked against source and
targeted regressions, not treated as reliable proof of impact or safety.

Still not verified: a complete multi-external-owner model flow, selection changes
during an external approval in that flow, live v2 download, phone installation,
publication and deployment. This checkpoint does not complete block B.
