//! dromedary-acme — ACME certificate lifecycle.
//!
//! DNA from: beardog-acme (account.rs, challenge.rs, order.rs, jws.rs, storage.rs, hot_reload.rs).
//!
//! Flow:
//!   1. Register ACME account with Let's Encrypt
//!   2. Create order for domain(s)
//!   3. Solve HTTP-01 challenge (Dromedary serves /.well-known/acme-challenge/)
//!   4. Finalize order with CSR
//!   5. Download certificate
//!   6. Store cert + key on disk
//!   7. Hot-reload rustls ServerConfig
//!   8. Schedule renewal (before expiry)
//!
//! Phase 2: Port the beardog-acme code here.
//! Phase 3: On-demand TLS (cert per hostname on first request).

/// Placeholder — ACME integration is Phase 2.
/// The full beardog-acme crate has:
///   - account.rs (5.6KB) — ACME account registration
///   - challenge.rs (8.5KB) — HTTP-01 challenge solving
///   - order.rs (3.2KB) — Certificate order management
///   - jws.rs (5.8KB) — JWS signing for ACME protocol
///   - storage.rs (6KB) — Cert + key persistence
///   - hot_reload.rs (5.3KB) — Live cert reload without restart
///   - shadow_metrics.rs (11.9KB) — Certificate health metrics
///
/// Total: ~46KB of battle-tested ACME Rust code ready to port.

pub fn version() -> &'static str {
    "0.1.0"
}
