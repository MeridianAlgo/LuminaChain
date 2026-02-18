use super::*;
use lumina_types::state::{GlobalState, AccountState};
use lumina_types::instruction::{StablecoinInstruction, AssetType};
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
    state.oracle_prices.insert("ETH-USD".to_string(), 3000_000_000);

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

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let _tx = Transaction {
            sender,
            nonce: 0,
            instruction: mint_si.clone(),
            signature: vec![],
            gas_limit: 1000,
            gas_price: 1,
        };
    }
    
    let mut tx = Transaction {
        sender,
        nonce: 1,
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
    
    state.accounts.insert(sender, AccountState {
        nonce: 0,
        lusd_balance: 5000,
        ..Default::default()
    });
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
