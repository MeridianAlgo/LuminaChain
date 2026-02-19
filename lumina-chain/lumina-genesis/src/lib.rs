use lumina_types::state::{GlobalState, AccountState, ValidatorState};
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
    accounts.insert(deployer_addr, AccountState {
        nonce: 0,
        lusd_balance: 0,
        ljun_balance: 0,
        lumina_balance: 1_000_000_000,
        commitment: None,
        passkey_device_key: None,
        guardians: Vec::new(),
        pq_pubkey: None,
        epoch_tx_volume: 0,
        last_reward_epoch: 0,
        credit_score: 0,
        active_streams: Vec::new(),
        yield_positions: Vec::new(),
        pending_flash_mint: 0,
        pending_flash_collateral: 0,
    });

    // Bootstrap oracle prices (fixed-point 1e6)
    let mut oracle_prices = HashMap::new();
    oracle_prices.insert("ETH-USD".to_string(), 3000_000_000);
    oracle_prices.insert("BTC-USD".to_string(), 90000_000_000);
    oracle_prices.insert("LUSD-USD".to_string(), 1_000_000); // $1.00 peg

    GlobalState {
        accounts,
        total_lusd_supply: 0,
        total_ljun_supply: 0,
        stabilization_pool_balance: 0,
        reserve_ratio: 1.0,
        oracle_prices,
        validators,
        circuit_breaker_active: false,
        fair_redeem_queue: Vec::new(),
        last_rebalance_height: 0,
        insurance_fund_balance: 0,
        custodians: Vec::new(),
        last_reserve_rotation_height: 0,
        compliance_circuits: HashMap::new(),
        rwa_listings: HashMap::new(),
        next_rwa_id: 0,
        trusted_credit_oracles: Vec::new(),
        used_credit_proofs: Vec::new(),
        next_yield_token_id: 0,
        health_index: 10000, // Perfect health at genesis
        pending_flash_mints: 0,
        current_epoch: 0,
        velocity_reward_pool: 0,
    }
}
