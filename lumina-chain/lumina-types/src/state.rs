use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AccountState {
    pub nonce: u64,
    pub lusd_balance: u64,
    pub ljun_balance: u64,
    pub lumina_balance: u64, // Native gas token
    pub commitment: Option<[u8; 32]>, // Phase 3: Pedersen Commitment for privacy
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GlobalState {
    pub accounts: HashMap<[u8; 32], AccountState>,
    pub total_lusd_supply: u64,
    pub total_ljun_supply: u64,
    
    // === Stability & Tranches ===
    pub stabilization_pool_balance: u64, // USDC or similar backing
    pub reserve_ratio: f64,              // Senior tranche backing ratio (Collateral / LUSD)
    pub oracle_prices: HashMap<String, u64>,
    pub validators: Vec<ValidatorState>,

    // === Protection ===
    pub circuit_breaker_active: bool,
    pub fair_redeem_queue: Vec<RedemptionRequest>,
    pub last_rebalance_height: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RedemptionRequest {
    pub address: [u8; 32],
    pub amount: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidatorState {
    pub pubkey: [u8; 32],
    pub stake: u64,
    pub power: u64,
}
