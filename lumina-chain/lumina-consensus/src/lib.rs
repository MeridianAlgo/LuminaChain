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
use bincode;
use anyhow::{Result, bail, Context};
use std::collections::HashSet;

pub struct ConsensusService {
    state: Arc<RwLock<GlobalState>>,
    storage: Arc<Storage>,
    network_tx: mpsc::Sender<NetworkCommand>,
    tx_rx: mpsc::Receiver<Transaction>,
    block_rx: mpsc::Receiver<Block>,
    mempool: Vec<Transaction>,
    seen_blocks: HashSet<[u8; 32]>,
}

impl ConsensusService {
    pub fn new(
        state: Arc<RwLock<GlobalState>>,
        storage: Arc<Storage>,
        network_tx: mpsc::Sender<NetworkCommand>,
        tx_rx: mpsc::Receiver<Transaction>,
        block_rx: mpsc::Receiver<Block>,
    ) -> Self {
        Self {
            state,
            storage,
            network_tx,
            tx_rx,
            block_rx,
            mempool: Vec::new(),
            seen_blocks: HashSet::new(),
        }
    }

    pub async fn run(mut self) {
        info!("Starting Consensus Service (Mocked Malachite Loop)...");
        
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let mut current_height = 0;
        let mut last_block_hash = [0u8; 32];

        // Load canonical chain tip from storage for recovery
        if let Ok(Some((h, hash))) = self.storage.load_tip() {
            current_height = h;
            last_block_hash = hash;
            info!("Recovered chain tip. Starting at height {}", current_height);
        }

        // Ensure we have a state snapshot for the recovered tip height.
        // This is required for deterministic block validation/import.
        {
            let state_guard = self.state.read().await;
            if self
                .storage
                .load_state_by_height(current_height)
                .ok()
                .flatten()
                .is_none()
            {
                if let Err(e) = self.storage.save_state_at_height(current_height, &state_guard) {
                    error!("Failed to save state snapshot at height {}: {}", current_height, e);
                }
            }
        }

        loop {
            tokio::select! {
                Some(tx) = self.tx_rx.recv() => {
                    self.mempool.push(tx);
                }
                Some(block) = self.block_rx.recv() => {
                    let bh = block.hash();
                    if self.seen_blocks.insert(bh) {
                        match self.import_block_and_maybe_reorg(&block).await {
                            Ok(did_reorg) => {
                                if did_reorg {
                                    if let Ok(Some((h, hash))) = self.storage.load_tip() {
                                        current_height = h;
                                        last_block_hash = hash;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Network block import failed: {}", e);
                            }
                        }
                    }
                }
                _ = interval.tick() => {
                    if self.mempool.is_empty() {
                        continue;
                    }

                    let txs: Vec<Transaction> = self.mempool.drain(..).collect();
                    info!("Consensus: Proposing block {} with {} txs", current_height + 1, txs.len());

                    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let height = current_height + 1;

                    let parent_state = match self
                        .storage
                        .load_state_by_height(current_height)
                        .context("load_state_by_height")
                    {
                        Ok(Some(s)) => s,
                        Ok(None) => {
                            error!("Missing parent snapshot at height {}", current_height);
                            continue;
                        }
                        Err(e) => {
                            error!("Failed to load parent snapshot at height {}: {}", current_height, e);
                            continue;
                        }
                    };

                    let proposed = match build_block_from_parent(parent_state, txs, height, last_block_hash, timestamp) {
                        Ok(b) => b,
                        Err(e) => {
                            error!("Failed to build block {}: {}", height, e);
                            continue;
                        }
                    };

                    match self.import_block_and_maybe_reorg(&proposed).await {
                        Ok(did_reorg) => {
                            if did_reorg {
                                last_block_hash = proposed.hash();
                                current_height = proposed.header.height;
                                let state_guard = self.state.read().await;
                                info!(
                                    "Consensus: Committed block {} with {} txs. Total LUSD Supply: {}",
                                    current_height,
                                    proposed.transactions.len(),
                                    state_guard.total_lusd_supply
                                );

                                // Broadcast canonical block
                                if let Ok(bytes) = bincode::serialize(&proposed) {
                                    let _ = self.network_tx.send(NetworkCommand::BroadcastBlock(bytes)).await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Block import failed at height {}: {}", height, e);
                        }
                    }
                }
            }
        }
    }

    async fn import_block_and_maybe_reorg(&self, block: &Block) -> Result<bool> {
        let block_hash = block.hash();

        // Fast-path: already imported
        if self.storage.load_block_meta(&block_hash)?.is_some() {
            return Ok(false);
        }

        if block.header.height == 0 {
            bail!("Invalid block height 0");
        }

        // Ensure parent is known (or genesis)
        let parent_hash = block.header.prev_hash;
        if block.header.height > 1 {
            if self.storage.load_block_meta(&parent_hash)?.is_none() {
                bail!("Unknown parent block");
            }
        }

        // Verify tx root
        let expected_tx_root = Block::transactions_root(&block.transactions);
        if block.header.transactions_root != expected_tx_root {
            bail!("Invalid transactions_root");
        }

        // Load parent state by hash
        let parent_state = if block.header.height == 1 {
            self.storage.load_state_by_height(0)?.unwrap_or_default()
        } else {
            self.storage
                .load_state_by_hash(&parent_hash)?
                .ok_or_else(|| anyhow::anyhow!("Missing parent state (by hash)"))?
        };

        // Execute txs to compute expected state root
        let mut next_state = parent_state.clone();
        {
            let mut ctx = ExecutionContext {
                state: &mut next_state,
                height: block.header.height,
                timestamp: block.header.timestamp,
            };
            for tx in &block.transactions {
                execute_transaction(tx, &mut ctx)?;
            }
        }

        let expected_state_root = next_state.root_hash();
        if block.header.state_root != expected_state_root {
            bail!("Invalid state_root");
        }

        // Persist fork block
        self.storage.save_block(block)?;
        self.storage.save_block_meta(block_hash, block.header.height, parent_hash)?;
        self.storage.save_state_by_hash(block_hash, &next_state)?;

        // Fork-choice: choose best tip by (height, hash)
        let (cur_tip_h, cur_tip_hash) = self.storage.load_tip()?.unwrap_or((0, [0u8; 32]));
        let better =
            (block.header.height > cur_tip_h) || (block.header.height == cur_tip_h && block_hash > cur_tip_hash);
        if !better {
            return Ok(false);
        }

        // Reorg canonical mapping to this new tip
        self.reorg_to_tip(block_hash, block.header.height).await?;
        Ok(true)
    }

    async fn reorg_to_tip(&self, new_tip_hash: [u8; 32], new_tip_height: u64) -> Result<()> {
        // Walk back to genesis collecting (height, hash)
        let mut chain: Vec<(u64, [u8; 32])> = Vec::new();
        let mut cursor_hash = new_tip_hash;
        loop {
            let (h, parent) = self
                .storage
                .load_block_meta(&cursor_hash)?
                .ok_or_else(|| anyhow::anyhow!("Missing block meta during reorg"))?;
            chain.push((h, cursor_hash));
            if h <= 1 {
                break;
            }
            cursor_hash = parent;
        }
        chain.reverse();

        // Update canonical height map + per-height state snapshots
        for (h, hash) in &chain {
            self.storage.save_canonical_block_at_height(*h, *hash)?;
            let st = self
                .storage
                .load_state_by_hash(hash)?
                .ok_or_else(|| anyhow::anyhow!("Missing state for block during reorg"))?;
            self.storage.save_state_at_height(*h, &st)?;
        }

        // Persist latest state + tip
        let tip_state = self
            .storage
            .load_state_by_hash(&new_tip_hash)?
            .ok_or_else(|| anyhow::anyhow!("Missing tip state"))?;
        self.storage.save_state(&tip_state)?;
        self.storage.save_tip(new_tip_height, new_tip_hash)?;

        // Update in-memory state
        {
            let mut guard = self.state.write().await;
            *guard = tip_state;
        }

        Ok(())
    }
}

fn build_block_from_parent(
    mut parent_state: GlobalState,
    txs: Vec<Transaction>,
    height: u64,
    prev_hash: [u8; 32],
    timestamp: u64,
) -> Result<Block> {
    let mut valid_txs = Vec::new();

    {
        let mut ctx = ExecutionContext {
            state: &mut parent_state,
            height,
            timestamp,
        };

        for tx in txs {
            match execute_transaction(&tx, &mut ctx) {
                Ok(()) => valid_txs.push(tx),
                Err(e) => {
                    error!("Tx execution failed: {}", e);
                }
            }
        }
    }

    if valid_txs.is_empty() {
        bail!("No valid transactions");
    }

    let transactions_root = Block::transactions_root(&valid_txs);
    let state_root = parent_state.root_hash();

    Ok(Block {
        header: BlockHeader {
            height,
            prev_hash,
            transactions_root,
            state_root,
            timestamp,
            proposer: [0u8; 32],
        },
        transactions: valid_txs,
        votes: Vec::new(),
    })
}
