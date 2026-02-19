# Dual-Tranche Math Specification

## Variables
- `R`: reserves held by the protocol.
- `S`: total LUSD (senior) supply.
- `J`: total LJUN (junior) supply.
- `CR`: collateral ratio, `CR = R / S`.

## Collateral targets
- Healthy band: `1.20 <= CR <= 1.60`.
- Soft warning: `1.05 <= CR < 1.20`.
- Crisis mode: `CR < 1.05`.

## Rebalancing logic
1. If `CR > 1.60`, allocate incremental reserves to:
   - insurance fund (40%),
   - LJUN yield (40%),
   - protocol treasury (20%).
2. If `1.20 <= CR <= 1.60`, no forced rebalance.
3. If `CR < 1.20`, suspend junior redemptions and route fees to reserves.
4. If `CR < 1.05`, circuit breaker is activated and senior redemptions move to queue processing.

## Junior wipeout rules
- Let `loss = max(0, S - R)`.
- Junior absorbs first loss up to its notional buffer.
- If `loss >= J_buffer`, junior tranche is fully wiped before senior principal is touched.

## Redemption queue
- FIFO queue with deterministic ordering by `(timestamp, tx_hash)`.
- Each block processes up to `batch_size` requests.
- If reserves are insufficient, remaining requests stay queued without reordering.
