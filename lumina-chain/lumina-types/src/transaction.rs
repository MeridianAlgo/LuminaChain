use serde::{Serialize, Deserialize};
use crate::instruction::StablecoinInstruction;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub sender: [u8; 32],
    pub nonce: u64,
    pub instruction: StablecoinInstruction,
    pub signature: Vec<u8>,
    pub gas_limit: u64,
    pub gas_price: u64,
}

impl Transaction {
    pub fn signing_bytes(&self) -> Vec<u8> {
        #[derive(Serialize)]
        struct SigningTx<'a> {
            sender: &'a [u8; 32],
            nonce: u64,
            instruction: &'a StablecoinInstruction,
            gas_limit: u64,
            gas_price: u64,
        }

        let signing = SigningTx {
            sender: &self.sender,
            nonce: self.nonce,
            instruction: &self.instruction,
            gas_limit: self.gas_limit,
            gas_price: self.gas_price,
        };

        bincode::serialize(&signing).expect("tx signing serialization")
    }

    pub fn id(&self) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&self.signing_bytes());
        hasher.update(&self.signature);
        *hasher.finalize().as_bytes()
    }
}
