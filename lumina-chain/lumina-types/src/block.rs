use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

fn hash_concat(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BlockHeader {
    pub height: u64,
    pub prev_hash: [u8; 32],
    pub transactions_root: [u8; 32],
    pub state_root: [u8; 32],
    pub timestamp: u64,
    pub proposer: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub votes: Vec<Vote>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Vote {
    pub validator: [u8; 32],
    pub signature: Vec<u8>,
}

impl Block {
    pub fn transactions_root(txs: &[Transaction]) -> [u8; 32] {
        if txs.is_empty() {
            return [0u8; 32];
        }

        let mut level: Vec<[u8; 32]> = txs.iter().map(|tx| tx.id()).collect();
        while level.len() > 1 {
            let mut next = Vec::with_capacity((level.len() + 1) / 2);
            let mut i = 0;
            while i < level.len() {
                let left = level[i];
                let right = if i + 1 < level.len() {
                    level[i + 1]
                } else {
                    left
                };
                next.push(hash_concat(&left, &right));
                i += 2;
            }
            level = next;
        }
        level[0]
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&bincode::serialize(&self.header).expect("block header serialization"));
        *hasher.finalize().as_bytes()
    }
}
