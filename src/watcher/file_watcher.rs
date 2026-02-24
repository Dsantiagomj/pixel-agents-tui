use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::watcher::parser;
use crate::watcher::types::JsonlRecord;

pub struct IncrementalReader {
    offsets: HashMap<PathBuf, u64>,
}

impl Default for IncrementalReader {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalReader {
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
        }
    }

    pub fn read_new_lines(&mut self, path: &Path) -> Vec<JsonlRecord> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let file_len = match file.metadata() {
            Ok(m) => m.len(),
            Err(_) => return Vec::new(),
        };

        let canonical = path.to_path_buf();
        let stored_offset = self.offsets.get(&canonical).copied().unwrap_or(0);

        // If file is smaller than stored offset, it was truncated/rotated — reset to 0
        let offset = if file_len < stored_offset {
            0
        } else {
            stored_offset
        };

        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(offset)).is_err() {
            return Vec::new();
        }

        let mut records = Vec::new();
        let mut current_offset = offset;
        let mut line_buf = String::new();

        loop {
            line_buf.clear();
            match reader.read_line(&mut line_buf) {
                Ok(0) => break, // EOF
                Ok(bytes_read) => {
                    current_offset += bytes_read as u64;
                    if let Some(record) = parser::parse_line(&line_buf) {
                        records.push(record);
                    }
                }
                Err(_) => break,
            }
        }

        self.offsets.insert(canonical, current_offset);
        records
    }

    pub fn remove(&mut self, path: &Path) {
        self.offsets.remove(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_new_lines_incrementally() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"hello"}}]}}}}"#).unwrap();
        }
        let mut reader = IncrementalReader::new();
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 1);
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 0); // No new content
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&file_path)
                .unwrap();
            writeln!(
                f,
                r#"{{"type":"system","subtype":"turn_duration","duration_ms":100}}"#
            )
            .unwrap();
        }
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0], JsonlRecord::System { .. }));
    }

    #[test]
    fn handles_missing_file() {
        let mut reader = IncrementalReader::new();
        let records = reader.read_new_lines(Path::new("/nonexistent/file.jsonl"));
        assert!(records.is_empty());
    }

    #[test]
    fn handles_file_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"first"}}]}}}}"#).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"second"}}]}}}}"#).unwrap();
        }
        let mut reader = IncrementalReader::new();
        reader.read_new_lines(&file_path); // Read all
        {
            let mut f = File::create(&file_path).unwrap(); // Truncate
            writeln!(
                f,
                r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"new"}}]}}}}"#
            )
            .unwrap();
        }
        let records = reader.read_new_lines(&file_path);
        assert_eq!(records.len(), 1); // Re-reads from beginning
    }

    #[test]
    fn remove_clears_tracking() {
        let mut reader = IncrementalReader::new();
        let path = PathBuf::from("/tmp/test.jsonl");
        reader.offsets.insert(path.clone(), 100);
        reader.remove(&path);
        assert!(!reader.offsets.contains_key(&path));
    }
}
