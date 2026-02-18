use super::*;
use lumina_types::instruction::StablecoinInstruction;
use lumina_types::state::{AccountState, GlobalState};
use lumina_types::transaction::Transaction;

fn new_sender() -> ([u8; 32], lumina_crypto::signatures::SigningKey) {
    let kp = lumina_crypto::signatures::generate_keypair();
    (kp.verifying_key().to_bytes(), kp)
}

#[test]
fn test_stabilization_rebalance() {
    let mut state = GlobalState::default();
    let (sender, _kp) = new_sender();

    state.total_lusd_supply = 1_000_000;
    state.reserve_ratio = 0.90;
    state.stabilization_pool_balance = 500_000;
    state
        .oracle_prices
        .insert("ETH-USD".to_string(), 3000_000_000);

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let si = StablecoinInstruction::TriggerStabilizer;
        assert!(execute_si(&si, &sender, &mut ctx).is_ok());
    }

    assert!(state.reserve_ratio > 0.0);
}

#[test]
fn test_circuit_breaker_logic() {
    let mut state = GlobalState::default();
    let (sender, kp) = new_sender();

    state.total_lusd_supply = 1_000_000;
    state.stabilization_pool_balance = 100_000;

    let mint_si = StablecoinInstruction::MintSenior {
        amount: 1,
        collateral_amount: 1,
        proof: vec![],
    };

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        assert!(execute_si(&mint_si, &sender, &mut ctx).is_ok());
    }

    assert!(state.reserve_ratio < 0.85);
    assert!(state.circuit_breaker_active);

    // Build a signed tx to verify circuit breaker blocks mints
    let mut tx = Transaction {
        sender,
        nonce: 0,
        instruction: mint_si,
        signature: vec![0u8; 64],
        gas_limit: 1000,
        gas_price: 1,
    };
    tx.signature = lumina_crypto::signatures::sign(&kp, &tx.signing_bytes());
    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 2,
        timestamp: 200,
    };
    assert!(execute_transaction(&tx, &mut ctx).is_err());
}

#[test]
fn test_redemption_queueing() {
    let mut state = GlobalState::default();
    let sender = [1u8; 32];

    state.accounts.insert(
        sender,
        AccountState {
            nonce: 0,
            lusd_balance: 5000,
            ..Default::default()
        },
    );
    state.total_lusd_supply = 5000;
    state.reserve_ratio = 0.90;

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let redeem_si = StablecoinInstruction::RedeemSenior { amount: 1000 };
        assert!(execute_si(&redeem_si, &sender, &mut ctx).is_ok());
    }

    assert_eq!(state.accounts.get(&sender).unwrap().lusd_balance, 4000);
    assert_eq!(state.total_lusd_supply, 5000);
    assert_eq!(state.fair_redeem_queue.len(), 1);

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let process_si = StablecoinInstruction::FairRedeemQueue { batch_size: 1 };
        assert!(execute_si(&process_si, &sender, &mut ctx).is_ok());
    }

    assert_eq!(state.total_lusd_supply, 4000);
    assert_eq!(state.fair_redeem_queue.len(), 0);
}

#[test]
fn test_passkey_account_creation() {
    let mut state = GlobalState::default();
    let sender = [2u8; 32];

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 100,
    };

    let si = StablecoinInstruction::CreatePasskeyAccount {
        device_key: vec![1u8; 65],
        guardians: vec![[3u8; 32], [4u8; 32], [5u8; 32]],
    };
    assert!(execute_si(&si, &sender, &mut ctx).is_ok());

    let acct = state.accounts.get(&sender).unwrap();
    assert!(acct.passkey_device_key.is_some());
    assert_eq!(acct.guardians.len(), 3);
}

#[test]
fn test_insurance_fund_mechanics() {
    let mut state = GlobalState::default();
    let sender = [6u8; 32];

    // Mint senior — 5% should go to insurance fund
    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 100,
    };

    let si = StablecoinInstruction::MintSenior {
        amount: 1000,
        collateral_amount: 1200,
        proof: vec![],
    };
    assert!(execute_si(&si, &sender, &mut ctx).is_ok());

    // 5% of 1000 = 50 fee to insurance
    assert_eq!(state.insurance_fund_balance, 50);
    assert_eq!(state.accounts.get(&sender).unwrap().lusd_balance, 950);
    assert_eq!(state.total_lusd_supply, 950);
}

#[test]
fn test_yield_token_wrap_unwrap() {
    let mut state = GlobalState::default();
    let sender = [7u8; 32];

    state.accounts.insert(
        sender,
        AccountState {
            lusd_balance: 10000,
            ..Default::default()
        },
    );
    state.total_lusd_supply = 10000;
    state.stabilization_pool_balance = 10000;

    // Wrap
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 100,
            timestamp: 1000,
        };
        let si = StablecoinInstruction::WrapToYieldToken {
            amount: 5000,
            maturity_blocks: 100,
        };
        assert!(execute_si(&si, &sender, &mut ctx).is_ok());
    }

    assert_eq!(state.accounts.get(&sender).unwrap().lusd_balance, 5000);
    assert_eq!(
        state.accounts.get(&sender).unwrap().yield_positions.len(),
        1
    );

    // Unwrap (at maturity)
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 3_153_800,
            timestamp: 2500,
        };
        let si = StablecoinInstruction::UnwrapYieldToken { token_id: 0 };
        assert!(execute_si(&si, &sender, &mut ctx).is_ok());
    }

    // Should have principal back plus net yield, and insurance pool should receive junior-yield cut.
    assert!(state.accounts.get(&sender).unwrap().lusd_balance > 10000);
    assert!(state.insurance_fund_balance > 0);
    assert_eq!(
        state.accounts.get(&sender).unwrap().yield_positions.len(),
        0
    );
}

#[test]
fn test_health_index_computation() {
    let mut state = GlobalState::default();
    state.total_lusd_supply = 1_000_000;
    state.stabilization_pool_balance = 1_000_000;
    state.reserve_ratio = 1.0;
    state.insurance_fund_balance = 50_000;
    state
        .oracle_prices
        .insert("LUSD-USD".to_string(), 1_000_000);

    let sender = [8u8; 32];
    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 100,
    };

    let si = StablecoinInstruction::ComputeHealthIndex;
    assert!(execute_si(&si, &sender, &mut ctx).is_ok());

    // Should be a reasonably high health index (above 5000)
    assert!(state.health_index > 5000);
}

#[test]
fn test_parallel_non_conflicting_transfers() {
    let mut state = GlobalState::default();
    let (s1, k1) = new_sender();
    let (s2, k2) = new_sender();
    let r1 = [9u8; 32];
    let r2 = [10u8; 32];

    state.accounts.entry(s1).or_default().lusd_balance = 100;
    state.accounts.entry(s2).or_default().lusd_balance = 100;

    let mut tx1 = Transaction {
        sender: s1,
        nonce: 0,
        instruction: StablecoinInstruction::Transfer {
            to: r1,
            amount: 10,
            asset: lumina_types::instruction::AssetType::LUSD,
        },
        signature: vec![0; 64],
        gas_limit: 1_000_000,
        gas_price: 1,
    };
    tx1.signature = lumina_crypto::signatures::sign(&k1, &tx1.signing_bytes());

    let mut tx2 = Transaction {
        sender: s2,
        nonce: 0,
        instruction: StablecoinInstruction::Transfer {
            to: r2,
            amount: 20,
            asset: lumina_types::instruction::AssetType::LUSD,
        },
        signature: vec![0; 64],
        gas_limit: 1_000_000,
        gas_price: 1,
    };
    tx2.signature = lumina_crypto::signatures::sign(&k2, &tx2.signing_bytes());

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 1,
    };

    execute_transactions_parallel_non_conflicting(&[tx1, tx2], &mut ctx).unwrap();

    assert_eq!(ctx.state.accounts.get(&r1).unwrap().lusd_balance, 10);
    assert_eq!(ctx.state.accounts.get(&r2).unwrap().lusd_balance, 20);
}
