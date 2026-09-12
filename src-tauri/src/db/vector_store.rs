//! LanceDB wrapper — opens `resources/lexuz.db` read-only at runtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, DistanceType};
use tracing::info;

#[derive(Debug, Clone)]
pub struct RetrievedChunk {
    pub id: String,
    pub text: String,
    pub law_name: String,
    pub article_number: String,
    pub chapter: String,
    pub section_title: String,
    pub score: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum VectorStoreError {
    #[error("Vector store not available: {0}")]
    NotReady(String),
    #[error("Search failed: {0}")]
    Search(String),
}

pub struct VectorStore {
    path: PathBuf,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl VectorStore {
    pub fn open_readonly(path: impl AsRef<Path>) -> Result<Self, VectorStoreError> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(VectorStoreError::NotReady(format!(
                "Missing lexuz DB at {}",
                path.display()
            )));
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("faqih-lancedb")
            .worker_threads(2)
            .build()
            .map_err(|e| VectorStoreError::NotReady(e.to_string()))?;

        // Smoke-open to fail fast if the table is missing.
        runtime.block_on(async {
            let db = connect(path.to_string_lossy().as_ref())
                .execute()
                .await
                .map_err(|e| VectorStoreError::NotReady(e.to_string()))?;
            let names = db
                .table_names()
                .execute()
                .await
                .map_err(|e| VectorStoreError::NotReady(e.to_string()))?;
            if !names.iter().any(|n| n == "lexuz_articles") {
                return Err(VectorStoreError::NotReady(
                    "table lexuz_articles not found in lexuz.db".into(),
                ));
            }
            Ok::<(), VectorStoreError>(())
        })?;

        info!(path = %path.display(), "Opened LanceDB lexuz store (read-only usage)");
        Ok(Self {
            path,
            runtime: Arc::new(runtime),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn search(
        &self,
        query_vector: &[f32],
        top_k: usize,
    ) -> Result<Vec<RetrievedChunk>, VectorStoreError> {
        let top_k = top_k.max(1);
        let path = self.path.clone();
        let vector = query_vector.to_vec();

        self.runtime.block_on(async move {
            let db = connect(path.to_string_lossy().as_ref())
                .execute()
                .await
                .map_err(|e| VectorStoreError::Search(e.to_string()))?;
            let table = db
                .open_table("lexuz_articles")
                .execute()
                .await
                .map_err(|e| VectorStoreError::Search(e.to_string()))?;

            let stream = table
                .vector_search(vector)
                .map_err(|e| VectorStoreError::Search(e.to_string()))?
                .column("embedding_vector")
                .distance_type(DistanceType::Cosine)
                .limit(top_k)
                .execute()
                .await
                .map_err(|e| VectorStoreError::Search(e.to_string()))?;

            let batches = stream
                .try_collect::<Vec<_>>()
                .await
                .map_err(|e| VectorStoreError::Search(e.to_string()))?;

            let mut out = Vec::new();
            for batch in batches {
                let ids = string_col(&batch, "id")?;
                let texts = string_col(&batch, "text")?;
                let laws = string_col(&batch, "law_name")?;
                let articles = string_col(&batch, "article_number")?;
                let chapters = string_col(&batch, "chapter")?;
                let sections = string_col(&batch, "section_title")?;
                let distances = distance_col(&batch)?;

                for i in 0..batch.num_rows() {
                    let distance = distances.get(i).copied().unwrap_or(1.0);
                    let score = 1.0 - distance;
                    out.push(RetrievedChunk {
                        id: ids.get(i).cloned().unwrap_or_default(),
                        text: texts.get(i).cloned().unwrap_or_default(),
                        law_name: laws.get(i).cloned().unwrap_or_default(),
                        article_number: articles.get(i).cloned().unwrap_or_default(),
                        chapter: chapters.get(i).cloned().unwrap_or_default(),
                        section_title: sections.get(i).cloned().unwrap_or_default(),
                        score,
                    });
                }
            }
            Ok(out)
        })
    }
}

fn string_col(
    batch: &arrow_array::RecordBatch,
    name: &str,
) -> Result<Vec<String>, VectorStoreError> {
    use arrow_array::{Array, StringArray};

    let col = batch
        .column_by_name(name)
        .ok_or_else(|| VectorStoreError::Search(format!("missing column {name}")))?;
    let arr = col
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| VectorStoreError::Search(format!("column {name} is not utf8")))?;
    Ok((0..arr.len())
        .map(|i| {
            if arr.is_null(i) {
                String::new()
            } else {
                arr.value(i).to_string()
            }
        })
        .collect())
}

fn distance_col(batch: &arrow_array::RecordBatch) -> Result<Vec<f32>, VectorStoreError> {
    use arrow_array::{Array, Float32Array, Float64Array};

    if let Some(col) = batch.column_by_name("_distance") {
        if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
            return Ok((0..arr.len())
                .map(|i| if arr.is_null(i) { 1.0 } else { arr.value(i) })
                .collect());
        }
        if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
            return Ok((0..arr.len())
                .map(|i| {
                    if arr.is_null(i) {
                        1.0
                    } else {
                        arr.value(i) as f32
                    }
                })
                .collect());
        }
    }
    Ok(vec![1.0; batch.num_rows()])
}
