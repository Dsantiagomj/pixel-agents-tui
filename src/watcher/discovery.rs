use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const DORMANCY_TIMEOUT: Duration = Duration::from_secs(300);

/// Scan ~/.claude/projects/ for active .jsonl files (modified within 5 minutes).
/// Returns an empty vec if the directory doesn't exist.
pub fn scan_sessions(claude_dir: &Path) -> Vec<PathBuf> {
    let projects_dir = claude_dir.join("projects");
    if !projects_dir.exists() {
        return Vec::new();
    }

    let now = SystemTime::now();
    let mut sessions = Vec::new();

    walk_for_jsonl(&projects_dir, now, &mut sessions);
    sessions
}

/// Recursively walk a directory, collecting .jsonl files modified within DORMANCY_TIMEOUT.
fn walk_for_jsonl(dir: &Path, now: SystemTime, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_for_jsonl(&path, now, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = now.duration_since(modified) {
                        if elapsed <= DORMANCY_TIMEOUT {
                            out.push(path);
                        }
                    }
                }
            }
        }
    }
}

/// Track known sessions with incremental IDs, detect new/removed sessions.
pub struct SessionTracker {
    known: HashMap<PathBuf, u32>,
    next_id: u32,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            known: HashMap::new(),
            next_id: 1,
        }
    }

    /// Update the tracker with the current list of active session paths.
    ///
    /// Returns a tuple of:
    /// - `new_sessions`: Vec of (id, path) for newly discovered sessions
    /// - `removed_ids`: Vec of IDs for sessions no longer present
    pub fn update(&mut self, current: &[PathBuf]) -> (Vec<(u32, PathBuf)>, Vec<u32>) {
        let current_set: std::collections::HashSet<&PathBuf> = current.iter().collect();

        // Find removed sessions
        let removed: Vec<u32> = self
            .known
            .iter()
            .filter(|(path, _)| !current_set.contains(path))
            .map(|(_, &id)| id)
            .collect();

        // Remove them from known
        self.known.retain(|path, _| current_set.contains(path));

        // Find new sessions and assign IDs
        let mut new_sessions = Vec::new();
        for path in current {
            if !self.known.contains_key(path) {
                let id = self.next_id;
                self.next_id += 1;
                self.known.insert(path.clone(), id);
                new_sessions.push((id, path.clone()));
            }
        }

        (new_sessions, removed)
    }

    /// Look up the ID for a given session path.
    pub fn get_id(&self, path: &Path) -> Option<u32> {
        self.known.get(path).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_assigns_incremental_ids() {
        let mut tracker = SessionTracker::new();
        let paths = vec![PathBuf::from("/tmp/a.jsonl"), PathBuf::from("/tmp/b.jsonl")];
        let (new, removed) = tracker.update(&paths);
        assert_eq!(new.len(), 2);
        assert_eq!(new[0].0, 1);
        assert_eq!(new[1].0, 2);
        assert!(removed.is_empty());
    }

    #[test]
    fn tracker_detects_removed_sessions() {
        let mut tracker = SessionTracker::new();
        let paths = vec![PathBuf::from("/tmp/a.jsonl"), PathBuf::from("/tmp/b.jsonl")];
        tracker.update(&paths);
        let (new, removed) = tracker.update(&[PathBuf::from("/tmp/a.jsonl")]);
        assert!(new.is_empty());
        assert_eq!(removed, vec![2]);
    }

    #[test]
    fn tracker_detects_new_sessions() {
        let mut tracker = SessionTracker::new();
        tracker.update(&[PathBuf::from("/tmp/a.jsonl")]);
        let (new, _) = tracker.update(&[
            PathBuf::from("/tmp/a.jsonl"),
            PathBuf::from("/tmp/c.jsonl"),
        ]);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].0, 2);
    }

    #[test]
    fn tracker_get_id() {
        let mut tracker = SessionTracker::new();
        tracker.update(&[PathBuf::from("/tmp/a.jsonl")]);
        assert_eq!(tracker.get_id(&PathBuf::from("/tmp/a.jsonl")), Some(1));
        assert_eq!(tracker.get_id(&PathBuf::from("/tmp/nope.jsonl")), None);
    }

    #[test]
    fn scan_sessions_handles_missing_dir() {
        let sessions = scan_sessions(Path::new("/nonexistent/path"));
        assert!(sessions.is_empty());
    }
}
