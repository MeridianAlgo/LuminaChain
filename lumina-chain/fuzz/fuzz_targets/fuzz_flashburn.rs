#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use lumina_execution::{execute_si, ExecutionContext};
use lumina_types::instruction::StablecoinInstruction;
use lumina_types::state::GlobalState;

#[derive(Arbitrary, Debug)]
struct FlashBurnInput {
    sender: [u8; 32],
    amount: u64,
}

fuzz_target!(|data: FlashBurnInput| {
    let mut state = GlobalState::default();

    // Seed a pending flash mint so burn can do real work.
    {
        let acct = state.accounts.entry(data.sender).or_default();
        acct.pending_flash_mint = (data.amount % 1_000_000).saturating_add(1);
        acct.pending_flash_collateral = 2_000_000;
        acct.lusd_balance = acct.pending_flash_mint;
    }

    state.total_lusd_supply = state.total_lusd_supply.saturating_add(10_000_000);
    state.stabilization_pool_balance = state.stabilization_pool_balance.saturating_add(10_000_000);
    state.reserve_ratio = 1.0;

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 1,
    };

    let si = StablecoinInstruction::FlashBurn {
        amount: (data.amount % 1_000_000).saturating_add(1),
    };

    let _ = execute_si(&si, &data.sender, &mut ctx);
});
