use std::collections::HashMap;
use std::time::Instant;
use std::sync::Arc;
use byteorder::{ByteOrder, LittleEndian};
use rayon::prelude::*;
use rusqlite::Connection;
use crate::ai::indexing_service::SearchResult;
use ndarray::prelude::*;
use dashmap::DashMap;
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;

pub struct CachedBoard {
    pub paths: Vec<String>,
    pub matrix: Vec<f32>,
    pub dim: usize,
    pub loaded_at: Instant,
}

pub struct EmbeddingCache {
    /// Sharded map from AI DB path -> CachedBoard for high-concurrency access
    cache: Arc<DashMap<Utf8PathBuf, CachedBoard>>,
}

impl Clone for EmbeddingCache {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }
}

impl Default for EmbeddingCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, db_path: &Utf8Path) -> Option<CachedBoardRef> {
        self.cache.get(db_path).map(|b| CachedBoardRef {
            paths: b.paths.clone(),
            matrix: b.matrix.clone(),
            dim: b.dim,
        })
    }

    pub fn set(&self, db_path: Utf8PathBuf, paths: Vec<String>, matrix: Vec<f32>, dim: usize) {
        self.cache.insert(db_path, CachedBoard {
            paths,
            matrix,
            dim,
            loaded_at: Instant::now(),
        });
    }

    pub fn invalidate(&self, db_path: &Utf8Path) {
        self.cache.remove(db_path);
    }

    pub fn clear(&self) {
        self.cache.clear();
    }
}

/// A cloned reference to cached board data for processing
pub struct CachedBoardRef {
    pub paths: Vec<String>,
    pub matrix: Vec<f32>,
    pub dim: usize,
}

pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let a_arr = ArrayView1::from(a);
    let b_arr = ArrayView1::from(b);
    a_arr.dot(&b_arr)
}

pub fn search_single_db(
    db_path: &Utf8Path,
    query_vec: &[f32],
    top_k: usize,
    cache: &EmbeddingCache,
) -> Vec<SearchResult> {
    let cached = if let Some(c) = cache.get(db_path) {
        c
    } else {
        match load_embeddings_from_db(db_path) {
            Ok((paths, matrix, dim)) => {
                cache.set(db_path.to_path_buf(), paths.clone(), matrix.clone(), dim);
                CachedBoardRef { paths, matrix, dim }
            }
            Err(e) => {
                log::error!("Failed to load embeddings from {:?}: {}", db_path, e);
                return Vec::new();
            }
        }
    };

    if cached.paths.is_empty() {
        return Vec::new();
    }

    search_in_memory(&cached.paths, &cached.matrix, cached.dim, query_vec, top_k)
}

fn load_embeddings_from_db(db_path: &Utf8Path) -> Result<(Vec<String>, Vec<f32>, usize), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT filepath, embedding FROM embeddings WHERE embedding IS NOT NULL")
        .map_err(|e| e.to_string())?;

    let mut paths = Vec::new();
    let mut matrix = Vec::new();
    let mut dim = 0;

    let rows = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((path, blob))
        })
        .map_err(|e| e.to_string())?;

    for (path, blob) in rows.flatten() {
        if blob.len() % 4 != 0 {
            continue;
        }
        let row_dim = blob.len() / 4;
        if dim == 0 {
            dim = row_dim;
        } else if dim != row_dim {
            continue;
        }

        let mut row_vec = vec![0.0f32; row_dim];
        LittleEndian::read_f32_into(&blob, &mut row_vec);
        
        paths.push(path);
        matrix.extend(row_vec);
    }

    Ok((paths, matrix, dim))
}

fn search_in_memory(
    paths: &[String],
    matrix: &[f32],
    dim: usize,
    query_vec: &[f32],
    top_k: usize,
) -> Vec<SearchResult> {
    if dim == 0 || query_vec.len() != dim {
        return Vec::new();
    }

    let query_arr = ArrayView1::from(query_vec);
    let matrix_arr = ArrayView2::from_shape((paths.len(), dim), matrix).unwrap();
    let scores = matrix_arr.dot(&query_arr);

    let mut results: Vec<SearchResult> = paths
        .iter()
        .zip(scores.iter())
        .map(|(path, &score)| {
            SearchResult {
                path: path.clone(),
                score: score as f64,
                ftype: None,
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);
    results
}

pub fn search_all_boards(
    db_paths: Vec<Utf8PathBuf>,
    query_vec: &[f32],
    top_k: usize,
    cache: &EmbeddingCache,
) -> Vec<SearchResult> {
    let boards: Vec<CachedBoardRef> = db_paths
        .into_iter()
        .filter_map(|path| {
            if let Some(cached) = cache.get(&path) {
                Some(cached)
            } else {
                match load_embeddings_from_db(&path) {
                    Ok((p, m, d)) => {
                        cache.set(path.clone(), p.clone(), m.clone(), d);
                        Some(CachedBoardRef { paths: p, matrix: m, dim: d })
                    }
                    Err(_) => None,
                }
            }
        })
        .collect();

    let query_arr = ArrayView1::from(query_vec);

    let merged: Vec<SearchResult> = boards
        .par_iter()
        .flat_map(|board| {
            if board.dim == 0 || query_arr.len() != board.dim {
                return Vec::new();
            }

            let matrix_arr = ArrayView2::from_shape((board.paths.len(), board.dim), &board.matrix).unwrap();
            let scores = matrix_arr.dot(&query_arr);

            board.paths.iter().zip(scores.iter()).map(move |(path, &score)| {
                SearchResult {
                    path: path.clone(),
                    score: score as f64,
                    ftype: None,
                }
            }).collect::<Vec<_>>()
        })
        .collect();

    merged.into_iter()
        .sorted_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
        .take(top_k)
        .collect()
}

pub fn rescore(
    db_path: &Utf8Path,
    query_vec: &[f32],
    target_paths: &[String],
    cache: &EmbeddingCache,
) -> Vec<SearchResult> {
    if let Some(cached) = cache.get(db_path) {
        return rescore_in_memory(&cached.paths, &cached.matrix, cached.dim, query_vec, target_paths);
    }

    let (paths, matrix, dim) = match load_embeddings_from_db(db_path) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to load embeddings for rescore from {:?}: {}", db_path, e);
            return Vec::new();
        }
    };

    if paths.is_empty() {
        return Vec::new();
    }

    cache.set(db_path.to_path_buf(), paths.clone(), matrix.clone(), dim);
    rescore_in_memory(&paths, &matrix, dim, query_vec, target_paths)
}

fn rescore_in_memory(
    all_paths: &[String],
    matrix: &[f32],
    dim: usize,
    query_vec: &[f32],
    target_paths: &[String],
) -> Vec<SearchResult> {
    if dim == 0 || query_vec.len() != dim {
        return Vec::new();
    }

    let path_to_idx: HashMap<_, _> = all_paths.iter().enumerate().map(|(i, p)| (p, i)).collect();
    let mut results: Vec<SearchResult> = target_paths.par_iter().filter_map(|path| {
        if let Some(&idx) = path_to_idx.get(path) {
            let start = idx * dim;
            let end = start + dim;
            let row = &matrix[start..end];
            let score = dot_product(row, query_vec);
            Some(SearchResult {
                path: path.clone(),
                score: score as f64,
                ftype: None,
            })
        } else {
            None
        }
    }).collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}
