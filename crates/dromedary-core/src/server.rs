//! HTTP + HTTPS server loops.
//!
//! Port 80: redirects all traffic to HTTPS.
//! Port 443: TLS termination via rustls, host-based routing, reverse proxy + static serving.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Router;
use tokio::net::TcpListener;

use crate::config::{DromedaryConfig, SiteBackend};

/// Extract hostname from request headers.
fn extract_host(req: &Request) -> String {
    req.headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

/// HTTP redirect server — all requests on :80 → https://
pub async fn serve_http_redirect(
    addr: SocketAddr,
    _config: Arc<DromedaryConfig>,
) -> anyhow::Result<()> {
    let app = Router::new().fallback(|req: Request| async move {
        let host = extract_host(&req);
        let uri = req.uri().clone();
        let target = format!("https://{host}{uri}");
        Redirect::permanent(&target)
    });

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("🐪 HTTP redirect listening on {addr}");

    axum::serve(listener, app).await?;
    Ok(())
}

/// HTTPS server — TLS termination + host-based routing.
///
/// Phase 1: Serve static files for hud.primals.eco and data feeds.
/// Phase 2: Add reverse proxy for cellMembrane, nestGate, etc.
/// Phase 3: Full ACME certificate lifecycle.
pub async fn serve_https(
    addr: SocketAddr,
    config: Arc<DromedaryConfig>,
) -> anyhow::Result<()> {
    let config_for_router = config.clone();

    let app = Router::new()
        .fallback(move |req: Request| {
            let cfg = config_for_router.clone();
            async move {
                let host = extract_host(&req);
                route_request(host, req, cfg).await
            }
        });

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("🐪 HTTPS server listening on {addr} (TLS pending ACME bootstrap)");

    axum::serve(listener, app).await?;
    Ok(())
}

/// Route a request to the correct backend based on hostname.
async fn route_request(
    host: String,
    req: Request,
    config: Arc<DromedaryConfig>,
) -> Response {
    let hostname = host.split(':').next().unwrap_or(&host);

    let Some(site) = config.find_site(hostname) else {
        tracing::debug!("🐪 no site for host={hostname}");
        return (StatusCode::NOT_FOUND, format!("unknown host: {hostname}")).into_response();
    };

    let path = req.uri().path().to_string();

    // Check sub-routes first
    for route in &site.routes {
        let route_path = route.path.trim_end_matches('*');
        if path.starts_with(route_path) {
            return serve_backend(&route.backend, &path).await;
        }
    }

    serve_backend(&site.backend, &path).await
}

/// Serve a request against a specific backend.
async fn serve_backend(backend: &SiteBackend, path: &str) -> Response {
    match backend {
        SiteBackend::Redirect { target } => {
            Redirect::permanent(target).into_response()
        }

        SiteBackend::Static { root, index } => {
            let clean_path = path.trim_start_matches('/');
            let file_path = if clean_path.is_empty() {
                root.join(index.as_deref().unwrap_or("index.html"))
            } else {
                root.join(clean_path)
            };

            match tokio::fs::read(&file_path).await {
                Ok(contents) => {
                    let mime = mime_from_path(&file_path);
                    (
                        StatusCode::OK,
                        [("content-type", mime)],
                        contents,
                    ).into_response()
                }
                Err(_) => {
                    (StatusCode::NOT_FOUND, "not found").into_response()
                }
            }
        }

        SiteBackend::Proxy { upstream, .. } => {
            (
                StatusCode::BAD_GATEWAY,
                format!("proxy to {upstream} — not yet wired (Phase 2)"),
            ).into_response()
        }
    }
}

/// Guess MIME type from file extension.
fn mime_from_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("rss") => "application/rss+xml; charset=utf-8",
        Some("atom") => "application/atom+xml; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("wasm") => "application/wasm",
        Some("txt") => "text/plain; charset=utf-8",
        Some("toml") => "application/toml; charset=utf-8",
        _ => "application/octet-stream",
    }
}
