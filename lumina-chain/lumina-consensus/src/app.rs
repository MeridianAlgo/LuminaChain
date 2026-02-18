#![cfg(feature = "malachite")]

use async_trait::async_trait;
use lumina_execution::{execute_transaction, ExecutionContext};
use lumina_storage::db::Storage;
use lumina_types::state::GlobalState;
use lumina_types::transaction::Transaction;
use bincode;

pub struct LuminaApp {
    pub state: GlobalState,
    pub storage: Storage,
    pub height: u64,
}

impl LuminaApp {
    pub fn new(storage: Storage) -> Self {
        let state = storage.load_state().unwrap_or_default();
        Self {
            state,
            storage,
            height: 0,
        }
    }
}

#[async_trait]
pub trait MalachiteApp {
    async fn check_tx(&self, tx: &[u8]) -> bool;
    async fn apply_block(&mut self, transactions: Vec<Vec<u8>>) -> [u8; 32];
    async fn commit(&mut self) -> Result<(), String>;
}

#[async_trait]
impl MalachiteApp for LuminaApp {
    async fn check_tx(&self, tx_bytes: &[u8]) -> bool {
        let tx: Result<Transaction, _> = bincode::deserialize(tx_bytes);
        match tx {
            Ok(_t) => true,
            Err(_) => false,
        }
    }

    async fn apply_block(&mut self, transactions: Vec<Vec<u8>>) -> [u8; 32] {
        self.height += 1;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut ctx = ExecutionContext {
            state: &mut self.state,
            height: self.height,
            timestamp,
        };

        for tx_bytes in transactions {
            if let Ok(tx) = bincode::deserialize::<Transaction>(&tx_bytes) {
                let _ = execute_transaction(&tx, &mut ctx);
            }
        }

        self.state.root_hash()
    }

    async fn commit(&mut self) -> Result<(), String> {
        self.storage.save_state(&self.state).map_err(|e| e.to_string())
    }
}
