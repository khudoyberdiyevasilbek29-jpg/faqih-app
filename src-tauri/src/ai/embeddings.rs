//! ONNX Runtime embeddings via `ort` + real `tokenizers`.
//!
//! Known-bug preventions applied:
//! 1. THREE ONNX inputs: input_ids, attention_mask, token_type_ids (all-zeros).
//! 2. Real tokenizer.json via `tokenizers` crate — never a custom tokenizer.
//! 3. e5 prefixes: "passage: " / "query: ".

use std::path::Path;
use std::sync::Mutex;

use ndarray::{Array2, Array3};
use ort::session::Session;
use ort::value::Tensor;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};
use tracing::info;

const DEFAULT_MAX_LEN: usize = 256;
const EMBEDDING_DIM: usize = 384;
#[allow(dead_code)]
const PASSAGE_PREFIX: &str = "passage: ";
const QUERY_PREFIX: &str = "query: ";

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Embedding model not loaded: {0}")]
    NotReady(String),
    #[error("Tokenizer error: {0}")]
    Tokenizer(String),
    #[error("ONNX inference error: {0}")]
    Inference(String),
}

pub struct EmbeddingEngine {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
    max_length: usize,
}

impl EmbeddingEngine {
    pub fn load(
        onnx_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        max_length: usize,
    ) -> Result<Self, EmbeddingError> {
        let onnx_path = onnx_path.as_ref();
        let tokenizer_path = tokenizer_path.as_ref();
        if !onnx_path.exists() {
            return Err(EmbeddingError::NotReady(format!(
                "missing ONNX model at {}",
                onnx_path.display()
            )));
        }
        if !tokenizer_path.exists() {
            return Err(EmbeddingError::NotReady(format!(
                "missing tokenizer.json at {}",
                tokenizer_path.display()
            )));
        }

        let mut tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| EmbeddingError::Tokenizer(e.to_string()))?;

        let max_length = max_length.max(8);
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length,
                strategy: TruncationStrategy::LongestFirst,
                stride: 0,
                direction: tokenizers::TruncationDirection::Right,
            }))
            .map_err(|e| EmbeddingError::Tokenizer(e.to_string()))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::Fixed(max_length),
            direction: tokenizers::PaddingDirection::Right,
            pad_to_multiple_of: None,
            pad_id: 0,
            pad_type_id: 0,
            pad_token: "[PAD]".into(),
        }));

        let session = Session::builder()
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?
            .commit_from_file(onnx_path)
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        info!(
            onnx = %onnx_path.display(),
            tokenizer = %tokenizer_path.display(),
            max_length,
            "Loaded ONNX embedding model + real tokenizer.json"
        );

        Ok(Self {
            tokenizer,
            session: Mutex::new(session),
            max_length,
        })
    }

    pub fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.embed_with_prefix(QUERY_PREFIX, text)
    }

    #[allow(dead_code)]
    pub fn embed_passage(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.embed_with_prefix(PASSAGE_PREFIX, text)
    }

    fn embed_with_prefix(&self, prefix: &str, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let input = format!("{prefix}{text}");
        let encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|e| EmbeddingError::Tokenizer(e.to_string()))?;

        let mut input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| i64::from(id)).collect();
        let mut attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| i64::from(m))
            .collect();

        // Ensure fixed length (tokenizer padding should already do this).
        input_ids.resize(self.max_length, 0);
        attention_mask.resize(self.max_length, 0);
        // CRITICAL: token_type_ids must be present (all-zeros for single-sequence e5).
        let token_type_ids = vec![0i64; self.max_length];

        let shape = vec![1usize, self.max_length];
        let ids_tensor = Tensor::from_array((shape.clone(), input_ids))
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
        let mask_tensor = Tensor::from_array((shape.clone(), attention_mask.clone()))
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
        let type_tensor = Tensor::from_array((shape, token_type_ids))
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| EmbeddingError::Inference("session lock poisoned".into()))?;

        let outputs = session
            .run(ort::inputs![
                "input_ids" => ids_tensor,
                "attention_mask" => mask_tensor,
                "token_type_ids" => type_tensor,
            ])
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        let hidden = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        // Shape: [1, seq, 384] — ort returns i64 dims
        let (shape, data) = hidden;
        if shape.len() != 3 || shape[2] as usize != EMBEDDING_DIM {
            return Err(EmbeddingError::Inference(format!(
                "unexpected last_hidden_state shape: {shape:?}"
            )));
        }
        let rows = shape[0] as usize;
        let seq = shape[1] as usize;
        let dim = shape[2] as usize;
        let arr = Array3::from_shape_vec((rows, seq, dim), data.to_vec())
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        let mask = Array2::from_shape_vec((1, self.max_length), attention_mask)
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;
        let pooled = mean_pool(&arr, &mask);
        Ok(l2_normalize(&pooled))
    }
}

fn mean_pool(hidden: &Array3<f32>, attention_mask: &Array2<i64>) -> Vec<f32> {
    let seq = hidden.shape()[1];
    let dim = hidden.shape()[2];
    let mut out = vec![0.0f32; dim];
    let mut count = 0.0f32;
    for t in 0..seq {
        if attention_mask[[0, t]] == 0 {
            continue;
        }
        count += 1.0;
        for d in 0..dim {
            out[d] += hidden[[0, t, d]];
        }
    }
    if count > 0.0 {
        for v in &mut out {
            *v /= count;
        }
    }
    out
}

fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    v.iter().map(|x| x / norm).collect()
}

#[allow(dead_code)]
pub fn embedding_dim() -> usize {
    EMBEDDING_DIM
}

#[allow(dead_code)]
pub fn default_max_len() -> usize {
    DEFAULT_MAX_LEN
}
