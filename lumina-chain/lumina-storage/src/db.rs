use anyhow::{anyhow, Result};
use lumina_types::block::Block;
use lumina_types::state::GlobalState;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[cfg(feature = "rocksdb")]
use rocksdb::{Options, DB};

#[cfg(feature = "rocksdb")]
pub struct Storage {
    pub db: DB,
}

#[cfg(feature = "rocksdb")]
impl Storage {
    pub fn new(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| anyhow!("Failed to open DB: {}", e))?;
        Ok(Self { db })
    }

    pub fn save_state(&self, state: &GlobalState) -> Result<()> {
        let encoded: Vec<u8> =
            bincode::serialize(state).map_err(|e| anyhow!("Serialization error: {}", e))?;
        self.db
            .put(b"global_state", encoded)
            .map_err(|e| anyhow!("DB write error: {}", e))?;
        Ok(())
    }

    pub fn save_state_at_height(&self, height: u64, state: &GlobalState) -> Result<()> {
        let key = format!("state_height_{}", height);
        let encoded: Vec<u8> =
            bincode::serialize(state).map_err(|e| anyhow!("Serialization error: {}", e))?;
        self.db
            .put(key.as_bytes(), encoded)
            .map_err(|e| anyhow!("DB write error: {}", e))?;
        Ok(())
    }

    pub fn load_state(&self) -> Result<GlobalState> {
        match self.db.get(b"global_state") {
            Ok(Some(value)) => {
                let decoded: GlobalState = bincode::deserialize(&value)
                    .map_err(|e| anyhow!("Deserialization error: {}", e))?;
                Ok(decoded)
            }
            Ok(None) => Ok(GlobalState::default()),
            Err(e) => Err(anyhow!("DB read error: {}", e)),
        }
    }

    pub fn load_state_by_height(&self, height: u64) -> Result<Option<GlobalState>> {
        let key = format!("state_height_{}", height);
        match self.db.get(key.as_bytes())? {
            Some(v) => Ok(Some(bincode::deserialize(&v)?)),
            None => Ok(None),
        }
    }

    pub fn save_state_by_hash(&self, block_hash: [u8; 32], state: &GlobalState) -> Result<()> {
        let key = format!("state_hash_{}", hex::encode(block_hash));
        let encoded: Vec<u8> =
            bincode::serialize(state).map_err(|e| anyhow!("Serialization error: {}", e))?;
        self.db
            .put(key.as_bytes(), encoded)
            .map_err(|e| anyhow!("DB write error: {}", e))?;
        Ok(())
    }

    pub fn load_state_by_hash(&self, block_hash: &[u8; 32]) -> Result<Option<GlobalState>> {
        let key = format!("state_hash_{}", hex::encode(block_hash));
        match self.db.get(key.as_bytes())? {
            Some(v) => Ok(Some(bincode::deserialize(&v)?)),
            None => Ok(None),
        }
    }

    pub fn save_block(&self, block: &Block) -> Result<()> {
        let hash_key = format!("block_hash_{}", hex::encode(block.hash()));
        let encoded =
            bincode::serialize(block).map_err(|e| anyhow!("Serialization error: {}", e))?;
        self.db
            .put(hash_key.as_bytes(), &encoded)
            .map_err(|e| anyhow!("DB hash-index error: {}", e))?;
        Ok(())
    }

    pub fn save_canonical_block_at_height(&self, height: u64, block_hash: [u8; 32]) -> Result<()> {
        let height_key = format!("block_height_{}", height);
        self.db
            .put(height_key.as_bytes(), block_hash)
            .map_err(|e| anyhow!("DB height-index error: {}", e))?;
        Ok(())
    }

    pub fn load_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let key = format!("block_height_{}", height);
        match self.db.get(key.as_bytes())? {
            Some(v) => {
                if v.len() != 32 {
                    return Err(anyhow!("Invalid canonical block hash length"));
                }
                let mut h = [0u8; 32];
                h.copy_from_slice(&v);
                self.load_block_by_hash(&h)
            }
            None => Ok(None),
        }
    }

    pub fn load_block_by_hash(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        let key = format!("block_hash_{}", hex::encode(hash));
        match self.db.get(key.as_bytes())? {
            Some(v) => Ok(Some(bincode::deserialize(&v)?)),
            None => Ok(None),
        }
    }

    pub fn save_block_meta(
        &self,
        block_hash: [u8; 32],
        height: u64,
        parent_hash: [u8; 32],
    ) -> Result<()> {
        let key = format!("block_meta_{}", hex::encode(block_hash));
        let encoded = bincode::serialize(&(height, parent_hash))?;
        self.db
            .put(key.as_bytes(), encoded)
            .map_err(|e| anyhow!("DB write error: {}", e))?;
        Ok(())
    }

    pub fn load_block_meta(&self, block_hash: &[u8; 32]) -> Result<Option<(u64, [u8; 32])>> {
        let key = format!("block_meta_{}", hex::encode(block_hash));
        match self.db.get(key.as_bytes())? {
            Some(v) => Ok(Some(bincode::deserialize(&v)?)),
            None => Ok(None),
        }
    }

    pub fn save_tip(&self, height: u64, hash: [u8; 32]) -> Result<()> {
        self.db
            .put(b"chain_tip_height", bincode::serialize(&height)?)
            .map_err(|e| anyhow!("DB tip write error: {}", e))?;
        self.db
            .put(b"chain_tip_hash", hash)
            .map_err(|e| anyhow!("DB tip write error: {}", e))?;
        Ok(())
    }

    pub fn load_tip(&self) -> Result<Option<(u64, [u8; 32])>> {
        let height = match self.db.get(b"chain_tip_height") {
            Ok(Some(v)) => Some(bincode::deserialize::<u64>(&v)?),
            Ok(None) => None,
            Err(e) => return Err(anyhow!("DB tip read error: {}", e)),
        };

        let hash = match self.db.get(b"chain_tip_hash") {
            Ok(Some(v)) => {
                if v.len() != 32 {
                    return Err(anyhow!("Invalid chain tip hash length"));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&v);
                Some(arr)
            }
            Ok(None) => None,
            Err(e) => return Err(anyhow!("DB tip read error: {}", e)),
        };

        Ok(match (height, hash) {
            (Some(h), Some(x)) => Some((h, x)),
            (None, None) => None,
            _ => return Err(anyhow!("Corrupt tip: missing height or hash")),
        })
    }
}

#[cfg(not(feature = "rocksdb"))]
#[derive(Clone, Default)]
pub struct Storage {
    inner: Arc<RwLock<MemDb>>,
}

#[cfg(not(feature = "rocksdb"))]
#[derive(Default)]
struct MemDb {
    global_state: Option<GlobalState>,
    canonical_hash_by_height: HashMap<u64, [u8; 32]>,
    blocks_by_hash: HashMap<[u8; 32], Block>,
    states_by_height: HashMap<u64, GlobalState>,
    states_by_hash: HashMap<[u8; 32], GlobalState>,
    block_meta: HashMap<[u8; 32], (u64, [u8; 32])>,
    tip: Option<(u64, [u8; 32])>,
}

#[cfg(not(feature = "rocksdb"))]
impl Storage {
    pub fn new(_path: &str) -> Result<Self> {
        Ok(Self::default())
    }

    pub fn save_state(&self, state: &GlobalState) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        guard.global_state = Some(state.clone());
        Ok(())
    }

    pub fn save_state_at_height(&self, height: u64, state: &GlobalState) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        guard.states_by_height.insert(height, state.clone());
        Ok(())
    }

    pub fn load_state(&self) -> Result<GlobalState> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        Ok(guard.global_state.clone().unwrap_or_default())
    }

    pub fn load_state_by_height(&self, height: u64) -> Result<Option<GlobalState>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        Ok(guard.states_by_height.get(&height).cloned())
    }

    pub fn save_state_by_hash(&self, block_hash: [u8; 32], state: &GlobalState) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        guard.states_by_hash.insert(block_hash, state.clone());
        Ok(())
    }

    pub fn load_state_by_hash(&self, block_hash: &[u8; 32]) -> Result<Option<GlobalState>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        Ok(guard.states_by_hash.get(block_hash).cloned())
    }

    pub fn save_block(&self, block: &Block) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        guard.blocks_by_hash.insert(block.hash(), block.clone());
        Ok(())
    }

    pub fn save_canonical_block_at_height(&self, height: u64, block_hash: [u8; 32]) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        guard.canonical_hash_by_height.insert(height, block_hash);
        Ok(())
    }

    pub fn load_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        match guard.canonical_hash_by_height.get(&height) {
            Some(h) => Ok(guard.blocks_by_hash.get(h).cloned()),
            None => Ok(None),
        }
    }

    pub fn load_block_by_hash(&self, hash: &[u8; 32]) -> Result<Option<Block>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        Ok(guard.blocks_by_hash.get(hash).cloned())
    }

    pub fn save_block_meta(
        &self,
        block_hash: [u8; 32],
        height: u64,
        parent_hash: [u8; 32],
    ) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        guard.block_meta.insert(block_hash, (height, parent_hash));
        Ok(())
    }

    pub fn load_block_meta(&self, block_hash: &[u8; 32]) -> Result<Option<(u64, [u8; 32])>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        Ok(guard.block_meta.get(block_hash).cloned())
    }

    pub fn save_tip(&self, height: u64, hash: [u8; 32]) -> Result<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        guard.tip = Some((height, hash));
        Ok(())
    }

    pub fn load_tip(&self) -> Result<Option<(u64, [u8; 32])>> {
        let guard = self
            .inner
            .read()
            .map_err(|_| anyhow!("Storage lock poisoned"))?;
        Ok(guard.tip)
    }
}
