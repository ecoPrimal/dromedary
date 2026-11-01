//! 🐪 Dromedary — Gaia's sovereign reverse proxy.
//!
//! The 17th primal. One hump. Carries her own water.
//! Replaces Caddy (Go) with Pure Rust. Gaia IS the proxy.
//!
//! Usage:
//!   dromedary                         # load dromedary.toml from cwd
//!   dromedary --config /path/to.toml  # explicit config path
//!
//! The config is the CRUSHED Caddyfile — 64KB of Go DSL ground into
//! Rust-native TOML. Every subdomain, every route, every blackwall rule.

use dromedary_core::{Dromedary, DromedaryConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dromedary=info,tower_http=info".parse().unwrap()),
        )
        .init();

    let config_path = std::env::args()
        .skip_while(|a| a != "--config")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dromedary.toml"));

    tracing::info!("🐪 loading config from {}", config_path.display());

    let config = DromedaryConfig::from_file(&config_path)?;

    tracing::info!(
        "🐪 Dromedary v{} — {} sites, blackwall={}",
        env!("CARGO_PKG_VERSION"),
        config.sites.len(),
        config.blackwall.block_meta,
    );

    let server = Dromedary::new(config);
    server.run().await
}
