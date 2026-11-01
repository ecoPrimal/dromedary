//! TLS configuration — Pure Rust via rustls.
//!
//! Phase 1: Self-signed or manual cert loading.
//! Phase 2: ACME via dromedary-acme (derived from beardog-acme).
//! Phase 3: Hot reload on cert renewal.
//!
//! DNA from: beardog-acme (ACME lifecycle), songbird-tls (handshake + record layer).

use std::path::Path;
use std::sync::Arc;

/// Create a rustls ServerConfig for TLS termination.
///
/// Initially loads certs from disk. Later: ACME auto-provision.
pub fn make_tls_config(
    cert_path: &Path,
    key_path: &Path,
) -> anyhow::Result<Arc<rustls::ServerConfig>> {
    // Install the Pure Rust crypto provider (from bearDog's pattern)
    let _ = rustls::crypto::CryptoProvider::install_default(
        rustls_rustcrypto::provider()
    );

    let cert_pem = std::fs::read(cert_path)?;
    let key_pem = std::fs::read(key_path)?;

    let certs = rustls_pemfile::certs(&mut &cert_pem[..])
        .collect::<Result<Vec<_>, _>>()?;

    let key = rustls_pemfile::private_key(&mut &key_pem[..])?
        .ok_or_else(|| anyhow::anyhow!("no private key found in {}", key_path.display()))?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(Arc::new(config))
}

/// Placeholder for ACME bootstrap — will use beardog-acme DNA.
pub async fn bootstrap_acme(
    _email: &str,
    _domains: &[String],
    _cert_dir: &Path,
) -> anyhow::Result<()> {
    tracing::info!("🐪 ACME bootstrap — Phase 2 (beardog-acme integration)");
    // TODO: Port beardog-acme's account.rs, challenge.rs, order.rs, jws.rs
    // into dromedary-acme crate. Same crypto, same flow, Dromedary's cert store.
    Ok(())
}
