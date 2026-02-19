# Community + Testnet Setup Checklist (Manual)

This issue requires external setup (Discord/X accounts + public hosting) and cannot be completed purely via code changes in this repository.

## 1) Discord
- [ ] Create official Discord server
- [ ] Create channels
  - [ ] #general
  - [ ] #announcements
  - [ ] #developers
  - [ ] #testnet
  - [ ] #support
- [ ] Configure moderation + permissions
- [ ] Create invite link
- [ ] Add link to `README.md`

## 2) X (Twitter)
- [ ] Create official account/handle
- [ ] Post initial announcement
- [ ] Add link to `README.md`

## 3) Public testnet (7 validators)
- [ ] Pick hosting provider (e.g. DO, AWS, bare metal)
- [ ] Provision 7 nodes (or 1 node + 6 replicas depending on your consensus topology)
- [ ] Expose RPC/API endpoints
- [ ] Add public endpoint to `README.md`
- [ ] Add monitoring (Prometheus/Grafana)

## 4) Faucet
- [ ] Decide faucet rate limits + abuse protection
- [ ] Deploy faucet frontend
- [ ] Ensure API server endpoint reachable publicly
- [ ] Document `lumina-cli faucet --address 0x...`

## 5) Announcement checklist
- [ ] 1st announcement: testnet launch + faucet
- [ ] 2nd announcement: how to run a node + contribute
- [ ] 3rd announcement: health index + metrics dashboard

## Done Definition
- Public RPC/API endpoint online
- Faucet usable by external testers
- Discord + X links present in repo README
