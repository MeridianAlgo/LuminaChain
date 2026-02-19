use anyhow::Result;
use clap::Parser;
use lumina_crypto::signatures::{generate_keypair, sign, SigningKey};
use lumina_crypto::zk::ZkManager;
use lumina_execution::{execute_transaction, ExecutionContext};
use lumina_types::instruction::{AssetType, StablecoinInstruction};
use lumina_types::state::{AccountState, GlobalState};
use lumina_types::transaction::Transaction;
use rand::Rng;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about = "LuminaChain realistic simulation runner")]
struct Args {
    /// Number of wallets created for the simulation.
    #[arg(long, default_value_t = 200)]
    wallets: usize,
    /// Number of transfer transactions to execute.
    #[arg(long, default_value_t = 20_000)]
    transfers: usize,
    /// Starting simulated money airdropped to each wallet.
    #[arg(long, default_value_t = 50_000)]
    simulation_money: u64,
}

#[derive(Clone)]
struct SimWallet {
    keypair: SigningKey,
    address: [u8; 32],
}

fn build_wallets(wallet_count: usize) -> Vec<SimWallet> {
    (0..wallet_count)
        .map(|_| {
            let kp = generate_keypair();
            let address = kp.verifying_key().to_bytes();
            SimWallet {
                keypair: kp,
                address,
            }
        })
        .collect()
}

fn seed_simulation_money(state: &mut GlobalState, wallets: &[SimWallet], amount: u64) {
    for wallet in wallets {
        state.accounts.insert(
            wallet.address,
            AccountState {
                lusd_balance: amount,
                ..Default::default()
            },
        );
        state.total_lusd_supply = state.total_lusd_supply.saturating_add(amount);
    }

    state.stabilization_pool_balance = state.total_lusd_supply.saturating_mul(125) / 100;
    state.reserve_ratio = 1.25;
}

fn build_transfer_tx(
    sender: &SimWallet,
    receiver: [u8; 32],
    amount: u64,
    nonce: u64,
) -> Transaction {
    let instruction = StablecoinInstruction::Transfer {
        to: receiver,
        amount,
        asset: AssetType::LUSD,
    };

    let mut tx = Transaction {
        sender: sender.address,
        nonce,
        instruction,
        signature: vec![],
        gas_limit: 1000,
        gas_price: 1,
    };

    tx.signature = sign(&sender.keypair, &tx.signing_bytes());
    tx
}

fn build_mint_tx(sender: &SimWallet, nonce: u64, amount: u64) -> Transaction {
    let zk = ZkManager::setup();
    let collateral = amount.saturating_mul(120) / 100;
    let proof = zk.prove_reserves(vec![collateral], collateral);

    let instruction = StablecoinInstruction::MintSenior {
        amount,
        collateral_amount: collateral,
        proof,
    };

    let mut tx = Transaction {
        sender: sender.address,
        nonce,
        instruction,
        signature: vec![],
        gas_limit: 100_000,
        gas_price: 1,
    };
    tx.signature = sign(&sender.keypair, &tx.signing_bytes());
    tx
}

fn run_simulation(args: &Args) -> Result<()> {
    let mut state = GlobalState::default();
    let wallets = build_wallets(args.wallets);
    seed_simulation_money(&mut state, &wallets, args.simulation_money);

    let mut nonce_book = std::collections::HashMap::<[u8; 32], u64>::new();
    for wallet in &wallets {
        nonce_book.insert(wallet.address, 0);
    }

    let start = Instant::now();
    let mut rng = rand::thread_rng();

    // First, mint from wallet[0] to prove mint path + PoR enforcement works in simulation.
    let minter = &wallets[0];
    let nonce = *nonce_book.get(&minter.address).unwrap_or(&0);
    let mint_tx = build_mint_tx(minter, nonce, 100_000);

    {
        let mut ctx = ExecutionContext {
            state: &mut state,
            height: 1,
            timestamp: 1_700_000_000,
        };
        execute_transaction(&mint_tx, &mut ctx)?;
    }
    nonce_book.insert(minter.address, nonce.saturating_add(1));

    let mut success = 0u64;
    let mut failed = 0u64;

    for i in 0..args.transfers {
        let sender_idx = rng.gen_range(0..wallets.len());
        let mut receiver_idx = rng.gen_range(0..wallets.len());
        if receiver_idx == sender_idx {
            receiver_idx = (receiver_idx + 1) % wallets.len();
        }

        let sender = &wallets[sender_idx];
        let receiver = wallets[receiver_idx].address;
        let nonce = *nonce_book.get(&sender.address).unwrap_or(&0);

        let tx = build_transfer_tx(sender, receiver, 1, nonce);
        let result = {
            let mut ctx = ExecutionContext {
                state: &mut state,
                height: 2 + i as u64,
                timestamp: 1_700_000_100 + i as u64,
            };
            execute_transaction(&tx, &mut ctx)
        };

        match result {
            Ok(_) => {
                success = success.saturating_add(1);
                nonce_book.insert(sender.address, nonce.saturating_add(1));
            }
            Err(_) => {
                failed = failed.saturating_add(1);
            }
        }
    }

    let elapsed = start.elapsed();
    let tps = if elapsed.as_secs_f64() > 0.0 {
        args.transfers as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };

    println!("=== Lumina Simulation (separate module, real execution algo) ===");
    println!("Wallets created: {}", wallets.len());
    println!(
        "Simulation money per wallet: {} LUSD",
        args.simulation_money
    );
    println!("Transfers attempted: {}", args.transfers);
    println!("Successful transfers: {}", success);
    println!("Failed transfers: {}", failed);
    println!("Elapsed: {:.2?}", elapsed);
    println!("TPS: {:.2}", tps);
    println!("Total supply: {}", state.total_lusd_supply);
    println!("Reserve ratio: {:.4}", state.reserve_ratio);
    println!("Insurance fund: {}", state.insurance_fund_balance);

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    run_simulation(&args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_bootstraps_wallets_and_money() {
        let mut state = GlobalState::default();
        let wallets = build_wallets(10);
        seed_simulation_money(&mut state, &wallets, 1_000);

        assert_eq!(wallets.len(), 10);
        assert_eq!(state.accounts.len(), 10);
        assert_eq!(state.total_lusd_supply, 10_000);
        assert!(state.reserve_ratio >= 1.0);
    }
}
