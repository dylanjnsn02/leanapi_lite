use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::Request;

/// The persisted (and in-memory) shape of an HTTP response. Body is stored
/// raw; pretty-printing/highlighting is a view-time transform.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseSnapshot {
    pub status_code: i32,
    pub status: String,
    pub headers: BTreeMap<String, Vec<String>>,
    pub body: String,
    pub size: i64,
}

/// One logged request/response pair. `response` is None when the request
/// errored before a response was received.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub request: Request,
    pub response: Option<ResponseSnapshot>,
    pub error: String,
    pub duration_ms: i64,
}

/// A sortable, unique-enough-for-single-process id.
pub fn new_id() -> String {
    Utc::now().format("%Y%m%dT%H%M%S%.9fZ").to_string()
}

/// Path to the history JSONL file, creating its parent directory if needed.
/// Deliberately a different path from the Go leanapi tool's history file.
pub fn default_path() -> Result<PathBuf> {
    let mut dir = dirs::config_dir().context("could not determine config directory")?;
    dir.push("leanapi-lite");
    fs::create_dir_all(&dir)?;
    Ok(dir.join("history.jsonl"))
}

/// Writes one entry as a single JSON line to the history file.
pub fn append(entry: &Entry) -> Result<()> {
    let path = default_path()?;
    let mut f = OpenOptions::new().append(true).create(true).open(path)?;
    let line = serde_json::to_string(entry)?;
    writeln!(f, "{line}")?;
    Ok(())
}

/// Reads every entry from the history file, oldest first.
pub fn load_all() -> Result<Vec<Entry>> {
    let path = default_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let f = fs::File::open(path)?;
    let reader = BufReader::new(f);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(e) = serde_json::from_str::<Entry>(&line) {
            entries.push(e);
        }
        // skip malformed lines rather than fail the whole browser
    }
    Ok(entries)
}
