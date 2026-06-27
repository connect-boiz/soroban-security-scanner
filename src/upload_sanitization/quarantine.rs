//! Quarantine store for suspicious uploads awaiting manual review.
//!
//! When a file is flagged as suspicious (but not outright malicious) it is
//! quarantined rather than accepted or destroyed: the bytes are held, indexed
//! by a content hash, and an admin can later release or purge them. Confirmed
//! malicious files are also quarantined for forensic review.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// Disposition of a quarantined item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuarantineStatus {
    /// Awaiting an admin decision.
    PendingReview,
    /// Cleared by a reviewer.
    Released,
    /// Confirmed bad and purged.
    Purged,
}

/// A quarantined file record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineEntry {
    /// Unique id.
    pub id: Uuid,
    /// SHA-256 of the content (also a dedupe key).
    pub sha256: String,
    /// Sanitized original filename.
    pub filename: String,
    /// Who uploaded it.
    pub uploader: Uuid,
    /// Why it was quarantined.
    pub reason: String,
    /// When it was quarantined (unix secs).
    pub created_at: i64,
    /// Current status.
    pub status: QuarantineStatus,
    /// The held bytes (omitted once purged).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
}

/// In-memory quarantine store.
#[derive(Default)]
pub struct Quarantine {
    entries: HashMap<Uuid, QuarantineEntry>,
}

impl Quarantine {
    /// Creates an empty quarantine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Quarantines a file, returning its new entry id.
    pub fn add(
        &mut self,
        bytes: &[u8],
        filename: impl Into<String>,
        uploader: Uuid,
        reason: impl Into<String>,
        now: i64,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let entry = QuarantineEntry {
            id,
            sha256: sha256_hex(bytes),
            filename: filename.into(),
            uploader,
            reason: reason.into(),
            created_at: now,
            status: QuarantineStatus::PendingReview,
            bytes: Some(bytes.to_vec()),
        };
        self.entries.insert(id, entry);
        id
    }

    /// Fetches an entry.
    pub fn get(&self, id: Uuid) -> Option<&QuarantineEntry> {
        self.entries.get(&id)
    }

    /// All entries awaiting review.
    pub fn pending(&self) -> Vec<&QuarantineEntry> {
        self.entries
            .values()
            .filter(|e| e.status == QuarantineStatus::PendingReview)
            .collect()
    }

    /// Releases an entry as a false positive; returns the held bytes.
    pub fn release(&mut self, id: Uuid) -> Option<Vec<u8>> {
        let entry = self.entries.get_mut(&id)?;
        if entry.status != QuarantineStatus::PendingReview {
            return None;
        }
        entry.status = QuarantineStatus::Released;
        entry.bytes.clone()
    }

    /// Purges an entry's bytes after confirming it is malicious.
    pub fn purge(&mut self, id: Uuid) -> bool {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.status = QuarantineStatus::Purged;
            entry.bytes = None;
            true
        } else {
            false
        }
    }

    /// Number of entries currently pending review.
    pub fn pending_count(&self) -> usize {
        self.pending().len()
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_fetch() {
        let mut q = Quarantine::new();
        let uploader = Uuid::new_v4();
        let id = q.add(b"suspicious", "x.rs", uploader, "overlong line", 1000);
        let entry = q.get(id).unwrap();
        assert_eq!(entry.status, QuarantineStatus::PendingReview);
        assert_eq!(entry.uploader, uploader);
        assert!(entry.bytes.is_some());
        assert_eq!(q.pending_count(), 1);
    }

    #[test]
    fn release_returns_bytes_and_clears_pending() {
        let mut q = Quarantine::new();
        let id = q.add(b"data", "x.rs", Uuid::new_v4(), "review", 1000);
        let released = q.release(id).unwrap();
        assert_eq!(released, b"data");
        assert_eq!(q.get(id).unwrap().status, QuarantineStatus::Released);
        assert_eq!(q.pending_count(), 0);
        // Cannot release twice.
        assert!(q.release(id).is_none());
    }

    #[test]
    fn purge_drops_bytes() {
        let mut q = Quarantine::new();
        let id = q.add(b"malware", "x.rs", Uuid::new_v4(), "eicar", 1000);
        assert!(q.purge(id));
        let entry = q.get(id).unwrap();
        assert_eq!(entry.status, QuarantineStatus::Purged);
        assert!(entry.bytes.is_none());
    }

    #[test]
    fn hash_is_recorded() {
        let mut q = Quarantine::new();
        let id = q.add(b"abc", "x.rs", Uuid::new_v4(), "r", 1000);
        // SHA-256("abc")
        assert_eq!(
            q.get(id).unwrap().sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn purged_entry_omits_bytes_in_json() {
        let mut q = Quarantine::new();
        let id = q.add(b"malware", "x.rs", Uuid::new_v4(), "eicar", 1000);
        q.purge(id);
        let json = serde_json::to_string(q.get(id).unwrap()).unwrap();
        assert!(!json.contains("bytes"));
    }
}
