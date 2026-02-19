# LuminaChain L1: The Sovereign Stablecoin Network

LuminaChain is a production-grade, state-of-the-art Layer 1 blockchain specifically engineered for the LuminaUSD (LUSD) stablecoin. It addresses fundamental stablecoin failure modes by integrating advanced economic mechanisms, high-performance networking, and verifiable security directly into the protocol.

## 1. Core Architecture

LuminaChain is built with a focus on high throughput, deterministic execution, and formal verification of supply dynamics.

*   **Execution Engine**: Implements over 50 native `StablecoinInstructions` (SIs) covering monetary policy, custodian management, and advanced financial primitives.
*   **Consensus**: High-performance BFT consensus layer ensuring finality and liveness for global stability operations.
*   **Storage**: Merkle Patricia Trie (MPT) for verifiable state roots and RocksDB for high-speed persistence.
*   **Networking**: Libp2p QUIC integration with GossipSub and Request-Response protocols for resilient block propagation.

## 2. Advanced Stablecoin Features (Implemented)

LuminaChain is not just a ledger; it is an active economic agent. Every feature is a first-class instruction:

*   **Dual-Tranche Stability**: Senior tranche (LUSD) for stability and Junior tranche (LJUN) for capital growth and risk absorption.
*   **Merkle Patricia Trie PoR**: Real-time on-chain verifiable Proof of Reserves.
*   **Insurance Fund**: A dedicated buffer (seeded with 5% of mint fees) to absorb black swan events.
*   **Lumina Health Index**: A weighted composite score indicating the absolute health of the protocol, visible to all institutional participants.
*   **Native Passkey Accounts**: Seedless security with social recovery and guardian thresholds.
*   **Post-Quantum Readiness**: Fallback support for quantum-resistant signature schemes.
*   **Green Validator Bonus**: 2x consensus power for validators providing verified renewable energy proofs.
*   **Velocity Incentives**: Protocol-native rewards for high transaction velocity, encouraging ecosystem liquidity.

## 3. Technology Stack

*   **Language**: Rust (Memory-safe, high-performance)
*   **Cryptography**: `ed25519-dalek`, `blake3`, Arkworks (Groth16 ZK-SNARKs)
*   **Storage**: RocksDB with in-memory Merkle Patricia Trie
*   **P2P**: Libp2p (GossipSub, Noise, Yamux)
*   **API**: Axum (REST) and Tonic (gRPC)

## 4. Getting Started

### Prerequisites
- Rust 1.85+
- Docker & Docker Compose

### Building from Source
```bash
cargo build --release
```

### Running a Node
```bash
./target/release/lumina-node --validator --data-dir ./data
```

### High-Throughput Simulation (separate module)
To validate wallet creation, simulation-money funding, PoR-backed minting, and real transfer execution in an isolated simulation crate:
```bash
cargo run -p lumina-simulation --release -- --wallets 200 --transfers 20000 --simulation-money 50000
```

## 5. Security and Integrity

LuminaChain emphasizes a "Security-First" approach:
*   **Deterministic Execution**: Every transaction result is identical across all nodes.
*   **Overflow Protection**: Checked arithmetic used for all monetary operations.
*   **Auditable Supply**: Total supply is verifiable via on-chain MPT roots.

## 6. Evolution and Excellence

The LuminaChain protocol is designed for continuous refinement:
*   **MPC Custodian Marketplace**: Enabling decentralized collateral management.
*   **RWA Tokenization Path**: Direct integration for real-world asset collateralization.
*   **Confidential Transfers**: Integrated ZK circuits for transaction privacy.
*   **Institutional Yield Curve**: Native yield markets for LUSD and LJUN.
