#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use lumina_execution::{execute_si, ExecutionContext};
use lumina_types::instruction::{AssetType, StablecoinInstruction};
use lumina_types::state::GlobalState;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    sender: [u8; 32],
    to: [u8; 32],
    amount: u64,
    collateral_amount: u64,
    rwa_id: u64,
    amount_to_pledge: u64,
    commitment: [u8; 32],
    use_custom_asset: bool,
    custom_asset_len: u8,
}

fn asset_from(input: &FuzzInput) -> AssetType {
    if input.use_custom_asset {
        let n = (input.custom_asset_len % 16) as usize;
        let s: String = std::iter::repeat('A').take(n).collect();
        AssetType::Custom(s)
    } else {
        AssetType::LUSD
    }
}

fuzz_target!(|data: FuzzInput| {
    let mut state = GlobalState::default();

    // Seed sender account so some paths can progress.
    {
        let acct = state.accounts.entry(data.sender).or_default();
        acct.lusd_balance = data.amount.saturating_add(1);
        acct.ljun_balance = data.amount.saturating_add(1);
        acct.lumina_balance = data.amount.saturating_add(1);
        acct.nonce = 0;
    }

    // Some global context to avoid degenerate / NaN behavior.
    state.total_lusd_supply = state.total_lusd_supply.saturating_add(1);
    state.stabilization_pool_balance = state.stabilization_pool_balance.saturating_add(1);
    state.reserve_ratio = 1.0;
    state.oracle_prices.insert("LUSD-USD".to_string(), 1_000_000);

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 1,
    };

    // Choose from a small subset of instructions that don't require heavy ZK proof payloads.
    let si = match data.amount % 6 {
        0 => StablecoinInstruction::Transfer {
            to: data.to,
            amount: data.amount % 1_000_000,
            asset: asset_from(&data),
        },
        1 => StablecoinInstruction::Burn {
            amount: data.amount % 1_000_000,
            asset: asset_from(&data),
        },
        2 => StablecoinInstruction::FlashMint {
            amount: (data.amount % 1_000_000).saturating_add(1),
            collateral_asset: AssetType::LUSD,
            collateral_amount: (data.collateral_amount % 2_000_000).saturating_add(1),
            commitment: data.commitment,
        },
        3 => StablecoinInstruction::FlashBurn {
            amount: (data.amount % 1_000_000).saturating_add(1),
        },
        4 => StablecoinInstruction::InstantRedeem {
            amount: (data.amount % 1_000_000).saturating_add(1),
            destination: data.to,
        },
        _ => StablecoinInstruction::UseRWAAsCollateral {
            rwa_id: data.rwa_id,
            amount_to_pledge: data.amount_to_pledge,
        },
    };

    let _ = execute_si(&si, &data.sender, &mut ctx);
});
