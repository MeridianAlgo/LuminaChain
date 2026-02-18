use clap::{Parser, Subcommand};
use anyhow::{Result, anyhow};
use lumina_types::transaction::Transaction;
use lumina_types::instruction::{StablecoinInstruction, AssetType};
use lumina_crypto::signatures::{generate_keypair, sign};
use reqwest::Client;
use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, default_value = "http://localhost:3000")]
    node_url: String,
    #[arg(short, long, default_value = "wallet.json")]
    wallet_path: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new wallet
    Init,
    /// Show current wallet info
    Show,
    /// Mint stablecoin (Testnet only)
    Mint {
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        asset: String, // senior/junior
    },
    /// Transfer tokens
    Transfer {
        #[arg(long)]
        to: String, // hex
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        asset: String, // lusd/ljun
    },
    /// Get account balance
    Balance {
        #[arg(long)]
        address: String, // hex
    },
    /// Get block info
    Block {
        #[arg(long)]
        height: u64,
    },
}

#[derive(Serialize, Deserialize)]
struct Wallet {
    secret_key: String, // hex
    public_key: String, // hex
}

impl Wallet {
    fn load(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn to_keypair(&self) -> Result<SigningKey> {
        let secret = hex::decode(&self.secret_key)?;
        Ok(SigningKey::from_bytes(secret.as_slice().try_into()?))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new();

    match &cli.command {
        Commands::Init => {
            let kp = generate_keypair();
            let wallet = Wallet {
                secret_key: hex::encode(kp.to_bytes()),
                public_key: hex::encode(kp.verifying_key().as_bytes()),
            };
            wallet.save(&cli.wallet_path)?;
            println!("Wallet initialized at {:?}", cli.wallet_path);
            println!("Public Key: {}", wallet.public_key);
        }
        Commands::Show => {
            let wallet = Wallet::load(&cli.wallet_path)?;
            println!("Wallet: {:?}", cli.wallet_path);
            println!("Public Key: {}", wallet.public_key);
        }
        Commands::Mint { amount, asset } => {
            let wallet = Wallet::load(&cli.wallet_path)?;
            let kp = wallet.to_keypair()?;
            let sender = kp.verifying_key().to_bytes();

            let instruction = match asset.to_lowercase().as_str() {
                "senior" | "lusd" => StablecoinInstruction::MintSenior {
                    amount: *amount,
                    collateral_amount: 0,
                    proof: vec![],
                },
                "junior" | "ljun" => StablecoinInstruction::MintJunior {
                    amount: *amount,
                    collateral_amount: 0,
                },
                _ => return Err(anyhow!("Invalid asset type")),
            };

            // Fetch current nonce from node (Phase 3 enhancement)
            // Simplified: use 0 or let node handle (mempool logic)
            let nonce = 0; 

            let mut tx = Transaction {
                sender,
                nonce,
                instruction,
                signature: vec![],
                gas_limit: 100000,
                gas_price: 1,
            };

            tx.signature = sign(&kp, &bincode::serialize(&tx.instruction).unwrap());

            let res = client.post(format!("{}/tx", cli.node_url))
                .json(&tx)
                .send()
                .await?;
            
            println!("Response: {}", res.text().await?);
        }
        Commands::Transfer { to, amount, asset } => {
            let wallet = Wallet::load(&cli.wallet_path)?;
            let kp = wallet.to_keypair()?;
            let sender = kp.verifying_key().to_bytes();

            let mut to_bytes = [0u8; 32];
            hex::decode_to_slice(to.trim_start_matches("0x"), &mut to_bytes)?;

            let asset_type = match asset.to_lowercase().as_str() {
                "lusd" => AssetType::LUSD,
                "ljun" => AssetType::LJUN,
                "lumina" => AssetType::Lumina(*amount),
                _ => return Err(anyhow!("Invalid asset type")),
            };

            let instruction = StablecoinInstruction::Transfer {
                to: to_bytes,
                amount: *amount,
                asset: asset_type,
            };

            let mut tx = Transaction {
                sender,
                nonce: 0,
                instruction,
                signature: vec![],
                gas_limit: 100000,
                gas_price: 1,
            };

            tx.signature = sign(&kp, &bincode::serialize(&tx.instruction).unwrap());

            let res = client.post(format!("{}/tx", cli.node_url))
                .json(&tx)
                .send()
                .await?;
            
            println!("Response: {}", res.text().await?);
        }
        Commands::Balance { address: _ } => {
            let res = client.get(format!("{}/state", cli.node_url))
                .send()
                .await?
                .json::<lumina_types::state::GlobalState>()
                .await?;
            
            println!("--- LuminaChain Global State ---");
            println!("Total LUSD Supply: {}", res.total_lusd_supply);
            println!("Total LJUN Supply: {}", res.total_ljun_supply);
            println!("Reserve Ratio: {:.2}", res.reserve_ratio);
            println!("Stabilization Pool: {}", res.stabilization_pool_balance);
            println!("Circuit Breaker: {}", res.circuit_breaker_active);
        }
        Commands::Block { height } => {
            let res = client.get(format!("{}/block/{}", cli.node_url, height))
                .send()
                .await?;
            
            if res.status().is_success() {
                 println!("Block Info: {}", res.text().await?);
            } else {
                 println!("Block not found");
            }
        }
    }

    Ok(())
}
