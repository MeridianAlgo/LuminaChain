# lumina-simulation

Dedicated simulation module for LuminaChain.

- Uses the **same real execution algorithm** from `lumina-execution`.
- Creates fresh wallets, airdrops configurable simulation money, activates custom assets, executes a PoR-backed mint, then runs mixed-asset transfer load.
- Isolated in its own folder/crate so simulation concerns are separated from production node binaries.

## Run

```bash
cargo run -p lumina-simulation -- --wallets 200 --transfers 20000 --simulation-money 50000 --custom-assets BTC,ETH,SOL --custom-asset-amount 100
```

## What it validates

- Wallet generation and transaction signing.
- Nonce handling in the real transaction executor.
- Mint path with non-empty valid PoR proof.
- Transfer throughput and deterministic state transitions.
- Multi-asset wallet state with custom crypto balances.
