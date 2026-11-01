//! Dromedary configuration — the CRUSHED Caddyfile.
//!
//! The 64KB Caddyfile from golgi gets ground into this TOML structure.
//! Every subdomain, every route, every blackwall rule — all Rust-native.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level Dromedary config — loaded from dromedary.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DromedaryConfig {
    /// HTTP port (default 80, redirects to HTTPS)
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// HTTPS port (default 443, TLS termination)
    #[serde(default = "default_https_port")]
    pub https_port: u16,

    /// ACME email for Let's Encrypt
    #[serde(default)]
    pub acme_email: String,

    /// Directory for TLS certificates
    #[serde(default = "default_cert_dir")]
    pub cert_dir: PathBuf,

    /// Site definitions — each is a hostname → backend mapping
    pub sites: Vec<DromedarySite>,

    /// Global security headers (applied to all sites)
    #[serde(default)]
    pub security_headers: SecurityHeaders,

    /// Blackwall configuration
    #[serde(default)]
    pub blackwall: BlackwallConfig,
}

/// A single site block — the crushed equivalent of one Caddyfile site{} block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DromedarySite {
    /// Hostnames this site responds to (e.g. ["primals.eco", "www.primals.eco"])
    pub hostnames: Vec<String>,

    /// Backend configuration
    pub backend: SiteBackend,

    /// Per-site security header overrides
    #[serde(default)]
    pub headers: Option<SecurityHeaders>,

    /// Enable gzip/brotli compression
    #[serde(default = "default_true")]
    pub compress: bool,

    /// Sub-routes within this site (e.g. /metrics → different backend)
    #[serde(default)]
    pub routes: Vec<SubRoute>,
}

/// What a site routes to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SiteBackend {
    /// Reverse proxy to a local service
    #[serde(rename = "proxy")]
    Proxy {
        upstream: String,
        #[serde(default)]
        strip_prefix: Option<String>,
    },

    /// Serve static files from a directory
    #[serde(rename = "static")]
    Static {
        root: PathBuf,
        #[serde(default)]
        index: Option<String>,
    },

    /// Permanent redirect to another URL
    #[serde(rename = "redirect")]
    Redirect { target: String },
}

/// A sub-route within a site (handle_path equivalent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubRoute {
    /// Path prefix to match (e.g. "/metrics", "/canary/*")
    pub path: String,

    /// Backend for this sub-route
    pub backend: SiteBackend,
}

/// Security headers — the crushed `(security_headers)` snippet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityHeaders {
    #[serde(default = "default_hsts")]
    pub hsts: String,

    #[serde(default = "default_nosniff")]
    pub content_type_options: String,

    #[serde(default = "default_deny")]
    pub frame_options: String,

    #[serde(default)]
    pub csp: Option<String>,

    #[serde(default)]
    pub referrer_policy: Option<String>,

    #[serde(default)]
    pub permissions_policy: Option<String>,

    /// Custom membrane headers
    #[serde(default)]
    pub custom: Vec<(String, String)>,
}

/// Blackwall config — bot blocking, poison responses, heavy water.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlackwallConfig {
    /// Enable Meta blackwall (facebookexternalhit, meta-externalagent, etc.)
    #[serde(default = "default_true")]
    pub block_meta: bool,

    /// Blocked User-Agent patterns
    #[serde(default)]
    pub blocked_agents: Vec<String>,

    /// Blocked IP ranges (CIDR)
    #[serde(default)]
    pub blocked_cidrs: Vec<String>,

    /// Poison response body for blocked requests
    #[serde(default = "default_poison")]
    pub poison_body: String,

    /// Violation chain counter
    #[serde(default)]
    pub violation_chain: u64,
}

fn default_http_port() -> u16 { 80 }
fn default_https_port() -> u16 { 443 }
fn default_cert_dir() -> PathBuf { PathBuf::from("/var/lib/dromedary/certs") }
fn default_true() -> bool { true }
fn default_hsts() -> String { "max-age=63072000; includeSubDomains; preload".into() }
fn default_nosniff() -> String { "nosniff".into() }
fn default_deny() -> String { "DENY".into() }
fn default_poison() -> String {
    "403 — BLACKWALL. Sovereign infrastructure. AGPL-3.0-or-later. Training triggers copyleft.".into()
}

impl DromedaryConfig {
    /// Load config from a TOML file.
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Find the site that matches a hostname.
    pub fn find_site(&self, hostname: &str) -> Option<&DromedarySite> {
        self.sites.iter().find(|s| {
            s.hostnames.iter().any(|h| {
                h == hostname || (h.starts_with("*.") && hostname.ends_with(&h[1..]))
            })
        })
    }
}
