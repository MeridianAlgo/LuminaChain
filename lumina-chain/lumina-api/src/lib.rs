use axum::{
    routing::{get, post},
    Router, Json, extract::{State, Path},
};
use lumina_types::transaction::Transaction;
use lumina_types::block::Block;
use lumina_types::state::GlobalState;
use lumina_types::instruction::{StablecoinInstruction, AssetType};
use lumina_storage::db::Storage;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;
use serde::Deserialize;

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

    let app = Router::new()
        .route("/", get(root))
        .route("/state", get(get_state))
        .route("/tx", post(submit_tx))
        .route("/block/:height", get(get_block))
        .route("/faucet", post(faucet))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("API listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Lumina Chain API v0.2 (Phase 2 Enabled)"
}

async fn get_state(State(state): State<AppState>) -> Json<GlobalState> {
    let guard = state.global_state.read().await;
    Json(guard.clone())
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

async fn submit_tx(
    State(state): State<AppState>,
    Json(tx): Json<Transaction>,
) -> Json<String> {
    match state.tx_sender.send(tx).await {
        Ok(_) => Json("Transaction submitted".to_string()),
        Err(_) => Json("Failed to submit transaction".to_string()),
    }
}

async fn faucet(
    State(state): State<AppState>,
    Json(_req): Json<serde_json::Value>,
) -> Json<String> {
    let tx = Transaction {
        sender: [0u8; 32],
        nonce: 0,
        instruction: StablecoinInstruction::MintSenior {
            amount: 1000,
            collateral_amount: 0,
            proof: vec![],
        },
        signature: vec![],
        gas_limit: 100000,
        gas_price: 1,
    };
    
    match state.tx_sender.send(tx).await {
        Ok(_) => Json("Faucet tx submitted".to_string()),
        Err(_) => Json("Failed".to_string()),
    }
}
