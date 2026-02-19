use lumina_crypto::signatures::{generate_keypair, sign};
use lumina_execution::{execute_transaction, ExecutionContext};
use lumina_types::instruction::{AssetType, StablecoinInstruction};
use lumina_types::state::{AccountState, GlobalState};
use lumina_types::transaction::Transaction;
use std::time::Instant;

fn main() {
    println!("=== LuminaChain High-Throughput Simulator ===");
    println!();

    // 1. Setup State
    let mut state = GlobalState::default();
    let whale_kp = generate_keypair();
    let whale_addr = whale_kp.verifying_key().to_bytes();

    state.accounts.insert(
        whale_addr,
        AccountState {
            nonce: 0,
            lusd_balance: 1_000_000_000,
            ljun_balance: 1_000_000_000,
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
        },
    );

    state.total_lusd_supply = 1_000_000_000;
    state.stabilization_pool_balance = 1_200_000_000;
    state.reserve_ratio = 1.2;

    let start_time = Instant::now();
    let num_txs: usize = 10_000;

    println!("Generating and executing {} transactions...", num_txs);

    let mut successful_txs: u64 = 0;
    let mut failed_txs: u64 = 0;

    let timestamp = 1678886400u64;

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp,
    };

    for i in 0..num_txs {
        let recipient = [0u8; 32];
        let instruction = StablecoinInstruction::Transfer {
            to: recipient,
            amount: 1,
            asset: AssetType::LUSD,
        };

        // Sign every transaction for realistic benchmarking
        let mut tx = Transaction {
            sender: whale_addr,
            nonce: i as u64,
            instruction,
            signature: vec![],
            gas_limit: 1000,
            gas_price: 1,
        };

        tx.signature = sign(&whale_kp, &tx.signing_bytes());

        match execute_transaction(&tx, &mut ctx) {
            Ok(_) => successful_txs = successful_txs.saturating_add(1),
            Err(_) => failed_txs = failed_txs.saturating_add(1),
        }
    }

    let elapsed = start_time.elapsed();
    let tps = num_txs as f64 / elapsed.as_secs_f64();

    println!();
    println!("=== Simulation Complete ===");
    println!("Executed {} transactions in {:.2?}", num_txs, elapsed);
    println!("Throughput: {:.2} TPS", tps);
    println!("Successful: {}", successful_txs);
    println!("Failed: {}", failed_txs);
    println!("Final LUSD Supply: {}", ctx.state.total_lusd_supply);
    println!("Final Reserve Ratio: {:.4}", ctx.state.reserve_ratio);
    println!("Insurance Fund: {}", ctx.state.insurance_fund_balance);
}
