use arrow_array::types::Float32Type;
use arrow_array::{
    FixedSizeListArray, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

async fn open_store(path: &str) -> Result<lancedb::Connection, lancedb::Error> {
    connect(path).execute().await
}

async fn create_table(
    db: &lancedb::Connection,
    table_name: &str,
    ids: Vec<String>,
    vectors: Vec<Vec<f32>>,
    texts: Vec<String>,
) -> Result<lancedb::Table, lancedb::Error> {
    let dim = vectors.first().map(|v| v.len()).unwrap_or(0);

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dim as i32,
            ),
            false,
        ),
        Field::new("text", DataType::Utf8, false),
    ]));

    let id_array = Arc::new(StringArray::from(ids));
    let vector_array =
        Arc::new(
            FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                vectors
                    .into_iter()
                    .map(|v| Some(v.into_iter().map(Some).collect::<Vec<_>>())),
                dim as i32,
            ),
        );
    let text_array = Arc::new(StringArray::from(texts));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![id_array, vector_array, text_array],
    )
    .unwrap();

    let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
    let reader: Box<dyn arrow_array::RecordBatchReader + Send> =
        Box::new(batches);
    db.create_table(table_name, reader).execute().await
}

async fn vector_search(
    db: &lancedb::Connection,
    table_name: &str,
    query_vector: Vec<f32>,
    top_k: usize,
) -> Result<Vec<RecordBatch>, lancedb::Error> {
    let table = db.open_table(table_name).execute().await?;
    let results: Vec<RecordBatch> = table
        .vector_search(query_vector)
        .map_err(|e| lancedb::Error::Runtime {
            message: e.to_string(),
        })?
        .limit(top_k)
        .execute()
        .await?
        .try_collect()
        .await?;
    Ok(results)
}

#[tokio::test]
async fn lancedb_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = open_store(tmp.path().to_str().unwrap()).await.unwrap();

    let ids = vec!["note-1".to_string(), "note-2".to_string()];
    let vectors = vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]];
    let texts = vec![
        "Réunion budget Q3".to_string(),
        "Appel client important".to_string(),
    ];

    create_table(&db, "chunks", ids, vectors, texts)
        .await
        .unwrap();

    let results = vector_search(&db, "chunks", vec![1.0, 0.0, 0.0, 0.0], 1)
        .await
        .unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].num_rows(), 1);
}

#[tokio::test]
async fn lancedb_multi_search() {
    let tmp = tempfile::tempdir().unwrap();
    let db = open_store(tmp.path().to_str().unwrap()).await.unwrap();

    let ids = vec![
        "note-1".to_string(),
        "note-2".to_string(),
        "note-3".to_string(),
    ];
    let vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.9, 0.1, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ];
    let texts = vec![
        "Budget réunion".to_string(),
        "Budget prévisionnel".to_string(),
        "Appel fournisseur".to_string(),
    ];

    create_table(&db, "chunks", ids, vectors, texts)
        .await
        .unwrap();

    let results = vector_search(&db, "chunks", vec![1.0, 0.0, 0.0, 0.0], 2)
        .await
        .unwrap();
    assert!(!results.is_empty());
    let total_rows: usize = results.iter().map(|b| b.num_rows()).sum();
    assert_eq!(total_rows, 2);
}
