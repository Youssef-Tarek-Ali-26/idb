use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use idb_core::{CoreError, CoreResult, RecordEnvelope, RecordId, TenantId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mutation")]
pub enum WalMutation {
    Upsert {
        record: RecordEnvelope,
    },
    Delete {
        tenant_id: TenantId,
        record_id: RecordId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalEntry {
    pub sequence: u64,
    pub committed_at: DateTime<Utc>,
    pub mutation: WalMutation,
}

#[derive(Debug)]
pub struct Wal {
    path: PathBuf,
}

impl Wal {
    pub fn new(path: impl AsRef<Path>) -> CoreResult<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            File::create(&path_buf)
                .map_err(|e| CoreError::Storage(format!("failed to create wal file: {e}")))?;
        }
        Ok(Self { path: path_buf })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, entry: &WalEntry) -> CoreResult<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|e| CoreError::Storage(format!("failed to open wal for append: {e}")))?;

        let json = serde_json::to_string(entry)
            .map_err(|e| CoreError::Serialization(format!("failed to serialize wal entry: {e}")))?;
        file.write_all(json.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.flush())
            .map_err(|e| CoreError::Storage(format!("failed to append wal entry: {e}")))?;
        Ok(())
    }

    pub fn read_all(&self) -> CoreResult<Vec<WalEntry>> {
        let file = File::open(&self.path)
            .map_err(|e| CoreError::Storage(format!("failed to open wal for read: {e}")))?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (line_no, line_result) in reader.lines().enumerate() {
            let line = line_result.map_err(|e| {
                CoreError::Storage(format!("failed reading wal line {}: {e}", line_no + 1))
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: WalEntry = serde_json::from_str(&line).map_err(|e| {
                CoreError::Serialization(format!("failed parsing wal line {}: {e}", line_no + 1))
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }
}
