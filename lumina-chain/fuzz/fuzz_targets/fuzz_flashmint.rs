#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use lumina_execution::{execute_si, ExecutionContext};
use lumina_types::instruction::{AssetType, StablecoinInstruction};
use lumina_types::state::GlobalState;

#[derive(Arbitrary, Debug)]
struct FlashMintInput {
    sender: [u8; 32],
    amount: u64,
    collateral_amount: u64,
    commitment: [u8; 32],
}

fuzz_target!(|data: FlashMintInput| {
    let mut state = GlobalState::default();
    state.oracle_prices.insert("LUSD-USD".to_string(), 1_000_000);
    state.reserve_ratio = 1.0;

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 1,
    };

    let si = StablecoinInstruction::FlashMint {
        amount: (data.amount % 1_000_000).saturating_add(1),
        collateral_asset: AssetType::LUSD,
        collateral_amount: (data.collateral_amount % 2_000_000).saturating_add(1),
        commitment: data.commitment,
    };

    let _ = execute_si(&si, &data.sender, &mut ctx);
});
