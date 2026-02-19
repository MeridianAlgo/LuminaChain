# lumina-simulation

Dedicated simulation module for LuminaChain.

- Uses the **same real execution algorithm** from `lumina-execution`.
- Creates fresh wallets, airdrops configurable simulation money, executes a PoR-backed mint, then runs transfer load.
- Isolated in its own folder/crate so simulation concerns are separated from production node binaries.

## Run

```bash
cargo run -p lumina-simulation -- --wallets 200 --transfers 20000 --simulation-money 50000
```

## What it validates

- Wallet generation and transaction signing.
- Nonce handling in the real transaction executor.
- Mint path with non-empty valid PoR proof.
- Transfer throughput and deterministic state transitions.
