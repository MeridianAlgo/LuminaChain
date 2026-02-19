use super::*;
use lumina_types::instruction::StablecoinInstruction;
use lumina_types::state::{AccountState, GlobalState};
use lumina_types::transaction::Transaction;

fn new_sender() -> ([u8; 32], lumina_crypto::signatures::SigningKey) {
    let kp = lumina_crypto::signatures::generate_keypair();
    (kp.verifying_key().to_bytes(), kp)
}

fn bound_proof(context: [u8; 32], raw_groth16_proof: Vec<u8>) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&context);
    hasher.update(&raw_groth16_proof);
    let tag = *hasher.finalize().as_bytes();

    let mut out = Vec::with_capacity(32 + raw_groth16_proof.len());
    out.extend_from_slice(&tag);
    out.extend_from_slice(&raw_groth16_proof);
    out
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

    let manager = lumina_crypto::zk::ZkManager::setup();
    let mint_si = StablecoinInstruction::MintSenior {
        amount: 1,
        collateral_amount: 1,
        proof: manager.prove_reserves(vec![1], 1),
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
fn test_social_recovery_threshold_and_uniqueness() {
    let mut state = GlobalState::default();
    let sender = [11u8; 32];

    // Create 3 guardian keypairs
    let g1 = lumina_crypto::signatures::generate_keypair();
    let g2 = lumina_crypto::signatures::generate_keypair();
    let g3 = lumina_crypto::signatures::generate_keypair();

    let guardians = vec![
        g1.verifying_key().to_bytes(),
        g2.verifying_key().to_bytes(),
        g3.verifying_key().to_bytes(),
    ];

    // Initialize passkey account with guardians
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let si = StablecoinInstruction::CreatePasskeyAccount {
            device_key: vec![9u8; 65],
            guardians: guardians.clone(),
        };
        execute_si(&si, &sender, &mut ctx).unwrap();
    }

    // Recovery requires threshold 2-of-3 (majority)
    let new_device_key = vec![7u8; 65];
    let sig1 = lumina_crypto::signatures::sign(&g1, &new_device_key);
    let sig2 = lumina_crypto::signatures::sign(&g2, &new_device_key);

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 2,
            timestamp: 200,
        };
        let recover = StablecoinInstruction::RecoverSocial {
            new_device_key: new_device_key.clone(),
            guardian_signatures: vec![sig1.clone(), sig2.clone()],
        };
        execute_si(&recover, &sender, &mut ctx).unwrap();
    }

    assert_eq!(
        state
            .accounts
            .get(&sender)
            .unwrap()
            .passkey_device_key
            .as_ref(),
        Some(&new_device_key)
    );

    // Duplicate guardian signatures should not satisfy threshold
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 3,
            timestamp: 300,
        };
        let recover_dup = StablecoinInstruction::RecoverSocial {
            new_device_key: vec![8u8; 65],
            guardian_signatures: vec![sig1.clone(), sig1],
        };
        assert!(execute_si(&recover_dup, &sender, &mut ctx).is_err());
    }
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

    let manager = lumina_crypto::zk::ZkManager::setup();
    let si = StablecoinInstruction::MintSenior {
        amount: 1000,
        collateral_amount: 1200,
        proof: manager.prove_reserves(vec![1200], 1200),
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

#[test]
fn test_flash_mint_and_flash_burn_same_block() {
    let mut state = GlobalState::default();
    let sender = [12u8; 32];

    // Provide some base collateral so reserve ratio doesn't instantly trip circuit breaker.
    state.stabilization_pool_balance = 1_000_000;
    state.total_lusd_supply = 1_000_000;
    state.reserve_ratio = 1.0;

    // Flash mint
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 10,
            timestamp: 1,
        };
        let si = StablecoinInstruction::FlashMint {
            amount: 1000,
            collateral_asset: lumina_types::instruction::AssetType::Lumina,
            collateral_amount: 1200,
            commitment: [9u8; 32],
        };
        execute_si(&si, &sender, &mut ctx).unwrap();
        assert_eq!(ctx.state.pending_flash_mints, 1000);
        let acct = ctx.state.accounts.get(&sender).unwrap();
        assert_eq!(acct.pending_flash_mint, 1000);
        assert_eq!(acct.pending_flash_collateral, 1200);
    }

    // Must burn full amount
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 10,
            timestamp: 2,
        };
        let burn = StablecoinInstruction::FlashBurn { amount: 1000 };
        execute_si(&burn, &sender, &mut ctx).unwrap();
        assert_eq!(ctx.state.pending_flash_mints, 0);
        let acct = ctx.state.accounts.get(&sender).unwrap();
        assert_eq!(acct.pending_flash_mint, 0);
        assert_eq!(acct.pending_flash_collateral, 0);
    }
}

#[test]
fn test_instant_redeem_queues_under_stress() {
    let mut state = GlobalState::default();
    let sender = [13u8; 32];

    state.accounts.insert(
        sender,
        AccountState {
            lusd_balance: 5000,
            ..Default::default()
        },
    );
    state.total_lusd_supply = 5000;
    state.reserve_ratio = 0.90;
    state.stabilization_pool_balance = 4500;

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 100,
    };
    let redeem = StablecoinInstruction::InstantRedeem {
        amount: 1000,
        destination: [0u8; 32],
    };
    execute_si(&redeem, &sender, &mut ctx).unwrap();
    assert_eq!(ctx.state.fair_redeem_queue.len(), 1);
    assert_eq!(ctx.state.accounts.get(&sender).unwrap().lusd_balance, 4000);
}

#[test]
fn test_mint_with_credit_score_allowlist_and_replay_protection() {
    let mut state = GlobalState::default();
    let sender = [14u8; 32];
    let oracle = [7u8; 32];
    state.trusted_credit_oracles.push(oracle);

    let manager = lumina_crypto::zk::ZkManager::setup();
    let raw = manager.prove_range(500, 1000);
    let proof = bound_proof(*blake3::hash(b"credit-score").as_bytes(), raw);

    // Derive the score the execution will compute so we can set a threshold it passes.
    let score_bytes = blake3::hash(&proof);
    let raw_score = u16::from_le_bytes([score_bytes.as_bytes()[0], score_bytes.as_bytes()[1]]);
    let score = 300 + (raw_score % 551);
    let threshold = score.saturating_sub(1);

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 100,
    };

    let mint = StablecoinInstruction::MintWithCreditScore {
        amount: 1000,
        collateral_amount: 1200,
        credit_score_proof: proof.clone(),
        min_score_threshold: threshold,
        oracle,
    };
    execute_si(&mint, &sender, &mut ctx).unwrap();
    assert_eq!(ctx.state.accounts.get(&sender).unwrap().lusd_balance, 1000);

    // Replaying the same proof should fall back to MintSenior (and thus charge 5% fee).
    let mut ctx2 = ExecutionContext {
        state: &mut state,
        height: 2,
        timestamp: 200,
    };
    execute_si(&mint, &sender, &mut ctx2).unwrap();
    // MintSenior mints net of fee: 1000 - 50 = 950
    assert_eq!(ctx2.state.accounts.get(&sender).unwrap().lusd_balance, 1950);
}

#[test]
fn test_rwa_listing_and_pledge() {
    let mut state = GlobalState::default();
    let sender = [15u8; 32];

    let manager = lumina_crypto::zk::ZkManager::setup();
    let raw = manager.prove_range(1, 10);
    let attested_value = 10_000u64;
    let proof = bound_proof(*blake3::hash(&attested_value.to_le_bytes()).as_bytes(), raw);

    // List RWA
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let list = StablecoinInstruction::ListRWA {
            asset_description: "invoice #123".to_string(),
            attested_value,
            attestation_proof: proof,
            maturity_date: Some(1_000_000),
            collateral_eligibility: true,
        };
        execute_si(&list, &sender, &mut ctx).unwrap();
        assert_eq!(ctx.state.rwa_listings.len(), 1);
        assert_eq!(ctx.state.next_rwa_id, 1);
    }

    // Pledge against it
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 2,
            timestamp: 200,
        };
        let pledge = StablecoinInstruction::UseRWAAsCollateral {
            rwa_id: 0,
            amount_to_pledge: 2500,
        };
        execute_si(&pledge, &sender, &mut ctx).unwrap();
        assert_eq!(ctx.state.accounts.get(&sender).unwrap().lusd_balance, 2500);
        assert_eq!(ctx.state.rwa_listings.get(&0).unwrap().pledged_amount, 2500);
    }
}

#[test]
fn test_submit_zk_por_requires_valid_proof_and_no_replay() {
    let mut state = GlobalState::default();
    let sender = [12u8; 32];
    let manager = lumina_crypto::zk::ZkManager::setup();
    let proof = manager.prove_reserves(vec![40, 60], 100);

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let si = StablecoinInstruction::SubmitZkPoR {
            proof: proof.clone(),
            total_reserves: 100,
            timestamp: 1,
        };
        assert!(execute_si(&si, &sender, &mut ctx).is_ok());
    }

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 2,
            timestamp: 200,
        };
        let replay = StablecoinInstruction::SubmitZkPoR {
            proof: proof.clone(),
            total_reserves: 100,
            timestamp: 2,
        };
        assert!(execute_si(&replay, &sender, &mut ctx).is_err());
    }
}

#[test]
fn test_zero_slip_batch_match_blocks_duplicates_and_replay() {
    let mut state = GlobalState::default();
    let sender = [13u8; 32];

    let orders = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 100,
        };
        let si = StablecoinInstruction::ZeroSlipBatchMatch {
            orders: orders.clone(),
        };
        assert!(execute_si(&si, &sender, &mut ctx).is_ok());
    }

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 2,
            timestamp: 200,
        };
        let replay = StablecoinInstruction::ZeroSlipBatchMatch {
            orders: orders.clone(),
        };
        assert!(execute_si(&replay, &sender, &mut ctx).is_err());
    }

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 3,
            timestamp: 300,
        };
        let dup = StablecoinInstruction::ZeroSlipBatchMatch {
            orders: vec![[9u8; 32], [9u8; 32]],
        };
        assert!(execute_si(&dup, &sender, &mut ctx).is_err());
    }
}
