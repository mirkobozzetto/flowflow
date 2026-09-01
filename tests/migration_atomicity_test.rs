use flowflow::infrastructure::persistence::apply_migrations;
use rusqlite::Connection;

#[test]
fn failed_migration_rolls_back_data_and_version() {
    let conn = Connection::open_in_memory().expect("open database");
    conn.execute_batch(
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL);
         INSERT INTO notes (id, body) VALUES (1, 'kept');
         CREATE TABLE _migrations (version INTEGER PRIMARY KEY);
         INSERT INTO _migrations (version) VALUES (1);",
    )
    .expect("seed database");

    let result = apply_migrations(
        &conn,
        &[(2, "DELETE FROM notes; INSERT INTO notes VALUES (2, 'lost');
              SELECT * FROM missing_table;")],
    );

    assert!(result.is_err(), "migration must fail");
    let rows: Vec<(i64, String)> = conn
        .prepare("SELECT id, body FROM notes ORDER BY id")
        .expect("prepare notes")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query notes")
        .collect::<Result<_, _>>()
        .expect("read notes");
    assert_eq!(rows, vec![(1, "kept".to_owned())]);
    let versions: Vec<i64> = conn
        .prepare("SELECT version FROM _migrations ORDER BY version")
        .expect("prepare versions")
        .query_map([], |row| row.get(0))
        .expect("query versions")
        .collect::<Result<_, _>>()
        .expect("read versions");
    assert_eq!(versions, vec![1]);
}
