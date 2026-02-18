use lumina_types::state::{GlobalState, AccountState, ValidatorState};
use std::collections::HashMap;

pub fn create_genesis_state() -> GlobalState {
    let mut accounts = HashMap::new();
    let mut validators = Vec::new();

    // 1. Initial Validator (Developer Key)
    // In a real scenario, this would be a config.
    // Let's use a dummy key for now.
    let validator_pubkey = [0u8; 32]; // Replace with actual key
    validators.push(ValidatorState {
        pubkey: validator_pubkey,
        stake: 1_000_000,
        power: 1_000_000,
    });

    // 2. Initial Accounts
    // Deployer account with initial Lumina (gas)
    let deployer_addr = [0u8; 32];
    accounts.insert(deployer_addr, AccountState {
        nonce: 0,
        lusd_balance: 0,
        ljun_balance: 0,
        lumina_balance: 1_000_000_000, // 1 billion gas tokens
        commitment: None,
    });

    // 3. Initial Oracle
    let mut oracle_prices = HashMap::new();
    oracle_prices.insert("ETH-USD".to_string(), 3000_000_000); // $3000.000000
    oracle_prices.insert("BTC-USD".to_string(), 90000_000_000); // $90000.000000

    GlobalState {
        accounts,
        total_lusd_supply: 0,
        total_ljun_supply: 0,
        stabilization_pool_balance: 0,
        reserve_ratio: 1.0, // 100% backed initially
        oracle_prices,
        validators,
        circuit_breaker_active: false,
        fair_redeem_queue: Vec::new(),
        last_rebalance_height: 0,
    }
}
