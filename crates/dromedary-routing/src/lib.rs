//! dromedary-routing — hostname → backend resolution.
//!
//! The routing table is the crushed Caddyfile. Every hostname maps to a backend.
//! Wildcard matching, sub-route matching, redirect chains.

/// Placeholder — routing logic lives in dromedary-core::config::find_site()
/// for now. This crate will grow to handle:
///   - Wildcard subdomain matching (*.primals.eco)
///   - Path prefix stripping
///   - Weighted load balancing (future: multi-gate)
///   - Circuit breaker (if upstream is down)
///   - WebSocket upgrade detection

pub fn version() -> &'static str {
    "0.1.0"
}
