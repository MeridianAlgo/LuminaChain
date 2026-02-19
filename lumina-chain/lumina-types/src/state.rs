use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Per-account state stored in the global state tree.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AccountState {
    pub nonce: u64,
    pub lusd_balance: u64,
    pub ljun_balance: u64,
    pub lumina_balance: u64,
    pub commitment: Option<[u8; 32]>,
    /// Passkey device key (65 bytes WebAuthn compressed public key)
    pub passkey_device_key: Option<Vec<u8>>,
    /// Social recovery guardians (list of pubkeys)
    pub guardians: Vec<[u8; 32]>,
    /// Post-quantum public key (Dilithium/Falcon), if account has opted in
    pub pq_pubkey: Option<Vec<u8>>,
    /// Cumulative transaction volume for velocity reward calculation (per epoch)
    pub epoch_tx_volume: u64,
    /// Last epoch in which velocity rewards were claimed
    pub last_reward_epoch: u64,
    /// On-chain credit score (0 = unscored, 300..850 mapped to u16)
    pub credit_score: u16,
    /// Active stream payments originated by this account
    pub active_streams: Vec<StreamState>,
    /// Yield token positions
    pub yield_positions: Vec<YieldPosition>,

    pub pending_flash_mint: u64,
    pub pending_flash_collateral: u64,
}

/// Streaming payment state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamState {
    pub recipient: [u8; 32],
    pub amount_per_sec: u64,
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub withdrawn: u64,
}

/// Yield-bearing wrapped token position
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct YieldPosition {
    pub token_id: u64,
    pub principal: u64,
    pub maturity_height: u64,
    pub issued_height: u64,
}

/// Global chain state — the complete state of LuminaChain at any height.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GlobalState {
    pub accounts: HashMap<[u8; 32], AccountState>,
    pub total_lusd_supply: u64,
    pub total_ljun_supply: u64,

    // Stability & Tranches
    pub stabilization_pool_balance: u64,
    pub reserve_ratio: f64,
    pub oracle_prices: HashMap<String, u64>,
    pub validators: Vec<ValidatorState>,

    // Protection
    pub circuit_breaker_active: bool,
    pub fair_redeem_queue: Vec<RedemptionRequest>,
    pub last_rebalance_height: u64,

    // Insurance fund
    pub insurance_fund_balance: u64,

    // Custodian marketplace
    pub custodians: Vec<CustodianState>,
    pub last_reserve_rotation_height: u64,

    // Compliance circuits registry
    pub compliance_circuits: HashMap<u64, Vec<u8>>,

    // RWA registry
    pub rwa_listings: HashMap<u64, RWAListing>,
    pub next_rwa_id: u64,

    // Credit oracle allowlist + proof replay protection
    pub trusted_credit_oracles: Vec<[u8; 32]>,
    pub used_credit_proofs: Vec<[u8; 32]>,

    // Yield token counter
    pub next_yield_token_id: u64,

    // Health index (0..10000 representing 0.00..100.00)
    pub health_index: u64,

    // Flash mint tracking (per-block, reset each block)
    pub pending_flash_mints: u64,

    // Epoch tracking for velocity rewards
    pub current_epoch: u64,
    pub velocity_reward_pool: u64,

    // Proof-of-reserves replay protection and ordering.
    pub last_por_timestamp: u64,
    pub last_por_hash: Option<[u8; 32]>,

    // Replay protection for zero-slip batches.
    pub executed_batch_matches: Vec<[u8; 32]>,
}

impl GlobalState {
    pub fn root_hash(&self) -> [u8; 32] {
        let entries: BTreeMap<[u8; 32], Vec<u8>> = self
            .accounts
            .iter()
            .map(|(k, v)| (*k, bincode::serialize(v).expect("account serialization")))
            .collect();
        account_trie_root(&entries)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum MptNode {
    Leaf {
        path: Vec<u8>,
        value: Vec<u8>,
    },
    Extension {
        path: Vec<u8>,
        child: [u8; 32],
    },
    Branch {
        children: [Option<[u8; 32]>; 16],
        value: Option<Vec<u8>>,
    },
}

fn account_trie_root(entries: &BTreeMap<[u8; 32], Vec<u8>>) -> [u8; 32] {
    let data: Vec<(Vec<u8>, Vec<u8>)> = entries
        .iter()
        .map(|(k, v)| (bytes_to_nibbles(k), v.clone()))
        .collect();
    build_hash(data, Vec::new()).unwrap_or([0u8; 32])
}

fn build_hash(entries: Vec<(Vec<u8>, Vec<u8>)>, prefix: Vec<u8>) -> Option<[u8; 32]> {
    if entries.is_empty() {
        return None;
    }

    if entries.len() == 1 {
        let (k, v) = &entries[0];
        let node = MptNode::Leaf {
            path: k[prefix.len()..].to_vec(),
            value: v.clone(),
        };
        return Some(hash_node(&node));
    }

    let has_exact = entries.iter().any(|(k, _)| k.len() == prefix.len());
    let ext = if has_exact {
        Vec::new()
    } else {
        longest_common_extension(&entries, prefix.len())
    };

    if !ext.is_empty() {
        let child_prefix = [prefix, ext.clone()].concat();
        let child = build_hash(entries, child_prefix)?;
        let node = MptNode::Extension { path: ext, child };
        return Some(hash_node(&node));
    }

    let mut children: [Option<[u8; 32]>; 16] = [None; 16];
    let mut value_at_node = None;

    for (k, v) in &entries {
        if k.len() == prefix.len() {
            value_at_node = Some(v.clone());
            break;
        }
    }

    for nib in 0u8..=15 {
        let subset: Vec<(Vec<u8>, Vec<u8>)> = entries
            .iter()
            .filter(|(k, _)| k.len() > prefix.len() && k[prefix.len()] == nib)
            .cloned()
            .collect();
        children[nib as usize] = build_hash(subset, [prefix.clone(), vec![nib]].concat());
    }

    let node = MptNode::Branch {
        children,
        value: value_at_node,
    };
    Some(hash_node(&node))
}

fn hash_node(node: &MptNode) -> [u8; 32] {
    let encoded = bincode::serialize(node).expect("node serialization");
    *blake3::hash(&encoded).as_bytes()
}

fn longest_common_extension(entries: &[(Vec<u8>, Vec<u8>)], start: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut idx = start;

    loop {
        let Some(first) = entries[0].0.get(idx) else {
            break;
        };
        if entries.iter().all(|(k, _)| k.get(idx) == Some(first)) {
            out.push(*first);
            idx += 1;
        } else {
            break;
        }
    }

    out
}

fn bytes_to_nibbles(bytes: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    for b in bytes {
        out.push((b >> 4) & 0x0F);
        out.push(b & 0x0F);
    }
    out
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RedemptionRequest {
    pub address: [u8; 32],
    pub amount: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidatorState {
    pub pubkey: [u8; 32],
    pub stake: u64,
    pub power: u64,
    pub is_green: bool,
    pub energy_proof: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustodianState {
    pub pubkey: [u8; 32],
    pub stake: u64,
    pub mpc_pubkeys: Vec<[u8; 32]>,
    pub registered_height: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RWAListing {
    pub owner: [u8; 32],
    pub asset_description: String,
    pub attestation_proof: Vec<u8>,
    pub attested_value: u64,
    pub maturity_date: Option<u64>,
    pub collateral_eligibility: bool,
    pub is_active: bool,
    pub pledged_amount: u64,
}
