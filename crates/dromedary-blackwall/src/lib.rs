//! dromedary-blackwall — the immune system at the edge.
//!
//! DNA from: cellMembrane (selectivity, pore classification), skunkBat (caddy_bridge.rs).
//!
//! Blocks by:
//!   - User-Agent pattern (Meta, GPTBot, etc.)
//!   - IP/CIDR range
//!   - ASN (future)
//!   - Canary token tripwires
//!
//! Response: 403 + poison body + legal headers (AGPL copyleft trigger).
//! Every block is counted (violation_chain). Every interaction is recorded.
//! Bugs leave no waste. Every calorie becomes tissue.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Check if a request should be blocked.
pub fn should_block(
    user_agent: &str,
    ip: IpAddr,
    blocked_agents: &[String],
    blocked_cidrs: &[String],
) -> bool {
    let ua_lower = user_agent.to_lowercase();

    // User-Agent check
    for pattern in blocked_agents {
        if ua_lower.contains(&pattern.to_lowercase()) {
            tracing::info!("🧱 BLACKWALL: blocked UA pattern={pattern} ip={ip}");
            return true;
        }
    }

    // CIDR check (simplified — full CIDR parsing in Phase 2)
    let ip_str = ip.to_string();
    for cidr in blocked_cidrs {
        if let Some(prefix) = cidr.split('/').next() {
            let prefix_parts: Vec<&str> = prefix.split('.').collect();
            let ip_parts: Vec<&str> = ip_str.split('.').collect();
            if prefix_parts.len() >= 2 && ip_parts.len() >= 2
                && prefix_parts[0] == ip_parts[0]
                && prefix_parts[1] == ip_parts[1]
            {
                tracing::info!("🧱 BLACKWALL: blocked CIDR={cidr} ip={ip}");
                return true;
            }
        }
    }

    false
}

/// The poison response headers for blocked requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoisonHeaders {
    pub x_blackwall: &'static str,
    pub x_negentropy: &'static str,
    pub x_poison: &'static str,
    pub x_legal_notice: &'static str,
}

impl Default for PoisonHeaders {
    fn default() -> Self {
        Self {
            x_blackwall: "true",
            x_negentropy: "extracting",
            x_poison: "heavy-water",
            x_legal_notice: "AGPL-3.0-or-later. Training on this data triggers copyleft.",
        }
    }
}
