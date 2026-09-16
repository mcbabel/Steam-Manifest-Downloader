use anyhow::{anyhow, Context, Result};
use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use dryoc::dryocbox::{DryocBox, KeyPair, SecretKey};
use dryoc::types::ByteArray;
use serde_json::Value;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
    sync::Mutex,
};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};
use tracing::{info, warn};

const MAX_PAYLOAD_BYTES: usize = 64 * 1024;

struct AppState {
    keypair: KeyPair,
    data_dir: PathBuf,
    writer: Mutex<()>,
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--gen-keypair" || a == "--gen-key") {
        return gen_keypair();
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "smd_telemetry_server=info,tower_http=warn".into()),
        )
        .init();

    let secret_hex = std::env::var("TELEMETRY_PRIVATE_KEY_HEX")
        .context("TELEMETRY_PRIVATE_KEY_HEX env var not set")?;
    let secret_bytes: [u8; 32] = hex::decode(secret_hex.trim())
        .context("private key hex is not valid hex")?
        .try_into()
        .map_err(|_| anyhow!("private key must be exactly 32 bytes"))?;
    let keypair = keypair_from_secret(&secret_bytes);

    let data_dir: PathBuf = std::env::var("TELEMETRY_DATA_DIR")
        .unwrap_or_else(|_| "./data".into())
        .into();
    fs::create_dir_all(&data_dir).await.with_context(|| {
        format!("failed to create data dir {}", data_dir.display())
    })?;

    let bind = std::env::var("TELEMETRY_BIND").unwrap_or_else(|_| "127.0.0.1:9999".into());
    let addr: SocketAddr = bind.parse().context("TELEMETRY_BIND is not a valid addr")?;

    let pubkey_hex = hex::encode(keypair.public_key.as_array());
    let state = Arc::new(AppState {
        keypair,
        data_dir,
        writer: Mutex::new(()),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/events", post(ingest))
        .layer(RequestBodyLimitLayer::new(MAX_PAYLOAD_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, pubkey = %pubkey_hex, "telemetry server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn ingest(
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let sealed = DryocBox::from_sealed_bytes(body.as_ref()).map_err(|_| {
        warn!(bytes = body.len(), "failed to parse sealed box");
        (StatusCode::BAD_REQUEST, "parse failed")
    })?;
    let plaintext = sealed.unseal_to_vec(&state.keypair).map_err(|_| {
        warn!(bytes = body.len(), "failed to decrypt sealed box");
        (StatusCode::BAD_REQUEST, "decrypt failed")
    })?;

    let parsed: Value = serde_json::from_slice(&plaintext)
        .map_err(|_| (StatusCode::BAD_REQUEST, "payload is not valid json"))?;

    if !parsed.is_object() {
        return Err((StatusCode::BAD_REQUEST, "payload must be a JSON object"));
    }

    let received_at = Utc::now().to_rfc3339();
    let record = serde_json::json!({
        "received_at": received_at,
        "payload": parsed,
    });

    let today = Utc::now().format("%Y-%m-%d");
    let path = state.data_dir.join(format!("events-{}.jsonl", today));

    let mut line = serde_json::to_vec(&record)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "serialize"))?;
    line.push(b'\n');

    let _guard = state.writer.lock().await;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "open file"))?;
    file.write_all(&line)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "write"))?;
    file.flush()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "flush"))?;

    Ok(StatusCode::NO_CONTENT)
}

fn keypair_from_secret(secret: &[u8; 32]) -> KeyPair {
    KeyPair::from_secret_key(SecretKey::from(*secret))
}

fn gen_keypair() -> Result<()> {
    let kp = KeyPair::generate();
    let priv_hex = hex::encode(kp.secret_key.as_array());
    let pub_bytes = kp.public_key.as_array();

    eprintln!("== Server PRIVATE key (server env: TELEMETRY_PRIVATE_KEY_HEX) ==");
    println!("{}", priv_hex);
    eprintln!();
    eprintln!("== Server PUBLIC key (paste into src-tauri/src/services/telemetry.rs SERVER_PUBLIC_KEY) ==");
    eprintln!("const SERVER_PUBLIC_KEY: [u8; 32] = [");
    for chunk in pub_bytes.chunks(12) {
        let formatted: Vec<String> = chunk.iter().map(|b| format!("0x{:02x}", b)).collect();
        eprintln!("    {},", formatted.join(", "));
    }
    eprintln!("];");
    eprintln!();
    eprintln!("Store the private key somewhere safe (password manager). If you lose");
    eprintln!("it, no opt-in client can ever send events that this server can decrypt.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("ctrl-c received, shutting down"),
        _ = terminate => info!("SIGTERM received, shutting down"),
    }
}