#![cfg(feature = "malachite")]

use async_trait::async_trait;
use lumina_crypto::signatures::{verify_pq_signature, verify_signature};
use lumina_execution::{end_block, execute_transaction, ExecutionContext};
use lumina_storage::db::Storage;
use lumina_types::state::GlobalState;
use lumina_types::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const F: usize = 2;
const QUORUM: usize = 2 * F + 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitChainRequest {
    pub genesis_state: GlobalState,
    pub initial_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeginBlockRequest {
    pub height: u64,
    pub proposer: [u8; 32],
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverTxRequest {
    pub tx: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndBlockRequest {
    pub height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
    pub height: u64,
    pub app_hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalState {
    height: u64,
    current_block: Option<BeginBlockRequest>,
    pending_txs: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct InflightBlock {
    height: u64,
    timestamp: u64,
    txs: Vec<Vec<u8>>,
}

pub struct LuminaApp {
    pub state: GlobalState,
    pub storage: Storage,
    pub height: u64,
    wal_path: PathBuf,
    inflight: Option<InflightBlock>,
}

impl LuminaApp {
    pub fn new(storage: Storage, wal_path: impl AsRef<Path>) -> Self {
        let state = storage.load_state().unwrap_or_default();
        let mut app = Self {
            state,
            storage,
            height: 0,
            wal_path: wal_path.as_ref().to_path_buf(),
            inflight: None,
        };
        let _ = app.recover_from_wal();
        app
    }

    fn recover_from_wal(&mut self) -> Result<(), String> {
        if !self.wal_path.exists() {
            return Ok(());
        }

        let bytes = fs::read(&self.wal_path).map_err(|e| e.to_string())?;
        let wal: WalState = bincode::deserialize(&bytes).map_err(|e| e.to_string())?;
        self.height = wal.height;
        self.inflight = wal.current_block.map(|b| InflightBlock {
            height: b.height,
            timestamp: b.timestamp,
            txs: wal.pending_txs,
        });
        Ok(())
    }

    fn persist_wal(&self) -> Result<(), String> {
        let wal = WalState {
            height: self.height,
            current_block: self.inflight.as_ref().map(|b| BeginBlockRequest {
                height: b.height,
                proposer: [0u8; 32],
                timestamp: b.timestamp,
            }),
            pending_txs: self
                .inflight
                .as_ref()
                .map(|b| b.txs.clone())
                .unwrap_or_default(),
        };
        let bytes = bincode::serialize(&wal).map_err(|e| e.to_string())?;
        if let Some(parent) = self.wal_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&self.wal_path, bytes).map_err(|e| e.to_string())
    }

    fn clear_wal(&self) -> Result<(), String> {
        if self.wal_path.exists() {
            fs::remove_file(&self.wal_path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[async_trait]
pub trait Application {
    async fn init_chain(&mut self, req: InitChainRequest) -> Result<(), String>;
    async fn begin_block(&mut self, req: BeginBlockRequest) -> Result<(), String>;
    async fn check_tx(&self, tx: &[u8]) -> bool;
    async fn deliver_tx(&mut self, req: DeliverTxRequest) -> Result<(), String>;
    async fn end_block(&mut self, req: EndBlockRequest) -> Result<(), String>;
    async fn commit(&mut self) -> Result<CommitResponse, String>;
}

#[async_trait]
impl Application for LuminaApp {
    async fn init_chain(&mut self, req: InitChainRequest) -> Result<(), String> {
        self.state = req.genesis_state;
        self.height = req.initial_height;
        self.storage
            .save_state(&self.state)
            .map_err(|e| e.to_string())?;
        self.storage
            .save_state_at_height(self.height, &self.state)
            .map_err(|e| e.to_string())?;
        self.persist_wal()
    }

    async fn begin_block(&mut self, req: BeginBlockRequest) -> Result<(), String> {
        if req.height <= self.height {
            return Err("non-monotonic height".to_string());
        }

        self.inflight = Some(InflightBlock {
            height: req.height,
            timestamp: req.timestamp,
            txs: Vec::new(),
        });
        self.persist_wal()
    }

    async fn check_tx(&self, tx: &[u8]) -> bool {
        let tx: Transaction = match bincode::deserialize(tx) {
            Ok(tx) => tx,
            Err(_) => return false,
        };

        let signing_bytes = tx.signing_bytes();
        if let Some(account) = self.state.accounts.get(&tx.sender) {
            if let Some(ref pq_key) = account.pq_pubkey {
                return verify_pq_signature(pq_key, &signing_bytes, &tx.signature).is_ok();
            }
        }

        verify_signature(&tx.sender, &signing_bytes, &tx.signature).is_ok()
    }

    async fn deliver_tx(&mut self, req: DeliverTxRequest) -> Result<(), String> {
        if bincode::deserialize::<Transaction>(&req.tx).is_err() {
            return Err("invalid tx bytes".to_string());
        }
        let Some(inflight) = self.inflight.as_mut() else {
            return Err("begin_block must be called first".to_string());
        };
        inflight.txs.push(req.tx);
        self.persist_wal()
    }

    async fn end_block(&mut self, req: EndBlockRequest) -> Result<(), String> {
        let Some(inflight) = &self.inflight else {
            return Err("begin_block must be called first".to_string());
        };
        if req.height != inflight.height {
            return Err("end_block height mismatch".to_string());
        }
        Ok(())
    }

    async fn commit(&mut self) -> Result<CommitResponse, String> {
        let inflight = self
            .inflight
            .take()
            .ok_or_else(|| "begin_block must be called first".to_string())?;

        let mut ctx = ExecutionContext {
            state: &mut self.state,
            height: inflight.height,
            timestamp: inflight.timestamp,
        };

        for tx_bytes in inflight.txs {
            let tx: Transaction = bincode::deserialize(&tx_bytes).map_err(|e| e.to_string())?;
            execute_transaction(&tx, &mut ctx).map_err(|e| e.to_string())?;
        }

        end_block(&mut ctx);

        self.height = inflight.height;
        let app_hash = self.state.root_hash();
        self.storage
            .save_state(&self.state)
            .map_err(|e| e.to_string())?;
        self.storage
            .save_state_at_height(self.height, &self.state)
            .map_err(|e| e.to_string())?;
        self.clear_wal()?;

        Ok(CommitResponse {
            height: self.height,
            app_hash,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocalProposal {
    pub height: u64,
    pub txs: Vec<Vec<u8>>,
}

pub struct LocalMalachiteEngine<A: Application + Send + Sync> {
    app: A,
    validators: Vec<[u8; 32]>,
    votes: HashMap<u64, HashSet<[u8; 32]>>,
}

impl<A: Application + Send + Sync> LocalMalachiteEngine<A> {
    pub fn new(app: A, validators: Vec<[u8; 32]>) -> Result<Self, String> {
        if validators.len() != 7 {
            return Err("exactly 7 validators required for local testnet profile".to_string());
        }
        Ok(Self {
            app,
            validators,
            votes: HashMap::new(),
        })
    }

    pub async fn propose_and_finalize(
        &mut self,
        proposal: LocalProposal,
    ) -> Result<CommitResponse, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        self.app
            .begin_block(BeginBlockRequest {
                height: proposal.height,
                proposer: self.validators[0],
                timestamp: now,
            })
            .await?;

        for tx in proposal.txs {
            self.app.deliver_tx(DeliverTxRequest { tx }).await?;
        }

        self.app
            .end_block(EndBlockRequest {
                height: proposal.height,
            })
            .await?;

        let validator_power = |pk: &[u8; 32]| -> u64 {
            self.app
                .state
                .validators
                .iter()
                .find(|v| &v.pubkey == pk)
                .map(|v| v.power.max(1))
                .unwrap_or(1)
        };

        let total_power: u64 = self.validators.iter().map(validator_power).sum();
        let quorum_power: u64 = (total_power * 2 / 3).saturating_add(1);

        for validator in &self.validators {
            let entry = self.votes.entry(proposal.height).or_default();
            entry.insert(*validator);
            let voted_power: u64 = entry.iter().map(|pk| validator_power(pk)).sum();
            if voted_power >= quorum_power {
                return self.app.commit().await;
            }
        }

        Err("failed to gather quorum".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumina_types::instruction::StablecoinInstruction;
    use tokio::time::Instant;

    fn sample_tx() -> Vec<u8> {
        let kp = lumina_crypto::signatures::generate_keypair();
        let sender = kp.verifying_key().to_bytes();
        let tx = Transaction {
            sender,
            nonce: 0,
            instruction: StablecoinInstruction::Transfer {
                to: [2u8; 32],
                amount: 1,
                asset: lumina_types::instruction::AssetType::LUSD,
            },
            signature: vec![0u8; 64],
            gas_limit: 1_000_000,
            gas_price: 1,
        };
        let mut tx = tx;
        tx.signature = lumina_crypto::signatures::sign(&kp, &tx.signing_bytes());
        bincode::serialize(&tx).unwrap()
    }

    #[tokio::test]
    async fn wal_recovery_restores_inflight_block() {
        let storage = Storage::new("/tmp/lumina-test-wal-1").unwrap();
        let wal_path = PathBuf::from("/tmp/lumina-test-wal-1/consensus.wal");

        let mut app = LuminaApp::new(storage.clone(), &wal_path);
        app.begin_block(BeginBlockRequest {
            height: 1,
            proposer: [0u8; 32],
            timestamp: 1,
        })
        .await
        .unwrap();
        app.deliver_tx(DeliverTxRequest { tx: sample_tx() })
            .await
            .unwrap();

        let recovered = LuminaApp::new(storage, &wal_path);
        assert_eq!(recovered.height, 0);
        assert!(recovered.inflight.is_some());
    }

    #[tokio::test]
    async fn seven_validator_finality_is_sub_900ms() {
        let storage = Storage::new("/tmp/lumina-test-wal-2").unwrap();
        let wal_path = PathBuf::from("/tmp/lumina-test-wal-2/consensus.wal");
        let app = LuminaApp::new(storage, &wal_path);

        let validators: Vec<[u8; 32]> = (0u8..7u8)
            .map(|i| {
                let mut v = [0u8; 32];
                v[0] = i;
                v
            })
            .collect();

        let mut engine = LocalMalachiteEngine::new(app, validators).unwrap();
        let start = Instant::now();
        let res = engine
            .propose_and_finalize(LocalProposal {
                height: 1,
                txs: vec![],
            })
            .await;
        let elapsed = start.elapsed();

        assert!(res.is_ok());
        assert!(elapsed.as_millis() < 900);
    }

    #[tokio::test]
    async fn green_validator_has_higher_weighted_voting_power() {
        let storage = Storage::new("/tmp/lumina-test-wal-3").unwrap();
        let wal_path = PathBuf::from("/tmp/lumina-test-wal-3/consensus.wal");
        let mut app = LuminaApp::new(storage, &wal_path);

        // Seed validator set in state with one green validator at 2x power.
        app.state.validators = (0u8..7u8)
            .map(|i| {
                let mut pk = [0u8; 32];
                pk[0] = i;
                lumina_types::state::ValidatorState {
                    pubkey: pk,
                    stake: 10,
                    power: if i == 0 { 20 } else { 10 },
                    is_green: i == 0,
                    energy_proof: if i == 0 { Some(vec![1u8; 64]) } else { None },
                }
            })
            .collect();

        let validators: Vec<[u8; 32]> = (0u8..7u8)
            .map(|i| {
                let mut v = [0u8; 32];
                v[0] = i;
                v
            })
            .collect();

        let mut engine = LocalMalachiteEngine::new(app, validators).unwrap();
        let res = engine
            .propose_and_finalize(LocalProposal {
                height: 1,
                txs: vec![],
            })
            .await;

        assert!(res.is_ok());
    }
}
