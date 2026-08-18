// Activation resolution (08-activation), deterministic stage: exact alias/id fires direct,
// the keyword prefilter yields a single candidate for the judge, ambiguity or no match
// falls through. Trigger schema parses from the manifest (platform-authored words).

use flowflow::application::agent_activation::{
    note_goal, palette_entries, resolve_deterministic, Activation,
    PaletteAgent, Resolution,
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
    // A goal appended to the launch form still fires: the rest of the message is the goal.
    for msg in [
        "lance synchro-clients maintenant",
        "lance agent-crm quelle est ma liste de clients à appeler ?",
        "run synchro-clients, update dupont",
    ] {
        assert_eq!(
            resolve_deterministic(msg, std::slice::from_ref(&m)),
            expected,
            "message: {msg}"
        );
    }
    // The boundary protects sibling ids: `agent-crm` must not hijack `agent-crm-v2`,
    // and a glued word is not a match.
    for msg in ["lance agent-crm-v2 fais un truc", "lance agent-crmx"] {
        assert_eq!(
            resolve_deterministic(msg, std::slice::from_ref(&m)),
            Resolution::None,
            "message: {msg}"
        );
    }
    // A bare alias with trailing words stays exact-only (no launch verb, no fire).
    assert_eq!(
        resolve_deterministic(
            "synchro-clients maintenant",
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

    // One source for both + menus: the chat shows the launch command, a note shows the
    // description, and neither surface can drift from the other.
    let entries = palette_entries(&db);
    assert_eq!(entries.len(), 1);
    let agent = &entries[0];
    assert_eq!(agent.id, "agent-crm-sync");
    assert_eq!(agent.name, "CRM Sync");
    assert!(!agent.description.trim().is_empty());
    // The command targets the unique id, never the (collidable) alias.
    assert_eq!(agent.launch_command(), "lance agent-crm-sync");
}

#[test]
fn palette_hides_a_deactivated_agent() {
    let dir = tempfile::tempdir().unwrap();
    let db = flowflow::infrastructure::persistence::Database::open_at(
        dir.path().join("t.db"),
    )
    .unwrap();
    let verified = flowflow::domain::agent_manifest::verify_package(
        flowflow::application::connector_module::FIXTURE_PACKAGE,
        flowflow::domain::agent_manifest::ADMIN_PUBKEY,
    )
    .unwrap();
    db.install_agent(&verified).unwrap();
    db.set_agent_active("agent-crm-sync", false).unwrap();

    assert!(palette_entries(&db).is_empty());
}

#[test]
fn a_note_palette_tap_resolves_its_own_agent_and_leaves_the_goal_free() {
    let m = manifest("agent-crm-sync", "synchro-clients", "[]", ONE_CHAIN);
    let agent = PaletteAgent {
        id: "agent-crm-sync".into(),
        name: "CRM Sync".into(),
        description: "Keeps the CRM tidy.".into(),
    };

    // The tap resolves through the ordinary activation path, on the launch command alone.
    assert_eq!(
        resolve_deterministic(&agent.launch_command(), &[m]),
        Resolution::Direct(Activation {
            agent_id: "agent-crm-sync".into(),
            chain: "main".into(),
        })
    );

    // The run gets the note itself, framed as material. The launch command never travels as
    // the goal: it is how the agent is picked, not what it is asked to do.
    let note = "Client meeting: Dupont signed for 12 seats.";
    let goal = note_goal(note);
    assert!(goal.contains(note));
    assert!(!goal.contains(&agent.launch_command()));
    assert!(goal.starts_with("The text below is a personal note"));
}
