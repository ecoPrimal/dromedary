//! 🐪 Dromedary — Gaia's sovereign reverse proxy.
//!
//! The 17th primal. One hump. Carries her own water.
//! Replaces Caddy (Go) with Pure Rust. Gaia IS the proxy.
//!
//! UniBin Architecture (ecoBin A++):
//!   • Single binary, multiple subcommands
//!   • server + doctor + version
//!   • Professional CLI with comprehensive help

use clap::{Parser, Subcommand};
use dromedary_core::{Dromedary, DromedaryConfig};
use std::path::PathBuf;

/// Dromedary — Sovereign Reverse Proxy & TLS Surface.
///
/// The 17th primal. One hump. Carries her own water.
/// Pure Rust replacement for Caddy.
#[derive(Parser)]
#[command(name = "dromedary", version, about, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Configuration file path.
    #[arg(short, long, global = true, default_value = "dromedary.toml")]
    config: PathBuf,
}

/// Available subcommands.
#[derive(Subcommand)]
enum Command {
    /// Start the Dromedary reverse proxy server (default).
    Server,
    /// Run health diagnostics and configuration validation.
    Doctor,
    /// Print version string.
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dromedary=info,tower_http=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Doctor) => {
            println!("🏥 Dromedary Health Diagnostics");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!();
            println!("📦 Binary Information");
            println!("   Name: dromedary");
            println!("   Version: {}", env!("CARGO_PKG_VERSION"));
            println!("   Role: Sovereign reverse proxy + TLS + ACME");
            println!();
            println!("📋 Configuration");
            println!("   Config path: {}", cli.config.display());
            match DromedaryConfig::from_file(&cli.config) {
                Ok(config) => {
                    println!("   Status: ✅ Valid");
                    println!("   Sites: {}", config.sites.len());
                    println!("   Blackwall: {}", config.blackwall.block_meta);
                }
                Err(e) => {
                    println!("   Status: ❌ Error: {e}");
                }
            }
            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("✅ Health check complete!");
            Ok(())
        }
        Some(Command::Version) => {
            println!("dromedary {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(Command::Server) | None => {
            tracing::info!("🐪 loading config from {}", cli.config.display());
            let config = DromedaryConfig::from_file(&cli.config)?;
            tracing::info!(
                "🐪 Dromedary v{} — {} sites, blackwall={}",
                env!("CARGO_PKG_VERSION"),
                config.sites.len(),
                config.blackwall.block_meta,
            );
            let server = Dromedary::new(config);
            server.run().await
        }
    }
}
