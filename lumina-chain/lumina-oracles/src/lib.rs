pub mod price_feed;

pub struct OracleManager {
    pub reporters: Vec<[u8; 32]>, // Public keys of authorized reporters
}

impl OracleManager {
    pub fn new() -> Self {
        Self {
            reporters: Vec::new(),
        }
    }
}
