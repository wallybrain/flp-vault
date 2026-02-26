use crate::parser;
use crate::store::files::{hash_in_cache, is_cached, update_path_index, upsert_file};
use rusqlite::Connection;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_64;

#[derive(Debug, Serialize, Clone)]
struct ScanStarted {
    total: usize,
}

#[derive(Debug, Serialize, Clone)]
struct ScanProgress {
    done: usize,
    total: usize,
    path: String,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct ScanComplete {
    total: usize,
}

#[derive(Debug, Serialize, Clone)]
struct ScanCancelled {
    done: usize,
}

pub fn run_scan(
    path: &str,
    db: Arc<Mutex<Connection>>,
    app: AppHandle,
    scan_running: Arc<Mutex<bool>>,
) {
    let flp_files: Vec<walkdir::DirEntry> = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("flp"))
                    .unwrap_or(false)
        })
        .collect();

    let total = flp_files.len();

    let _ = app.emit("scan:started", ScanStarted { total });

    let mut done = 0usize;

    for entry in &flp_files {
        {
            let running = scan_running.lock().unwrap();
            if !*running {
                drop(running);
                let _ = app.emit("scan:cancelled", ScanCancelled { done });
                return;
            }
        }

        let file_path = entry.path().to_string_lossy().to_string();

        let meta_result = entry.metadata();
        let (file_size, mtime) = match meta_result {
            Ok(m) => {
                let size = m.len() as i64;
                let mt = m
                    .modified()
                    .ok()
                    .and_then(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).ok()
                    })
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                (size, mt)
            }
            Err(_) => {
                done += 1;
                let _ = app.emit(
                    "scan:progress",
                    ScanProgress {
                        done,
                        total,
                        path: file_path,
                        warnings: vec!["Failed to read file metadata".to_string()],
                    },
                );
                continue;
            }
        };

        if is_cached(&db, &file_path, file_size, mtime) {
            done += 1;
            let _ = app.emit(
                "scan:progress",
                ScanProgress {
                    done,
                    total,
                    path: file_path,
                    warnings: vec![],
                },
            );
            continue;
        }

        let bytes = match std::fs::read(entry.path()) {
            Ok(b) => b,
            Err(e) => {
                done += 1;
                let _ = app.emit(
                    "scan:progress",
                    ScanProgress {
                        done,
                        total,
                        path: file_path,
                        warnings: vec![format!("Failed to read file: {}", e)],
                    },
                );
                continue;
            }
        };

        let hash = format!("{:016x}", xxh3_64(&bytes));

        if hash_in_cache(&db, &hash) {
            update_path_index(&db, &file_path, &hash, file_size, mtime);
            done += 1;
            let _ = app.emit(
                "scan:progress",
                ScanProgress {
                    done,
                    total,
                    path: file_path,
                    warnings: vec![],
                },
            );
            continue;
        }

        let warnings = match parser::parse_flp(&bytes) {
            Ok(meta) => {
                let w = meta.warnings.clone();
                upsert_file(&db, &hash, &file_path, file_size, mtime, &meta);
                w
            }
            Err(e) => {
                let warning = format!("Parse error: {:?}", e);
                let empty_meta = crate::parser::types::FlpMetadata {
                    warnings: vec![warning.clone()],
                    ..Default::default()
                };
                upsert_file(&db, &hash, &file_path, file_size, mtime, &empty_meta);
                vec![warning]
            }
        };

        done += 1;
        let _ = app.emit(
            "scan:progress",
            ScanProgress {
                done,
                total,
                path: file_path,
                warnings,
            },
        );
    }

    {
        let mut running = scan_running.lock().unwrap();
        *running = false;
    }

    let _ = app.emit("scan:complete", ScanComplete { total });
}

