use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum StablecoinInstruction {
    // === Core Stablecoin Operations ===
    /// Mint senior stablecoin (LUSD) with collateral (e.g. USDC bridged)
    MintSenior {
        amount: u64,
        collateral_amount: u64,
        proof: Vec<u8>, // zk-proof of valid collateral
    },
    /// Redeem senior stablecoin for underlying collateral (Enters the FairRedeemQueue)
    RedeemSenior {
        amount: u64,
    },
    /// Mint junior token (LJUN) for yield + risk absorption
    MintJunior {
        amount: u64,
        collateral_amount: u64,
    },
    /// Redeem junior token
    RedeemJunior {
        amount: u64,
    },

    // === Oracle & Reserves ===
    /// Update oracle price feed (multi-sig or weighted median)
    UpdateOracle {
        asset: String,
        price: u64,
        timestamp: u64,
        signature: Vec<u8>,
    },
    /// Submit Zero-Knowledge Proof of Reserves
    SubmitZkPoR {
        proof: Vec<u8>,
        total_reserves: u64,
        timestamp: u64,
    },

    // === Stability Control (Phase 2) ===
    /// Trigger stabilization rebalance (Senior/Junior collateral shifts)
    TriggerStabilizer,
    /// Manually trigger or automatically trip circuit breakers
    RunCircuitBreaker {
        active: bool,
    },
    /// Process next batch of redemptions from the queue
    ProcessRedemptionQueue {
        batch_size: u32,
    },

    // === Transfers & Privacy ===
    /// Standard transparent transfer
    Transfer {
        to: [u8; 32],
        amount: u64,
        asset: AssetType,
    },
    /// Confidential transfer (Bulletproofs / ZK)
    ConfidentialTransfer {
        commitment: [u8; 32],
        proof: Vec<u8>,
    },

    // === Governance & Staking ===
    /// Register a new validator
    RegisterValidator {
        pubkey: [u8; 32],
        stake: u64,
    },
    /// Vote on a proposal
    Vote {
        proposal_id: u64,
        vote: bool,
    },
    
    // === Compliance ===
    /// Compliance check (Travel Rule)
    ComplianceCheck {
        tx_hash: [u8; 32],
        compliance_proof: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AssetType {
    LUSD,
    LJUN,
    Lumina (u64), // Native gas token
}
