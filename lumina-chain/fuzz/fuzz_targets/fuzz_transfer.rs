#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use lumina_execution::{execute_si, ExecutionContext};
use lumina_types::instruction::{AssetType, StablecoinInstruction};
use lumina_types::state::GlobalState;

#[derive(Arbitrary, Debug)]
struct TransferInput {
    sender: [u8; 32],
    to: [u8; 32],
    amount: u64,
    asset_kind: u8,
}

fn asset_from(k: u8) -> AssetType {
    match k % 3 {
        0 => AssetType::LUSD,
        1 => AssetType::LJUN,
        _ => AssetType::Lumina,
    }
}

fuzz_target!(|data: TransferInput| {
    let mut state = GlobalState::default();

    {
        let acct = state.accounts.entry(data.sender).or_default();
        acct.lusd_balance = 10_000_000;
        acct.ljun_balance = 10_000_000;
        acct.lumina_balance = 10_000_000;
    }

    let mut ctx = ExecutionContext {
        state: &mut state,
        height: 1,
        timestamp: 1,
    };

    let si = StablecoinInstruction::Transfer {
        to: data.to,
        amount: data.amount % 10_000_000,
        asset: asset_from(data.asset_kind),
    };

    let _ = execute_si(&si, &data.sender, &mut ctx);
});
