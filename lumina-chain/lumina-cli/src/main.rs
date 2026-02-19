use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use lumina_crypto::signatures::{generate_keypair, sign};
use lumina_crypto::zk::ZkManager;
use lumina_types::instruction::{AssetType, StablecoinInstruction};
use lumina_types::transaction::Transaction;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "LuminaChain CLI — Interact with the LuminaChain network"
)]
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
    /// Mint stablecoin (Testnet)
    Mint {
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        asset: String,
    },
    /// Transfer tokens
    Transfer {
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        asset: String,
    },
    /// Get account balance
    Balance {
        #[arg(long)]
        address: String,
    },
    /// Get block info
    Block {
        #[arg(long)]
        height: u64,
    },
    /// Query the Lumina Health Index
    Health,
    /// Query insurance fund
    Insurance,
    /// Query validators
    Validators,
}

#[derive(Serialize, Deserialize)]
struct Wallet {
    secret_key: String,
    public_key: String,
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
                "senior" | "lusd" => {
                    let collateral = amount.saturating_mul(120) / 100;
                    let zk = ZkManager::setup();
                    StablecoinInstruction::MintSenior {
                        amount: *amount,
                        collateral_amount: collateral,
                        proof: zk.prove_reserves(vec![collateral], collateral),
                    }
                }
                "junior" | "ljun" => StablecoinInstruction::MintJunior {
                    amount: *amount,
                    collateral_amount: amount.saturating_mul(120) / 100,
                },
                _ => {
                    return Err(anyhow!(
                        "Invalid asset type. Use: senior/lusd or junior/ljun"
                    ))
                }
            };

            let mut tx = Transaction {
                sender,
                nonce: 0,
                instruction,
                signature: vec![],
                gas_limit: 100_000,
                gas_price: 1,
            };

            tx.signature = sign(&kp, &tx.signing_bytes());

            let res = client
                .post(format!("{}/tx", cli.node_url))
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
                "lumina" => AssetType::Lumina,
                _ => return Err(anyhow!("Invalid asset. Use: lusd, ljun, or lumina")),
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
                gas_limit: 100_000,
                gas_price: 1,
            };

            tx.signature = sign(&kp, &tx.signing_bytes());

            let res = client
                .post(format!("{}/tx", cli.node_url))
                .json(&tx)
                .send()
                .await?;

            println!("Response: {}", res.text().await?);
        }
        Commands::Balance { address } => {
            let res = client
                .get(format!("{}/account/{}", cli.node_url, address))
                .send()
                .await?
                .text()
                .await?;

            println!("Account Info:\n{}", res);
        }
        Commands::Block { height } => {
            let res = client
                .get(format!("{}/block/{}", cli.node_url, height))
                .send()
                .await?;

            if res.status().is_success() {
                println!("Block Info: {}", res.text().await?);
            } else {
                println!("Block not found");
            }
        }
        Commands::Health => {
            let res = client
                .get(format!("{}/health", cli.node_url))
                .send()
                .await?
                .text()
                .await?;

            println!("Lumina Health Index:\n{}", res);
        }
        Commands::Insurance => {
            let res = client
                .get(format!("{}/insurance", cli.node_url))
                .send()
                .await?
                .text()
                .await?;

            println!("Insurance Fund:\n{}", res);
        }
        Commands::Validators => {
            let res = client
                .get(format!("{}/validators", cli.node_url))
                .send()
                .await?
                .text()
                .await?;

            println!("Validators:\n{}", res);
        }
    }

    Ok(())
}
