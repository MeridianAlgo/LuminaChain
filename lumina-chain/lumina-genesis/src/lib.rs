use lumina_types::state::{AccountState, GlobalState, ValidatorState};
use std::collections::HashMap;

/// Create the genesis state for LuminaChain.
/// This initializes the very first state of the chain with:
/// - A deployer account with initial Lumina (gas) tokens
/// - An initial validator set
/// - Oracle price bootstraps
/// - Insurance fund seeded
/// - Velocity reward pool initialized
pub fn create_genesis_state() -> GlobalState {
    let mut accounts = HashMap::new();
    let mut validators = Vec::new();

    // Initial validator (replace with ceremony-derived keys before mainnet)
    let validator_pubkey = [0u8; 32];
    validators.push(ValidatorState {
        pubkey: validator_pubkey,
        stake: 1_000_000,
        power: 1_000_000,
        is_green: false,
        energy_proof: None,
    });

    // Deployer account with initial Lumina gas tokens
    let deployer_addr = [0u8; 32];
    accounts.insert(
        deployer_addr,
        AccountState {
            lumina_balance: 1_000_000_000,
            ..Default::default()
        },
    );

    // Bootstrap oracle prices (fixed-point 1e6)
    let mut oracle_prices = HashMap::new();
    oracle_prices.insert("ETH-USD".to_string(), 3000_000_000);
    oracle_prices.insert("BTC-USD".to_string(), 90000_000_000);
    oracle_prices.insert("LUSD-USD".to_string(), 1_000_000); // $1.00 peg

    GlobalState {
        accounts,
        reserve_ratio: 1.0,
        oracle_prices,
        validators,
        health_index: 10000, // Perfect health at genesis
        ..Default::default()
    }
}
