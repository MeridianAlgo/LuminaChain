use serde::{Serialize, Deserialize};

pub type ZkProof = Vec<u8>;

/// All 40+ native StablecoinInstructions for LuminaChain.
/// Each variant is a first-class on-chain operation with zero VM overhead.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum StablecoinInstruction {
    // ══════════════════════════════════════════════════════════════
    // Core Asset Operations
    // ══════════════════════════════════════════════════════════════
    RegisterAsset { ticker: String, decimals: u8 },
    MintSenior { amount: u64, collateral_amount: u64, proof: Vec<u8> },
    RedeemSenior { amount: u64 },
    MintJunior { amount: u64, collateral_amount: u64 },
    RedeemJunior { amount: u64 },
    Burn { amount: u64, asset: AssetType },
    Transfer { to: [u8; 32], amount: u64, asset: AssetType },

    // ══════════════════════════════════════════════════════════════
    // Stability & Tranche Management
    // ══════════════════════════════════════════════════════════════
    RebalanceTranches,
    DistributeYield { total_yield: u64 },
    TriggerStabilizer,
    RunCircuitBreaker { active: bool },
    FairRedeemQueue { batch_size: u32 },

    // ══════════════════════════════════════════════════════════════
    // Privacy & Compliance
    // ══════════════════════════════════════════════════════════════
    ConfidentialTransfer { commitment: [u8; 32], proof: Vec<u8> },
    ProveCompliance { tx_hash: [u8; 32], proof: Vec<u8> },
    ZkTaxAttest { period: u64, proof: Vec<u8> },
    MultiJurisdictionalCheck { jurisdiction_id: u32, proof: Vec<u8> },

    // ══════════════════════════════════════════════════════════════
    // Oracle & Reserves
    // ══════════════════════════════════════════════════════════════
    UpdateOracle { asset: String, price: u64, timestamp: u64, signature: Vec<u8> },
    SubmitZkPoR { proof: Vec<u8>, total_reserves: u64, timestamp: u64 },

    // ══════════════════════════════════════════════════════════════
    // Advanced DeFi & Fiat Hooks
    // ══════════════════════════════════════════════════════════════
    InstantFiatBridge { amount: u64, target_bank_id: [u8; 16], mpc_sig: Vec<u8> },
    ZeroSlipBatchMatch { orders: Vec<[u8; 32]> },
    DynamicHedge { ratio_bps: u64 },
    GeoRebalance { zone_id: u32 },
    VelocityIncentive { multiplier_bps: u64 },
    StreamPayment { to: [u8; 32], amount_per_sec: u64, duration: u64 },

    // ══════════════════════════════════════════════════════════════
    // Governance & Staking
    // ══════════════════════════════════════════════════════════════
    RegisterValidator { pubkey: [u8; 32], stake: u64 },
    Vote { proposal_id: u64, approve: bool },

    // ══════════════════════════════════════════════════════════════
    // Phase 1 Differentiators: Seedless Security & Dynamic Economics
    // ══════════════════════════════════════════════════════════════
    CreatePasskeyAccount { device_key: Vec<u8>, guardians: Vec<[u8; 32]> },
    RecoverSocial { new_device_key: Vec<u8>, guardian_signatures: Vec<Vec<u8>> },
    ClaimVelocityReward { epoch: u64, tx_volume: u64 },
    RegisterCustodian { stake: u64, mpc_pubkeys: Vec<[u8; 32]> },
    RotateReserves { new_custodian_set: Vec<[u8; 32]> },
    ClaimInsurance { loss_proof: Vec<u8>, claimed_amount: u64 },

    // ══════════════════════════════════════════════════════════════
    // Phase 2 Differentiators: Security & Compliance Excellence
    // ══════════════════════════════════════════════════════════════
    SwitchToPQSignature { new_pq_pubkey: Vec<u8> },
    RegisterGreenValidator { energy_proof: Vec<u8> },
    UploadComplianceCircuit { circuit_id: u64, verifier_key: Vec<u8> },

    // ══════════════════════════════════════════════════════════════
    // Phase 3 Differentiators: Capital Efficiency & RWA
    // ══════════════════════════════════════════════════════════════
    FlashMint {
        amount: u64,
        collateral_asset: AssetType,
        collateral_amount: u64,
        commitment: [u8; 32],
    },
    FlashBurn { amount: u64 },
    InstantRedeem { amount: u64, destination: [u8; 32] },
    MintWithCreditScore {
        amount: u64,
        collateral_amount: u64,
        credit_score_proof: ZkProof,
        min_score_threshold: u16,
        oracle: [u8; 32],
    },
    WrapToYieldToken { amount: u64, maturity_blocks: u64 },
    UnwrapYieldToken { token_id: u64 },
    ListRWA {
        asset_description: String,
        attested_value: u64,
        attestation_proof: ZkProof,
        maturity_date: Option<u64>,
        collateral_eligibility: bool,
    },
    UseRWAAsCollateral {
        rwa_id: u64,
        amount_to_pledge: u64,
    },
    ComputeHealthIndex,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AssetType {
    LUSD,
    LJUN,
    Lumina,
    Custom(String),
}
