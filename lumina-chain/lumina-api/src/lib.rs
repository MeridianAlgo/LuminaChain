use axum::{
    routing::{get, post},
    Router, Json, extract::{State, Path},
    response::{IntoResponse, Response},
    http::{HeaderMap, HeaderValue, StatusCode},
};
use lumina_types::transaction::Transaction;
use lumina_types::block::Block;
use lumina_types::state::GlobalState;
use lumina_storage::db::Storage;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

#[derive(Clone)]
pub struct AppState {
    pub global_state: Arc<RwLock<GlobalState>>,
    pub storage: Arc<Storage>,
    pub tx_sender: mpsc::Sender<Transaction>,
}

pub async fn start_server(
    global_state: Arc<RwLock<GlobalState>>,
    storage: Arc<Storage>,
    tx_sender: mpsc::Sender<Transaction>,
) {
    let state = AppState { global_state, storage, tx_sender };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/", get(root))
        .route("/state", get(get_state))
        .route("/health", get(get_health))
        .route("/metrics", get(get_metrics))
        .route("/tx/signing_bytes", post(tx_signing_bytes))
        .route("/tx", post(submit_tx))
        .route("/block/{height}", get(get_block))
        .route("/account/{address}", get(get_account))
        .route("/faucet", post(faucet))
        .route("/validators", get(get_validators))
        .route("/insurance", get(get_insurance))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "LuminaChain API v1.0 — Production L1 Stablecoin Network"
}

async fn get_state(State(state): State<AppState>) -> Json<serde_json::Value> {
    let guard = state.global_state.read().await;
    let summary = serde_json::json!({
        "total_lusd_supply": guard.total_lusd_supply,
        "total_ljun_supply": guard.total_ljun_supply,
        "reserve_ratio": guard.reserve_ratio,
        "stabilization_pool_balance": guard.stabilization_pool_balance,
        "circuit_breaker_active": guard.circuit_breaker_active,
        "insurance_fund_balance": guard.insurance_fund_balance,
        "health_index": guard.health_index,
        "validator_count": guard.validators.len(),
        "custodian_count": guard.custodians.len(),
        "rwa_listing_count": guard.rwa_listings.len(),
        "pending_redeem_queue": guard.fair_redeem_queue.len(),
        "current_epoch": guard.current_epoch,
        "velocity_reward_pool": guard.velocity_reward_pool,
        "account_count": guard.accounts.len(),
    });
    Json(summary)
}

async fn get_health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let guard = state.global_state.read().await;
    let health = serde_json::json!({
        "health_index": guard.health_index,
        "health_pct": format!("{:.2}%", guard.health_index as f64 / 100.0),
        "reserve_ratio": guard.reserve_ratio,
        "circuit_breaker_active": guard.circuit_breaker_active,
        "insurance_fund_balance": guard.insurance_fund_balance,
        "green_validator_count": guard.validators.iter().filter(|v| v.is_green).count(),
        "total_validator_count": guard.validators.len(),
    });
    Json(health)
}

async fn get_metrics(State(state): State<AppState>) -> Response {
    let guard = state.global_state.read().await;

    let mut registry = Registry::default();

    fn as_i64_u64(v: u64) -> i64 {
        i64::try_from(v).unwrap_or(i64::MAX)
    }

    fn as_i64_usize(v: usize) -> i64 {
        i64::try_from(v).unwrap_or(i64::MAX)
    }

    let mut health_index = Gauge::<i64>::default();
    health_index.set(as_i64_u64(guard.health_index));
    registry.register("lumina_health_index", "Health index (0..10000)", health_index);

    let mut reserve_ratio_bps = Gauge::<i64>::default();
    let rr_bps = (guard.reserve_ratio.max(0.0) * 10_000.0) as u64;
    reserve_ratio_bps.set(as_i64_u64(rr_bps));
    registry.register(
        "lumina_reserve_ratio_bps",
        "Reserve ratio in basis points (reserve_ratio * 10000)",
        reserve_ratio_bps,
    );

    let mut total_lusd_supply = Gauge::<i64>::default();
    total_lusd_supply.set(as_i64_u64(guard.total_lusd_supply));
    registry.register("lumina_total_lusd_supply", "Total LUSD supply", total_lusd_supply);

    let mut total_ljun_supply = Gauge::<i64>::default();
    total_ljun_supply.set(as_i64_u64(guard.total_ljun_supply));
    registry.register("lumina_total_ljun_supply", "Total LJUN supply", total_ljun_supply);

    let mut stabilization_pool_balance = Gauge::<i64>::default();
    stabilization_pool_balance.set(as_i64_u64(guard.stabilization_pool_balance));
    registry.register(
        "lumina_stabilization_pool_balance",
        "Stabilization pool balance",
        stabilization_pool_balance,
    );

    let mut insurance_fund_balance = Gauge::<i64>::default();
    insurance_fund_balance.set(as_i64_u64(guard.insurance_fund_balance));
    registry.register(
        "lumina_insurance_fund_balance",
        "Insurance fund balance",
        insurance_fund_balance,
    );

    let mut circuit_breaker_active = Gauge::<i64>::default();
    circuit_breaker_active.set(if guard.circuit_breaker_active { 1 } else { 0 });
    registry.register(
        "lumina_circuit_breaker_active",
        "Circuit breaker active (1/0)",
        circuit_breaker_active,
    );

    let mut validator_count = Gauge::<i64>::default();
    validator_count.set(as_i64_usize(guard.validators.len()));
    registry.register("lumina_validator_count", "Validator count", validator_count);

    let mut green_validator_count = Gauge::<i64>::default();
    green_validator_count.set(as_i64_usize(guard.validators.iter().filter(|v| v.is_green).count()));
    registry.register(
        "lumina_green_validator_count",
        "Green validator count",
        green_validator_count,
    );

    let mut account_count = Gauge::<i64>::default();
    account_count.set(as_i64_usize(guard.accounts.len()));
    registry.register("lumina_account_count", "Account count", account_count);

    let mut rwa_listing_count = Gauge::<i64>::default();
    rwa_listing_count.set(as_i64_usize(guard.rwa_listings.len()));
    registry.register("lumina_rwa_listing_count", "RWA listing count", rwa_listing_count);

    let mut out = String::new();
    if encode(&mut out, &registry).is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    (headers, out).into_response()
}

async fn get_block(
    State(state): State<AppState>,
    Path(height): Path<u64>,
) -> Json<Option<Block>> {
    match state.storage.load_block_by_height(height) {
        Ok(block) => Json(block),
        Err(_) => Json(None),
    }
}

#[derive(serde::Deserialize)]
struct UnsignedTxRequest {
    pub sender: [u8; 32],
    pub nonce: u64,
    pub instruction: lumina_types::instruction::StablecoinInstruction,
    pub gas_limit: u64,
    pub gas_price: u64,
}

async fn tx_signing_bytes(Json(req): Json<UnsignedTxRequest>) -> Json<serde_json::Value> {
    let tx = Transaction {
        sender: req.sender,
        nonce: req.nonce,
        instruction: req.instruction,
        signature: Vec::new(),
        gas_limit: req.gas_limit,
        gas_price: req.gas_price,
    };

    let signing_bytes = tx.signing_bytes();
    Json(serde_json::json!({
        "signing_bytes_hex": hex::encode(signing_bytes),
    }))
}

async fn get_account(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Json<serde_json::Value> {
    let guard = state.global_state.read().await;
    let addr_hex = address.trim_start_matches("0x");
    if let Ok(bytes) = hex::decode(addr_hex) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            if let Some(account) = guard.accounts.get(&key) {
                return Json(serde_json::json!({
                    "address": address,
                    "lusd_balance": account.lusd_balance,
                    "ljun_balance": account.ljun_balance,
                    "lumina_balance": account.lumina_balance,
                    "nonce": account.nonce,
                    "has_passkey": account.passkey_device_key.is_some(),
                    "guardian_count": account.guardians.len(),
                    "has_pq": account.pq_pubkey.is_some(),
                    "credit_score": account.credit_score,
                    "yield_positions": account.yield_positions.len(),
                    "active_streams": account.active_streams.len(),
                }));
            }
        }
    }
    Json(serde_json::json!({"error": "Account not found"}))
}

async fn submit_tx(
    State(state): State<AppState>,
    Json(tx): Json<Transaction>,
) -> Json<serde_json::Value> {
    let tx_id = hex::encode(tx.id());
    match state.tx_sender.send(tx).await {
        Ok(_) => Json(serde_json::json!({
            "status": "submitted",
            "tx_id": tx_id,
        })),
        Err(_) => Json(serde_json::json!({
            "status": "failed",
            "error": "Channel full or closed",
        })),
    }
}

async fn faucet(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let amount: u64 = 10_000;

    let addr_hex = req
        .get("address")
        .and_then(|v| v.as_str())
        .unwrap_or("0x")
        .trim()
        .trim_start_matches("0x");

    let Ok(bytes) = hex::decode(addr_hex) else {
        return Json(serde_json::json!({
            "status": "failed",
            "error": "invalid address hex"
        }));
    };
    if bytes.len() != 32 {
        return Json(serde_json::json!({
            "status": "failed",
            "error": "address must be 32 bytes"
        }));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);

    let mut guard = state.global_state.write().await;
    let account = guard.accounts.entry(key).or_default();
    account.lusd_balance = account.lusd_balance.saturating_add(amount);
    guard.total_lusd_supply = guard.total_lusd_supply.saturating_add(amount);

    Json(serde_json::json!({
        "status": "funded",
        "address": format!("0x{}", addr_hex),
        "amount": amount,
        "asset": "LUSD"
    }))
}

async fn get_validators(State(state): State<AppState>) -> Json<serde_json::Value> {
    let guard = state.global_state.read().await;
    let validators: Vec<serde_json::Value> = guard
        .validators
        .iter()
        .map(|v| {
            serde_json::json!({
                "pubkey": hex::encode(v.pubkey),
                "stake": v.stake,
                "power": v.power,
                "is_green": v.is_green,
            })
        })
        .collect();
    Json(serde_json::json!({ "validators": validators }))
}

async fn get_insurance(State(state): State<AppState>) -> Json<serde_json::Value> {
    let guard = state.global_state.read().await;
    Json(serde_json::json!({
        "insurance_fund_balance": guard.insurance_fund_balance,
        "total_lusd_supply": guard.total_lusd_supply,
        "coverage_ratio": if guard.total_lusd_supply > 0 {
            guard.insurance_fund_balance as f64 / guard.total_lusd_supply as f64
        } else {
            1.0
        },
    }))
}
