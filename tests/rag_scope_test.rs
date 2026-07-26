use flowflow::infrastructure::llm::NotesTools;

/// A scoped chat must stay scoped, including when the agent searches on its own.
///
/// `search_notes` used to pass `None` as its id filter, so the first retrieval
/// respected the folder or thread and the agent's own re-search did not. The extra
/// notes reached the answer while the sources panel - built from the first
/// retrieval only - never showed them, which made the leak invisible.
#[test]
fn a_scoped_run_carries_its_ids_to_the_tools() {
    let ids = vec!["note-a".to_string(), "note-b".to_string()];
    let mounted = NotesTools::from_allowed(Some(&ids))
        .scope()
        .expect("scoped run mounts the notes tools");
    let carried = mounted.expect("a scoped run carries an id filter");
    assert_eq!(&*carried, ids.as_slice());
}

#[test]
fn a_global_run_carries_no_filter() {
    let mounted = NotesTools::from_allowed(None)
        .scope()
        .expect("global run mounts the notes tools");
    assert!(mounted.is_none(), "a global run must not narrow anything");
}

#[test]
fn a_run_without_notes_tools_mounts_nothing() {
    assert!(
        NotesTools::None.scope().is_none(),
        "None means the notes tools are not mounted at all"
    );
}

/// An empty scope is not the same as no scope: it means "these zero notes", and
/// must never widen back to the whole corpus.
#[test]
fn an_empty_scope_stays_empty_rather_than_becoming_global() {
    let mounted = NotesTools::from_allowed(Some(&[]))
        .scope()
        .expect("still mounts");
    let carried = mounted.expect("an empty scope is still a filter");
    assert!(carried.is_empty());
}
