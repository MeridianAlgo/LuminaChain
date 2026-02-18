use lumina_types::block::{Block, Vote, BlockHeader};
use lumina_types::transaction::Transaction;
use lumina_types::instruction::StablecoinInstruction;
use lumina_types::state::GlobalState;
use lumina_execution::{execute_transaction, ExecutionContext};
use lumina_network::{NetworkCommand, NetworkEvent};
use lumina_storage::db::Storage;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use tracing::{info, error};
use blake3;
use bincode;

pub struct ConsensusService {
    state: Arc<RwLock<GlobalState>>,
    storage: Arc<Storage>,
    network_tx: mpsc::Sender<NetworkCommand>,
    tx_rx: mpsc::Receiver<Transaction>,
    mempool: Vec<Transaction>,
}

impl ConsensusService {
    pub fn new(
        state: Arc<RwLock<GlobalState>>,
        storage: Arc<Storage>,
        network_tx: mpsc::Sender<NetworkCommand>,
        tx_rx: mpsc::Receiver<Transaction>,
    ) -> Self {
        Self {
            state,
            storage,
            network_tx,
            tx_rx,
            mempool: Vec::new(),
        }
    }

    pub async fn run(mut self) {
        info!("Starting Consensus Service (Mocked Malachite Loop)...");
        
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let mut current_height = 0;
        let mut last_block_hash = [0u8; 32];

        // Load highest block from storage for recovery
        if let Ok(Some(last_stored_state)) = self.storage.load_state() {
            // This is simplified. In a real system, we'd iterate blocks to find the highest.
            // For now, we assume state reflects highest committed.
            let highest_block_height = last_stored_state.last_rebalance_height; // Using last_rebalance_height as a proxy
            if highest_block_height > 0 {
                if let Ok(Some(last_block)) = self.storage.load_block_by_height(highest_block_height) {
                    current_height = last_block.header.height;
                    last_block_hash = last_block.hash();
                    info!("Recovered from storage. Starting at height {}", current_height);
                }
            }
        }

        loop {
            tokio::select! {
                Some(tx) = self.tx_rx.recv() => {
                    self.mempool.push(tx);
                }
                _ = interval.tick() => {
                    if self.mempool.is_empty() {
                        continue;
                    }

                    let txs: Vec<Transaction> = self.mempool.drain(..).collect();
                    info!("Consensus: Proposing block {} with {} txs", current_height + 1, txs.len());

                    let mut state_guard = self.state.write().await;
                    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

                    let mut valid_txs = Vec::new();
                    let mut ctx = ExecutionContext {
                        state: &mut state_guard,
                        height: current_height + 1,
                        timestamp,
                    };

                    for tx in txs {
                        match execute_transaction(&tx, &mut ctx) {
                            Ok(_) => valid_txs.push(tx),
                            Err(e) => {
                                error!("Tx execution failed: {}", e);
                            }
                        }
                    }

                    if valid_txs.is_empty() {
                        continue;
                    }

                    // Mock Merkle root calculations
                    let transactions_root = blake3::hash(&bincode::serialize(&valid_txs).unwrap()).into();
                    let state_root = blake3::hash(&bincode::serialize(&*state_guard).unwrap()).into();

                    let new_block = Block {
                        header: BlockHeader {
                            height: current_height + 1,
                            prev_hash: last_block_hash,
                            transactions_root,
                            state_root,
                            timestamp,
                            proposer: [0u8; 32],
                        },
                        transactions: valid_txs.clone(),
                        votes: Vec::new(),
                    };
                    
                    last_block_hash = new_block.hash();
                    current_height += 1;
                    state_guard.last_rebalance_height = current_height; // Update for state recovery

                    // Persist block and state
                    if let Err(e) = self.storage.save_block(&new_block) {
                        error!("Failed to save block: {}", e);
                    }
                    if let Err(e) = self.storage.save_state(&state_guard) {
                        error!("Failed to save state: {}", e);
                    }

                    info!("Consensus: Committed block {} with {} valid txs. Total LUSD Supply: {}", current_height, new_block.transactions.len(), state_guard.total_lusd_supply);
                }
            }
        }
    }
}
