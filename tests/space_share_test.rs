// Sharing a theme into a team (proposal 0004 T01). The push itself needs a
// backend and is validated on device; what stands alone is that nothing is
// marked, created or moved before the server has answered.

use flowflow::application::space::{
    resume_adoptions, share_existing_folder, ShareTarget, SpaceError,
};
use flowflow::domain::NewFolder;
use flowflow::infrastructure::persistence::Database;

fn db() -> (tempfile::TempDir, Database) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open_at(dir.path().join("flowflow.db")).unwrap();
    (dir, db)
}

fn folder(db: &Database, name: &str, parent: Option<&str>) -> String {
    db.create_folder(&NewFolder {
        name: name.into(),
        description: None,
        parent_id: parent.map(String::from),
    })
    .unwrap()
    .id
}

#[tokio::test]
async fn share_into_an_existing_team_needs_the_server_first() {
    let (_d, db) = db();
    db.upsert_space("team-1", "Atlas", true).unwrap();
    let parent = folder(&db, "Parent", None);
    let nested = folder(&db, "Nested", Some(&parent));

    let res = share_existing_folder(
        &db,
        &nested,
        ShareTarget::Existing("team-1".into()),
        false,
    )
    .await;

    assert_eq!(res, Err(SpaceError::NoBackend));
    let f = db.get_folder(&nested).unwrap().unwrap();
    assert_eq!(f.space_id, None, "nothing marked without the server");
    assert_eq!(f.parent_id.as_deref(), Some(parent.as_str()), "not moved");
    assert_eq!(db.list_spaces().unwrap().len(), 1, "no second space");
}

#[tokio::test]
async fn resume_without_a_server_marks_nothing() {
    let (_d, db) = db();
    db.upsert_space("team-1", "Atlas", true).unwrap();
    let root = folder(&db, "Shared", None);
    db.mark_folder_in_space(&root, "team-1", "rf1", "collab")
        .unwrap();
    let child = folder(&db, "Child", Some(&root));

    resume_adoptions(&db, "team-1").await;

    let c = db.get_folder(&child).unwrap().unwrap();
    assert_eq!(c.remote_id, None);
    assert_eq!(c.space_id, None);
}
