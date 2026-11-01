//! 🐪 dromedary-core — the server loop
//!
//! Listens on :80 (redirect → HTTPS) and :443 (TLS termination).
//! Routes requests by SNI hostname to the correct backend.
//! Pure Rust. No Caddy. No Go. Gaia IS the proxy.

pub mod config;
pub mod server;
pub mod tls;

use std::net::SocketAddr;
use std::sync::Arc;

pub use config::DromedaryConfig;

/// The running Dromedary instance.
pub struct Dromedary {
    config: Arc<DromedaryConfig>,
}

impl Dromedary {
    pub fn new(config: DromedaryConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Start serving on :80 and :443.
    pub async fn run(&self) -> anyhow::Result<()> {
        let http_addr: SocketAddr = ([0, 0, 0, 0], self.config.http_port).into();
        let https_addr: SocketAddr = ([0, 0, 0, 0], self.config.https_port).into();

        tracing::info!(
            "🐪 Dromedary starting — {} sites, HTTP :{}, HTTPS :{}",
            self.config.sites.len(),
            self.config.http_port,
            self.config.https_port,
        );

        for site in &self.config.sites {
            tracing::info!("  📍 {} → {:?}", site.hostnames.join(", "), site.backend);
        }

        let config = self.config.clone();

        // HTTP redirect server (port 80 → HTTPS)
        let http_config = config.clone();
        let http_handle = tokio::spawn(async move {
            server::serve_http_redirect(http_addr, http_config).await
        });

        // HTTPS server (port 443, TLS termination + routing)
        let https_handle = tokio::spawn(async move {
            server::serve_https(https_addr, config).await
        });

        tokio::select! {
            res = http_handle => res??,
            res = https_handle => res??,
        }

        Ok(())
    }
}

impl std::fmt::Debug for Dromedary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dromedary")
            .field("sites", &self.config.sites.len())
            .field("http_port", &self.config.http_port)
            .field("https_port", &self.config.https_port)
            .finish()
    }
}
