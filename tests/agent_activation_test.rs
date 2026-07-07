// Activation resolution (08-activation), deterministic stage: exact alias/id fires direct,
// the keyword prefilter yields a single candidate for the judge, ambiguity or no match
// falls through. Trigger schema parses from the manifest (platform-authored words).

use flowflow::application::agent_activation::{
    palette_entries, resolve_deterministic, Activation, Resolution,
};
use flowflow::domain::agent_manifest::{parse_manifest, AgentManifest};
use flowflow::domain::orchestration::parse_orchestration;

fn manifest(
    id: &str,
    alias: &str,
    triggers: &str,
    chains: &str,
) -> AgentManifest {
    let json = format!(
        r#"{{
        "schema_version": "1",
        "id": "{id}",
        "version": "1.0.0",
        "name": "Agent {id}",
        "description": "Keeps the {id} data tidy.",
        "alias": "{alias}",
        "model": "gpt-5.4-mini",
        "governance": {{
            "tools": [{{ "tool": "t_read", "mode": "read_only" }}],
            "limits": {{ "max_steps": 3, "max_tool_calls": 5 }}
        }},
        "orchestration": {{ "triggers": {triggers}, "chains": {chains} }}
    }}"#
    );
    parse_manifest(&json).expect("test manifest parses")
}

const ONE_CHAIN: &str = r#"{
    "main": {
        "initial": "read",
        "states": {
            "read":   { "allowed_tools": ["t_read"], "on_done": "answer" },
            "answer": { "terminal": true }
        }
    }
}"#;

#[test]
fn trigger_schema_parses_with_defaults() {
    let o = parse_orchestration(
        r#"{
            "triggers": [
                { "kind": "keyword", "words": ["synchro clients"], "then": "main" }
            ],
            "chains": {}
        }"#,
    )
    .unwrap();
    assert_eq!(o.triggers.len(), 1);
    let t = &o.triggers[0];
    assert_eq!(t.kind, "keyword");
    assert_eq!(t.scope, "in_run");
    assert!((t.confidence - 0.7).abs() < f32::EPSILON);
    assert_eq!(t.then, "main");

    // A triggerless orchestration (every pre-existing manifest) still parses.
    let legacy = parse_orchestration(r#"{ "chains": {} }"#).unwrap();
    assert!(legacy.triggers.is_empty());
}

#[test]
fn alias_and_id_fire_direct_case_insensitive() {
    let m = manifest("agent-crm", "synchro-clients", "[]", ONE_CHAIN);
    let expected = Resolution::Direct(Activation {
        agent_id: "agent-crm".into(),
        chain: "main".into(),
    });
    for msg in [
        "synchro-clients",
        "  Synchro-Clients  ",
        "lance synchro-clients",
        "Run synchro-clients",
        "exécute agent-crm",
    ] {
        assert_eq!(
            resolve_deterministic(msg, std::slice::from_ref(&m)),
            expected,
            "message: {msg}"
        );
    }
    // Near-misses never fire: extra words are not the exact form.
    assert_eq!(
        resolve_deterministic(
            "lance synchro-clients maintenant",
            std::slice::from_ref(&m)
        ),
        Resolution::None
    );
}

#[test]
fn keyword_prefilter_yields_single_candidate_for_the_judge() {
    let triggers = r#"[
        { "kind": "keyword", "words": ["mets a jour mes prospects"], "then": "main" }
    ]"#;
    let m = manifest("agent-crm", "synchro-clients", triggers, ONE_CHAIN);
    assert_eq!(
        resolve_deterministic(
            "Mets a jour mes prospects avec la réunion de lundi",
            std::slice::from_ref(&m)
        ),
        Resolution::Candidate(Activation {
            agent_id: "agent-crm".into(),
            chain: "main".into(),
        })
    );
    // An unrelated question falls through.
    assert_eq!(
        resolve_deterministic(
            "quelle heure est-il ?",
            std::slice::from_ref(&m)
        ),
        Resolution::None
    );
}

#[test]
fn ambiguous_keyword_match_falls_through() {
    let triggers = r#"[
        { "kind": "keyword", "words": ["prospects"], "then": "main" }
    ]"#;
    let a = manifest("agent-a", "", triggers, ONE_CHAIN);
    let b = manifest("agent-b", "", triggers, ONE_CHAIN);
    assert_eq!(
        resolve_deterministic("mes prospects du mois", &[a, b]),
        Resolution::None
    );
}

#[test]
fn non_in_run_or_non_keyword_triggers_never_fire() {
    let triggers = r#"[
        { "kind": "keyword", "words": ["prospects"], "scope": "background", "then": "main" },
        { "kind": "state",   "words": ["prospects"], "then": "main" }
    ]"#;
    let m = manifest("agent-a", "", triggers, ONE_CHAIN);
    assert_eq!(
        resolve_deterministic("mes prospects du mois", &[m]),
        Resolution::None
    );
}

#[test]
fn duplicate_alias_falls_through_instead_of_guessing() {
    let a = manifest("agent-a", "synchro-clients", "[]", ONE_CHAIN);
    let b = manifest("agent-b", "synchro-clients", "[]", ONE_CHAIN);
    assert_eq!(
        resolve_deterministic("lance synchro-clients", &[a.clone(), b]),
        Resolution::None
    );
    // The id stays unambiguous even when the alias collides.
    assert_eq!(
        resolve_deterministic(
            "lance agent-a",
            &[a, manifest("agent-b", "synchro-clients", "[]", ONE_CHAIN)]
        ),
        Resolution::Direct(Activation {
            agent_id: "agent-a".into(),
            chain: "main".into(),
        })
    );
}

#[test]
fn agent_without_a_runnable_chain_cannot_activate() {
    let m = manifest("agent-x", "go-x", "[]", "{}");
    assert_eq!(resolve_deterministic("lance go-x", &[m]), Resolution::None);
}

#[test]
fn palette_lists_installed_active_agents_with_launch_command() {
    let dir = tempfile::tempdir().unwrap();
    let db = flowflow::infrastructure::persistence::Database::open_at(
        dir.path().join("t.db"),
    )
    .unwrap();
    assert!(palette_entries(&db).is_empty());

    let verified = flowflow::domain::agent_manifest::verify_package(
        flowflow::application::connector_module::FIXTURE_PACKAGE,
        flowflow::domain::agent_manifest::ADMIN_PUBKEY,
    )
    .unwrap();
    db.install_agent(&verified).unwrap();

    // The command targets the unique id, never the (collidable) alias.
    let entries = palette_entries(&db);
    assert_eq!(
        entries,
        vec![("CRM Sync".to_string(), "lance agent-crm-sync".to_string())]
    );
}
