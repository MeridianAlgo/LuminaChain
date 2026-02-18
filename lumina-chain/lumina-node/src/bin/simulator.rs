use lumina_types::state::{GlobalState, AccountState};
use lumina_types::instruction::{StablecoinInstruction, AssetType};
use lumina_types::transaction::Transaction;
use lumina_execution::{execute_transaction, ExecutionContext};
use lumina_crypto::signatures::{generate_keypair, sign};
use std::time::Instant;
use std::sync::{Arc, RwLock};
use ed25519_dalek::Signer;

fn main() {
    println!("--- LuminaChain High-Throughput Simulator ---");
    
    // 1. Setup State
    let mut state = GlobalState::default();
    // Fund a "whale" account to simulate transfers
    let whale_kp = generate_keypair();
    let whale_addr = whale_kp.verifying_key().to_bytes();
    
    state.accounts.insert(whale_addr, AccountState {
        nonce: 0,
        lusd_balance: 1_000_000_000,
        ljun_balance: 1_000_000_000,
        lumina_balance: 1_000_000_000,
        commitment: None,
    });
    
    let state_arc = Arc::new(RwLock::new(state));
    let start_time = Instant::now();
    let num_txs = 10_000;
    
    println!("Generating and executing {} transactions...", num_txs);

    let mut successful_txs = 0;
    let mut failed_txs = 0;

    // Simulate block execution
    let mut state_guard = state_arc.write().unwrap();
    let timestamp = 1678886400; // Mock timestamp
    
    let mut ctx = ExecutionContext {
        state: &mut state_guard,
        height: 1,
        timestamp,
    };

    for i in 0..num_txs {
        // Create a transfer tx
        let recipient = [0u8; 32]; // Burn address for speed
        let instruction = StablecoinInstruction::Transfer {
            to: recipient,
            amount: 1,
            asset: AssetType::LUSD,
        };
        
        // In a real benchmark, we'd sign every tx, but that dominates CPU.
        // We'll sign once and reuse to test EXECUTION throughput, or sign all if we want full realism.
        // Let's sign the payload once to simulate structure.
        let signature = vec![0u8; 64]; // Mock signature for speed in this specific bench

        let tx = Transaction {
            sender: whale_addr,
            nonce: i as u64, // Increment nonce
            instruction,
            signature,
            gas_limit: 1000,
            gas_price: 1,
        };

        match execute_transaction(&tx, &mut ctx) {
            Ok(_) => successful_txs += 1,
            Err(_) => failed_txs += 1,
        }
    }

    let elapsed = start_time.elapsed();
    let tps = num_txs as f64 / elapsed.as_secs_f64();
    
    println!("--- Simulation Complete ---");
    println!("Executed {} transactions in {:.2?}", num_txs, elapsed);
    println!("Throughput: {:.2} TPS", tps);
    println!("Successful: {}", successful_txs);
    println!("Failed: {}", failed_txs);
    println!("Final LUSD Supply: {}", state_guard.total_lusd_supply);
}
