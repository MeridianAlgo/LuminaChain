//! lumina-oracles/src/price_feed.rs
//! Lumina Adaptive Stability Oracle (LASO) — decentralized mode

use anyhow::{bail, Result};
use lumina_crypto::signatures::verify_signature;
use lumina_types::state::GlobalState;
use std::collections::{BTreeMap, VecDeque};

const MIN_REPORTERS: usize = 7;
const REPORT_STALENESS_SECONDS: u64 = 300;
const SLASH_THRESHOLD_BPS: u64 = 1_000; // 10%

#[derive(Debug, Clone, PartialEq)]
pub enum OracleRegime {
    Stable,
    Volatile,
    Stress,
}

#[derive(Debug, Clone)]
pub struct PriceReport {
    pub price: f64,
    pub timestamp: u64,
    pub confidence: u8,
    pub volatility_1h_forecast: f64,
    pub stability_impact: f64,
    pub regime: OracleRegime,
    pub data_hash: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct SignedPriceReport {
    pub reporter_pubkey: [u8; 32],
    pub asset: String,
    pub price: f64,
    pub timestamp: u64,
    pub signature: [u8; 64],
    pub stake: u64,
}

#[derive(Debug, Clone)]
pub struct ReporterState {
    pub stake: u64,
    pub reputation: f64,
    pub total_slashed: u64,
}

pub struct PriceFeed {
    symbol: String,
    reports: BTreeMap<[u8; 32], SignedPriceReport>,
    reporters: BTreeMap<[u8; 32], ReporterState>,
    price_history: VecDeque<f64>,
    last_aggregate: Option<PriceReport>,
    slash_treasury: u64,
}

impl PriceFeed {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            reports: BTreeMap::new(),
            reporters: BTreeMap::new(),
            price_history: VecDeque::with_capacity(60),
            last_aggregate: None,
            slash_treasury: 0,
        }
    }

    pub fn register_reporter(&mut self, pubkey: [u8; 32], stake: u64) -> Result<()> {
        if stake == 0 {
            bail!("reporter stake must be > 0")
        }
        self.reporters.insert(
            pubkey,
            ReporterState {
                stake,
                reputation: 0.7,
                total_slashed: 0,
            },
        );
        Ok(())
    }

    pub fn reporter_state(&self, pubkey: &[u8; 32]) -> Option<&ReporterState> {
        self.reporters.get(pubkey)
    }

    pub fn slash_treasury(&self) -> u64 {
        self.slash_treasury
    }

    /// Main entry point — decentralized, reputation + stake-weighted aggregation.
    pub async fn get_latest_report(&mut self, state: &mut GlobalState) -> Result<PriceReport> {
        let now = current_unix_ts()?;
        let mut weighted_prices = Vec::new();

        for (pubkey, signed) in &self.reports {
            if signed.asset != self.symbol || !self.is_report_fresh(signed, now) {
                continue;
            }
            if let Some(reporter) = self.reporters.get(pubkey) {
                let weight = reporter.reputation.max(0.0) * reporter.stake as f64;
                if weight > 0.0 {
                    weighted_prices.push((*pubkey, signed.price, weight));
                }
            }
        }

        if weighted_prices.len() < MIN_REPORTERS {
            bail!(
                "insufficient decentralized reporters for {}: {} < {}",
                self.symbol,
                weighted_prices.len(),
                MIN_REPORTERS
            );
        }

        let report = self.aggregate_with_laso(&weighted_prices)?;
        self.last_aggregate = Some(report.clone());

        self.apply_reputation_and_slashing(weighted_prices, report.price)?;
        self.apply_to_state(&report, state);

        Ok(report)
    }

    fn aggregate_with_laso(
        &mut self,
        weighted_prices: &[([u8; 32], f64, f64)],
    ) -> Result<PriceReport> {
        let mut sorted: Vec<_> = weighted_prices.to_vec();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let median_price = sorted[sorted.len() / 2].1;
        let mad: f64 = sorted
            .iter()
            .map(|(_, p, _)| (p - median_price).abs())
            .sum::<f64>()
            / sorted.len() as f64;

        let filtered: Vec<([u8; 32], f64, f64)> = sorted
            .into_iter()
            .filter(|(_, p, _)| mad < f64::EPSILON || (p - median_price).abs() <= 3.0 * mad)
            .collect();

        let total_weight: f64 = filtered.iter().map(|(_, _, w)| w).sum();
        if total_weight <= f64::EPSILON {
            bail!("invalid aggregated weight")
        }

        let final_price = filtered.iter().map(|(_, p, w)| p * w).sum::<f64>() / total_weight;

        self.price_history.push_back(final_price);
        if self.price_history.len() > 60 {
            self.price_history.pop_front();
        }

        let vol = self.calculate_ewma_volatility();
        let momentum = if self.price_history.len() >= 2 {
            *self.price_history.back().unwrap_or(&final_price)
                - self.price_history[self.price_history.len() - 2]
        } else {
            0.0
        };

        let forecast_vol = vol * 1.2 + momentum.abs() * 0.3;
        let regime = if forecast_vol > 0.08 {
            OracleRegime::Stress
        } else if forecast_vol > 0.03 {
            OracleRegime::Volatile
        } else {
            OracleRegime::Stable
        };

        let peg_dev = (final_price - 1.0).abs();
        let stability_impact = (1.0 - peg_dev * 10.0 - forecast_vol * 5.0).clamp(0.0, 1.0);
        let confidence = ((filtered.len() as f64 / weighted_prices.len() as f64) * 100.0) as u8;
        let data_hash = blake3::hash(&final_price.to_le_bytes()).into();

        Ok(PriceReport {
            price: final_price,
            timestamp: current_unix_ts()?,
            confidence: confidence.min(100),
            volatility_1h_forecast: forecast_vol,
            stability_impact,
            regime,
            data_hash,
        })
    }

    fn apply_reputation_and_slashing(
        &mut self,
        reports: Vec<([u8; 32], f64, f64)>,
        aggregate_price: f64,
    ) -> Result<()> {
        for (pubkey, reported_price, _) in reports {
            let reporter = self
                .reporters
                .get_mut(&pubkey)
                .ok_or_else(|| anyhow::anyhow!("missing reporter state"))?;

            let err_ratio = if aggregate_price.abs() > f64::EPSILON {
                (reported_price - aggregate_price).abs() / aggregate_price
            } else {
                0.0
            };

            reporter.reputation = (reporter.reputation * 0.9) + (0.1 * (1.0 - err_ratio.min(1.0)));
            reporter.reputation = reporter.reputation.clamp(0.0, 1.0);

            let err_bps = (err_ratio * 10_000.0).round() as u64;
            if err_bps > SLASH_THRESHOLD_BPS {
                let slash_amount = reporter.stake / 50; // 2%
                reporter.stake = reporter.stake.saturating_sub(slash_amount);
                reporter.total_slashed = reporter.total_slashed.saturating_add(slash_amount);
                self.slash_treasury = self.slash_treasury.saturating_add(slash_amount);
                reporter.reputation = (reporter.reputation * 0.8).max(0.0);
            }
        }
        Ok(())
    }

    fn calculate_ewma_volatility(&self) -> f64 {
        if self.price_history.len() < 2 {
            return 0.01;
        }
        let mut ewma = 0.0;
        let alpha = 0.2;
        let mut prev = *self.price_history.front().unwrap_or(&1.0);
        for &p in &self.price_history {
            if prev.abs() > f64::EPSILON {
                let ret = (p - prev) / prev;
                ewma = alpha * ret * ret + (1.0 - alpha) * ewma;
            }
            prev = p;
        }
        ewma.sqrt()
    }

    fn is_report_fresh(&self, report: &SignedPriceReport, now: u64) -> bool {
        now.saturating_sub(report.timestamp) < REPORT_STALENESS_SECONDS
    }

    /// P2P signed report ingestion
    pub fn add_signed_report(&mut self, report: SignedPriceReport) -> Result<()> {
        if report.asset != self.symbol {
            bail!("asset mismatch for report")
        }
        if !self.reporters.contains_key(&report.reporter_pubkey) {
            bail!("unregistered reporter")
        }

        verify_signature(
            &report.reporter_pubkey,
            &report.price.to_le_bytes(),
            &report.signature[..],
        )?;

        if let Some(reporter) = self.reporters.get_mut(&report.reporter_pubkey) {
            reporter.stake = report.stake;
        }
        self.reports.insert(report.reporter_pubkey, report);
        Ok(())
    }

    fn apply_to_state(&self, report: &PriceReport, state: &mut GlobalState) {
        let fixed = (report.price * 1_000_000.0).round().max(0.0) as u64;
        state.oracle_prices.insert(self.symbol.clone(), fixed);

        if report.stability_impact < 0.75 {
            state.circuit_breaker_active = true;
        }

        if state.total_lusd_supply > 0 {
            state.reserve_ratio =
                state.stabilization_pool_balance as f64 / state.total_lusd_supply as f64;
        }
    }
}

fn current_unix_ts() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn decentralized_aggregation_and_slashing() {
        let mut feed = PriceFeed::new("LUSD-USD");
        let mut state = GlobalState::default();

        let mut outlier_pubkey = [0u8; 32];
        for i in 0..7 {
            let kp = lumina_crypto::signatures::generate_keypair();
            let pubkey = kp.verifying_key().to_bytes();
            if i == 6 {
                outlier_pubkey = pubkey;
            }
            feed.register_reporter(pubkey, 1_000_000).unwrap();

            let price = if i == 6 {
                1.6
            } else {
                1.0 + (i as f64 * 0.001)
            };
            let sig = lumina_crypto::signatures::sign(&kp, &price.to_le_bytes());
            let mut sig_arr = [0u8; 64];
            sig_arr.copy_from_slice(&sig[..64]);
            feed.add_signed_report(SignedPriceReport {
                reporter_pubkey: pubkey,
                asset: "LUSD-USD".to_string(),
                price,
                timestamp: current_unix_ts().unwrap(),
                signature: sig_arr,
                stake: 1_000_000,
            })
            .unwrap();
        }

        let report = feed.get_latest_report(&mut state).await.unwrap();
        assert!(report.price > 0.99 && report.price < 1.05);
        assert!(feed.slash_treasury() > 0);
        let outlier = feed.reporter_state(&outlier_pubkey).unwrap();
        assert!(outlier.total_slashed > 0);
    }
}
