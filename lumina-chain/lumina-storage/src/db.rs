use rocksdb::{DB, Options};
use anyhow::{anyhow, Result};
use lumina_types::state::GlobalState;
use lumina_types::block::Block;
use bincode;

pub struct Storage {
    pub db: DB,
}

impl Storage {
    pub fn new(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| anyhow!("Failed to open DB: {}", e))?;
        Ok(Self { db })
    }

    pub fn save_state(&self, state: &GlobalState) -> Result<()> {
        let encoded: Vec<u8> = bincode::serialize(state).map_err(|e| anyhow!("Serialization error: {}", e))?;
        self.db.put(b"global_state", encoded).map_err(|e| anyhow!("DB write error: {}", e))?;
        Ok(())
    }

    pub fn load_state(&self) -> Result<GlobalState> {
        match self.db.get(b"global_state") {
            Ok(Some(value)) => {
                let decoded: GlobalState = bincode::deserialize(&value).map_err(|e| anyhow!("Deserialization error: {}", e))?;
                Ok(decoded)
            }
            Ok(None) => Ok(GlobalState::default()),
            Err(e) => Err(anyhow!("DB read error: {}", e)),
        }
    }

    pub fn save_block(&self, block: &Block) -> Result<()> {
        let height_key = format!("block_height_{}", block.header.height);
        let hash_key = format!("block_hash_{}", hex::encode(block.hash()));
        let encoded = bincode::serialize(block).map_err(|e| anyhow!("Serialization error: {}", e))?;
        
        self.db.put(height_key.as_bytes(), &encoded).map_err(|e| anyhow!("DB height-index error: {}", e))?;
        self.db.put(hash_key.as_bytes(), &encoded).map_err(|e| anyhow!("DB hash-index error: {}", e))?;
        Ok(())
    }

    pub fn load_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let key = format!("block_height_{}", height);
        match self.db.get(key.as_bytes())? {
             Some(v) => Ok(Some(bincode::deserialize(&v)?)),
             None => Ok(None),
        }
    }
}
