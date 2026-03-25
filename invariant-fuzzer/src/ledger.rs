use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MockLedger {
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
    pub sequence_number: u32,
    pub timestamp: u64,
    pub network_id: [u8; 32],
    pub base_reserve: u32,
    pub min_temp_entry_ttl: u32,
    pub min_persistent_entry_ttl: u32,
    pub max_entry_ttl: u32,
}

impl MockLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_storage(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.storage.get(key)
    }

    pub fn put_storage(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.storage.insert(key, value);
    }

    pub fn delete_storage(&mut self, key: &[u8]) {
        self.storage.remove(key);
    }
}

pub struct ExecutionContext {
    pub ledger: MockLedger,
    pub contract_id: [u8; 32],
    pub depth: u32,
}

impl ExecutionContext {
    pub fn new(contract_id: [u8; 32]) -> Self {
        Self {
            ledger: MockLedger::new(),
            contract_id,
            depth: 0,
        }
    }
}
