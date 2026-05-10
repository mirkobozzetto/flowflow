use crate::services::constants::{EMBEDDING_DIMS, VECTOR_TABLE_NAME};
use arrow_array::types::Float32Type;
use arrow_array::{
    Array, FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

pub struct Chunk {
    pub id: String,
    pub note_id: String,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub vector: Vec<f32>,
    pub title: String,
    pub tags: String,
    pub created_at: String,
}

pub struct SearchResult {
    pub chunk_text: String,
    pub note_id: String,
    pub title: String,
    pub distance: f32,
}

fn vectordb_path() -> String {
    #[cfg(target_os = "ios")]
    {
        let dir = crate::platform::ios::documents_dir();
        std::fs::create_dir_all(&dir).ok();
        dir.join("vectordb").to_string_lossy().to_string()
    }
    #[cfg(not(target_os = "ios"))]
    {
        let dir = std::env::temp_dir().join("flowflow");
        std::fs::create_dir_all(&dir).ok();
        dir.join("vectordb").to_string_lossy().to_string()
    }
}

fn chunks_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("note_id", DataType::Utf8, false),
        Field::new("chunk_text", DataType::Utf8, false),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIMS as i32,
            ),
            false,
        ),
        Field::new("title", DataType::Utf8, false),
        Field::new("tags", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

fn chunks_to_batch(chunks: &[Chunk]) -> RecordBatch {
    let schema = chunks_schema();

    let ids: Vec<&str> = chunks.iter().map(|c| c.id.as_str()).collect();
    let note_ids: Vec<&str> =
        chunks.iter().map(|c| c.note_id.as_str()).collect();
    let texts: Vec<&str> =
        chunks.iter().map(|c| c.chunk_text.as_str()).collect();
    let indices: Vec<i32> = chunks.iter().map(|c| c.chunk_index).collect();
    let titles: Vec<&str> = chunks.iter().map(|c| c.title.as_str()).collect();
    let tags: Vec<&str> = chunks.iter().map(|c| c.tags.as_str()).collect();
    let dates: Vec<&str> =
        chunks.iter().map(|c| c.created_at.as_str()).collect();

    let vector_array =
        Arc::new(
            FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                chunks.iter().map(|c| {
                    Some(c.vector.iter().copied().map(Some).collect::<Vec<_>>())
                }),
                EMBEDDING_DIMS as i32,
            ),
        );

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(note_ids)),
            Arc::new(StringArray::from(texts)),
            Arc::new(Int32Array::from(indices)),
            vector_array,
            Arc::new(StringArray::from(titles)),
            Arc::new(StringArray::from(tags)),
            Arc::new(StringArray::from(dates)),
        ],
    )
    .expect("Failed to create RecordBatch")
}

pub struct VectorStore {
    db: lancedb::Connection,
}

impl VectorStore {
    pub async fn open() -> Result<Self, String> {
        let path = vectordb_path();
        eprintln!("[vectordb] opening {path}");
        let db = lancedb::connect(&path)
            .execute()
            .await
            .map_err(|e| format!("VectorDB open: {e}"))?;
        eprintln!("[vectordb] ready");
        Ok(Self { db })
    }

    pub async fn store_chunks(&self, chunks: Vec<Chunk>) -> Result<(), String> {
        if chunks.is_empty() {
            return Ok(());
        }
        let note_id = &chunks[0].note_id;
        eprintln!(
            "[vectordb] storing {} chunks for note {note_id}",
            chunks.len()
        );

        let batch = chunks_to_batch(&chunks);
        let schema = chunks_schema();

        match self.db.open_table(VECTOR_TABLE_NAME).execute().await {
            Ok(table) => {
                let filter =
                    format!("note_id = '{}'", note_id.replace('\'', "''"));
                let _ = table.delete(&filter).await;
                let reader: Box<dyn arrow_array::RecordBatchReader + Send> =
                    Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));
                table
                    .add(reader)
                    .execute()
                    .await
                    .map_err(|e| format!("VectorDB add: {e}"))?;
            }
            Err(_) => {
                let reader: Box<dyn arrow_array::RecordBatchReader + Send> =
                    Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));
                self.db
                    .create_table(VECTOR_TABLE_NAME, reader)
                    .execute()
                    .await
                    .map_err(|e| format!("VectorDB create: {e}"))?;
            }
        }

        eprintln!("[vectordb] stored {} chunks", chunks.len());
        Ok(())
    }

    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, String> {
        let table = match self.db.open_table(VECTOR_TABLE_NAME).execute().await
        {
            Ok(t) => t,
            Err(_) => return Ok(vec![]),
        };

        let batches: Vec<RecordBatch> = table
            .vector_search(query_vector)
            .map_err(|e| format!("VectorDB search init: {e}"))?
            .limit(top_k)
            .execute()
            .await
            .map_err(|e| format!("VectorDB search exec: {e}"))?
            .try_collect()
            .await
            .map_err(|e| format!("VectorDB search collect: {e}"))?;

        let mut results = Vec::new();
        for batch in &batches {
            let text_col = batch
                .column_by_name("chunk_text")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let note_col = batch
                .column_by_name("note_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col = batch
                .column_by_name("title")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let dist_col = batch.column_by_name("_distance").and_then(|c| {
                c.as_any().downcast_ref::<arrow_array::Float32Array>()
            });

            let (text_col, note_col, title_col, dist_col) =
                match (text_col, note_col, title_col, dist_col) {
                    (Some(t), Some(n), Some(ti), Some(d)) => (t, n, ti, d),
                    _ => continue,
                };

            for i in 0..batch.num_rows() {
                results.push(SearchResult {
                    chunk_text: text_col.value(i).to_string(),
                    note_id: note_col.value(i).to_string(),
                    title: title_col.value(i).to_string(),
                    distance: dist_col.value(i),
                });
            }
        }

        eprintln!("[vectordb] found {} results", results.len());
        Ok(results)
    }

    pub async fn delete_note_chunks(
        &self,
        note_id: &str,
    ) -> Result<(), String> {
        let table = match self.db.open_table(VECTOR_TABLE_NAME).execute().await
        {
            Ok(t) => t,
            Err(_) => return Ok(()),
        };

        let filter = format!("note_id = '{}'", note_id.replace('\'', "''"));
        table
            .delete(&filter)
            .await
            .map_err(|e| format!("VectorDB delete: {e}"))?;

        eprintln!("[vectordb] deleted chunks for note {note_id}");
        Ok(())
    }
}
