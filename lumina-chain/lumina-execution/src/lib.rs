use lumina_types::instruction::{StablecoinInstruction, AssetType};
use lumina_types::state::{GlobalState, AccountState, RedemptionRequest};
use lumina_types::transaction::Transaction;
use anyhow::{Result, bail};

pub struct ExecutionContext<'a> {
    pub state: &'a mut GlobalState,
    pub height: u64,
    pub timestamp: u64,
}

pub fn execute_transaction(tx: &Transaction, ctx: &mut ExecutionContext) -> Result<()> {
    // 1. Verify Signature (simplified for Phase 3)
    // verify_signature(&tx)?;

    // 2. Check Nonce
    let sender_account = ctx.state.accounts.entry(tx.sender).or_default();
    if tx.nonce != sender_account.nonce {
         if tx.nonce != 0 || sender_account.nonce != 0 {
            bail!("Invalid nonce: expected {}, got {}", sender_account.nonce, tx.nonce);
         }
    }
    sender_account.nonce += 1;

    // 3. Execute Instruction
    execute_si(&tx.instruction, &tx.sender, ctx)
}

pub fn execute_si(si: &StablecoinInstruction, sender: &[u8; 32], ctx: &mut ExecutionContext) -> Result<()> {
    match si {
        StablecoinInstruction::MintSenior { amount, .. } => {
            if ctx.state.circuit_breaker_active { bail!("Mints paused"); }
            let account = ctx.state.accounts.entry(*sender).or_default();
            account.lusd_balance += amount;
            ctx.state.total_lusd_supply += amount;
            recalculate_ratios(ctx);
            Ok(())
        },
        StablecoinInstruction::RedeemSenior { amount } => {
            let account = ctx.state.accounts.entry(*sender).or_default();
            if account.lusd_balance < *amount { bail!("Insufficient LUSD"); }

            if ctx.state.circuit_breaker_active || ctx.state.reserve_ratio < 0.95 {
                 ctx.state.fair_redeem_queue.push(RedemptionRequest {
                     address: *sender,
                     amount: *amount,
                     timestamp: ctx.timestamp,
                 });
                 account.lusd_balance -= amount;
                 return Ok(());
            }

            account.lusd_balance -= amount;
            ctx.state.total_lusd_supply -= amount;
            recalculate_ratios(ctx);
            Ok(())
        },
        StablecoinInstruction::TriggerStabilizer => {
            if ctx.state.reserve_ratio < 1.0 && ctx.state.stabilization_pool_balance > 0 {
                let deficit = (ctx.state.total_lusd_supply as f64 * (1.0 - ctx.state.reserve_ratio)) as u64;
                let amount_to_move = std::cmp::min(deficit, ctx.state.stabilization_pool_balance);
                ctx.state.stabilization_pool_balance -= amount_to_move;
                ctx.state.reserve_ratio += (amount_to_move as f64 / ctx.state.total_lusd_supply as f64);
            }
            Ok(())
        },
        StablecoinInstruction::ProcessRedemptionQueue { batch_size } => {
            if ctx.state.circuit_breaker_active { bail!("Paused"); }
            let to_process = std::cmp::min(*batch_size as usize, ctx.state.fair_redeem_queue.len());
            for _ in 0..to_process {
                let req = ctx.state.fair_redeem_queue.remove(0);
                ctx.state.total_lusd_supply -= req.amount;
            }
            recalculate_ratios(ctx);
            Ok(())
        },
        StablecoinInstruction::Transfer { to, amount, asset } => {
             let sender_account = ctx.state.accounts.entry(*sender).or_default();
             match asset {
                 AssetType::LUSD => {
                     if sender_account.lusd_balance < *amount { bail!("Insufficient LUSD"); }
                     sender_account.lusd_balance -= amount;
                     let receiver = ctx.state.accounts.entry(*to).or_default();
                     receiver.lusd_balance += amount;
                 },
                 AssetType::LJUN => {
                     if sender_account.ljun_balance < *amount { bail!("Insufficient LJUN"); }
                     sender_account.ljun_balance -= amount;
                     let receiver = ctx.state.accounts.entry(*to).or_default();
                     receiver.ljun_balance += amount;
                 },
                 AssetType::Lumina(val) => {
                     if sender_account.lumina_balance < *val { bail!("Insufficient Lumina"); }
                     sender_account.lumina_balance -= val;
                     let receiver = ctx.state.accounts.entry(*to).or_default();
                     receiver.lumina_balance += val;
                 }
             }
             Ok(())
        },
        // === Phase 3: Confidential Transfer ===
        StablecoinInstruction::ConfidentialTransfer { commitment, proof: _ } => {
             // 1. Verify ZK Range Proof (Mocked)
             // verify_confidential_proof(commitment, proof)?;

             // 2. Update state commitments
             let account = ctx.state.accounts.entry(*sender).or_default();
             account.commitment = Some(*commitment);
             // In a real UTXO/Account privacy model, we'd update multiple commitments
             Ok(())
        },
        StablecoinInstruction::UpdateOracle { asset, price, .. } => {
            ctx.state.oracle_prices.insert(asset.clone(), *price);
            recalculate_ratios(ctx);
            Ok(())
        },
        _ => Ok(())
    }
}

fn recalculate_ratios(ctx: &mut ExecutionContext) {
    if ctx.state.total_lusd_supply == 0 {
        ctx.state.reserve_ratio = 1.0;
        return;
    }
    let eth_price = ctx.state.oracle_prices.get("ETH-USD").unwrap_or(&3000_000_000);
    ctx.state.reserve_ratio = (*eth_price as f64 / 3000_000_000.0);
    if ctx.state.reserve_ratio < 0.85 {
        ctx.state.circuit_breaker_active = true;
    }
}
