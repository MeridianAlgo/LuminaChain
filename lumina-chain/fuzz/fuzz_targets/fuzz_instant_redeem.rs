#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use lumina_execution::{execute_si, ExecutionContext};
use lumina_types::instruction::StablecoinInstruction;
use lumina_types::state::GlobalState;

#[derive(Arbitrary, Debug)]
struct RedeemInput {
    sender: [u8; 32],
    destination: [u8; 32],
    amount: u64,
}

fuzz_target!(|data: RedeemInput| {
    let mut state = GlobalState::default();

    {
        let acct = state.accounts.entry(data.sender).or_default();
        acct.lusd_balance = 10_000_000;
    }

    state.total_lusd_supply = state.total_lusd_supply.saturating_add(10_000_000);
    state.stabilization_pool_balance = state.stabilization_pool_balance.saturating_add(10_000_000);
    state.reserve_ratio = 1.0;

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 1,
    };

    let si = StablecoinInstruction::InstantRedeem {
        amount: (data.amount % 1_000_000).saturating_add(1),
        destination: data.destination,
    };

    let _ = execute_si(&si, &data.sender, &mut ctx);
});
