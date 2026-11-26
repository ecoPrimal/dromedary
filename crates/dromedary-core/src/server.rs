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

use axum::body::Bytes;
use crate::config::{DromedaryConfig, SiteBackend};

/// Extract hostname from request headers.
/// Real client address of the connection, stamped from the socket (never from request headers).
#[derive(Clone, Copy)]
struct ClientIp(std::net::IpAddr);

fn extract_host(req: &Request) -> String {
    req.headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

/// HTTP server — serves content directly on :80.
/// When TLS certs exist, this becomes a redirect to HTTPS.
/// Until then, the organism radiates as grey shadow over HTTP.
pub async fn serve_http_redirect(
    addr: SocketAddr,
    config: Arc<DromedaryConfig>,
) -> anyhow::Result<()> {
    let config_for_router = config.clone();

    let app = Router::new()
        .fallback(move |axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<SocketAddr>, req: Request| {
            let cfg = config_for_router.clone();
            async move {
                let host = extract_host(&req);
                route_request(host, req, cfg, Some(peer.ip())).await
            }
        });

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("🐪 HTTP server listening on {addr} (static membrane)");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

/// HTTPS server — TLS termination + host-based routing.
///
/// Loads certs from cert_dir, terminates TLS via rustls, routes by SNI hostname.
/// DNA: beardog-acme (crypto), songbird-tls (handshake), riboCypher (keys).
pub async fn serve_https(
    addr: SocketAddr,
    config: Arc<DromedaryConfig>,
) -> anyhow::Result<()> {
    use tokio_rustls::TlsAcceptor;

    let cert_dir = &config.cert_dir;

    // Try to load TLS certs — use first available domain cert
    let tls_config = {
        let mut loaded = false;
        let mut server_config = None;

        // Collect all hostnames that need certs
        let domains: Vec<String> = config.sites.iter()
            .flat_map(|s| s.hostnames.iter())
            .filter(|h| *h != "*" && !h.starts_with("*."))
            .map(|h| h.to_string())
            .collect();

        // Try loading cert for each domain
        for domain in &domains {
            let cert_path = cert_dir.join(format!("{domain}.crt"));
            let key_path = cert_dir.join(format!("{domain}.key"));
            if cert_path.exists() && key_path.exists() {
                match crate::tls::make_tls_config(&cert_path, &key_path) {
                    Ok(cfg) => {
                        tracing::info!("🐪 TLS loaded: {domain}");
                        server_config = Some(cfg);
                        loaded = true;
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("🐪 TLS cert error for {domain}: {e}");
                    }
                }
            }
        }

        if !loaded {
            tracing::warn!("🐪 No TLS certs found in {} — HTTPS serving plain TCP", cert_dir.display());
        }
        server_config
    };

    let config_for_router = config.clone();
    let app = Router::new()
        .fallback(move |req: Request| {
            let cfg = config_for_router.clone();
            async move {
                let host = extract_host(&req);
                // TLS loop inserts ClientIp; the plain fallback only carries axum's ConnectInfo.
                let client_ip = req.extensions().get::<ClientIp>().map(|c| c.0)
                    .or_else(|| req.extensions().get::<axum::extract::ConnectInfo<SocketAddr>>().map(|c| c.0.ip()));
                route_request(host, req, cfg, client_ip).await
            }
        });

    let listener = TcpListener::bind(addr).await?;

    if let Some(tls_cfg) = tls_config {
        let acceptor = TlsAcceptor::from(tls_cfg);
        tracing::info!("🐪 HTTPS server listening on {addr} (TLS LIVE — beardog crypto)");

        loop {
            let (stream, peer) = listener.accept().await?;
            let acceptor = acceptor.clone();
            let app = app.clone();
            tokio::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let io = hyper_util::rt::TokioIo::new(tls_stream);
                        let service = hyper::service::service_fn(move |mut req: hyper::Request<hyper::body::Incoming>| {
                            let app = app.clone();
                            req.extensions_mut().insert(ClientIp(peer.ip()));
                            async move {
                                use tower::ServiceExt;
                                app.oneshot(req).await
                            }
                        });
                        if let Err(e) = hyper_util::server::conn::auto::Builder::new(
                            hyper_util::rt::TokioExecutor::new()
                        ).serve_connection(io, service).await {
                            tracing::debug!("🐪 TLS conn error from {peer}: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::debug!("🐪 TLS handshake failed from {peer}: {e}");
                    }
                }
            });
        }
    } else {
        tracing::info!("🐪 HTTPS server listening on {addr} (no certs — plain fallback)");
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    }

    Ok(())
}

/// Route a request to the correct backend based on hostname.
async fn route_request(
    host: String,
    req: Request,
    config: Arc<DromedaryConfig>,
    client_ip: Option<std::net::IpAddr>,
) -> Response {
    let hostname = host.split(':').next().unwrap_or(&host);

    let Some(site) = config.find_site(hostname) else {
        tracing::debug!("🐪 no site for host={hostname}");
        return (StatusCode::NOT_FOUND, format!("unknown host: {hostname}")).into_response();
    };

    let path = req.uri().path().to_string();
    let method = match req.method().clone() {
        axum::http::Method::GET => reqwest::Method::GET,
        axum::http::Method::POST => reqwest::Method::POST,
        axum::http::Method::PUT => reqwest::Method::PUT,
        axum::http::Method::DELETE => reqwest::Method::DELETE,
        axum::http::Method::PATCH => reqwest::Method::PATCH,
        axum::http::Method::OPTIONS => reqwest::Method::OPTIONS,
        axum::http::Method::HEAD => reqwest::Method::HEAD,
        _ => reqwest::Method::GET,
    };

    let content_type = req.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // The visitor's own User-Agent goes on to the backend. reqwest sends none of its own, so without this
    // every visitor reaches the backend with no User-Agent and a crawler cannot be told from a browser.
    let user_agent = req.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(b) if !b.is_empty() => Some(b),
        _ => None,
    };

    // Check sub-routes first
    for route in &site.routes {
        let route_path = route.path.trim_end_matches('*');
        if path.starts_with(route_path) {
            return serve_backend(&route.backend, &path, client_ip, method, body_bytes, content_type, user_agent, Some(hostname)).await;
        }
    }

    serve_backend(&site.backend, &path, client_ip, method, body_bytes, content_type, user_agent, Some(hostname)).await
}

/// Serve a request against a specific backend.
#[allow(clippy::too_many_arguments)]
async fn serve_backend(
    backend: &SiteBackend,
    path: &str,
    client_ip: Option<std::net::IpAddr>,
    method: reqwest::Method,
    body: Option<Bytes>,
    content_type: Option<String>,
    user_agent: Option<String>,
    original_host: Option<&str>,
) -> Response {
    match backend {
        SiteBackend::Redirect { target } => {
            Redirect::permanent(target).into_response()
        }

        SiteBackend::Static { root, index } => {
            let clean_path = path.trim_start_matches('/');
            let idx = index.as_deref().unwrap_or("index.html");
            let file_path = if clean_path.is_empty() {
                root.join(idx)
            } else {
                root.join(clean_path)
            };

            // Try exact path first, then directory/index.html, then path.html
            let candidates = [
                file_path.clone(),
                file_path.join(idx),
                root.join(format!("{clean_path}.html")),
            ];

            let mut found = None;
            for candidate in &candidates {
                if let Ok(contents) = tokio::fs::read(candidate).await {
                    found = Some((candidate.clone(), contents));
                    break;
                }
            }

            match found {
                Some((resolved, contents)) => {
                    let mime = mime_from_path(&resolved);
                    (
                        StatusCode::OK,
                        [("content-type", mime)],
                        contents,
                    ).into_response()
                }
                None => {
                    (StatusCode::NOT_FOUND, "not found").into_response()
                }
            }
        }

        SiteBackend::Proxy { upstream, strip_prefix } => {
            let mut target_path = path.to_string();
            if let Some(prefix) = strip_prefix {
                target_path = target_path.strip_prefix(prefix.as_str())
                    .unwrap_or(&target_path)
                    .to_string();
                if !target_path.starts_with('/') {
                    target_path = format!("/{target_path}");
                }
            }
            let scheme = if upstream.starts_with("http://") || upstream.starts_with("https://") { "" } else { "http://" };
            let url = format!("{scheme}{upstream}{target_path}");
            tracing::debug!("🐪 proxy {method} → {url}");

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let mut upstream_req = client.request(method.clone(), &url);
            if let Some(ip) = client_ip {
                upstream_req = upstream_req.header("X-Forwarded-For", ip.to_string());
            }
            if let Some(host) = original_host {
                upstream_req = upstream_req.header("Host", host);
                upstream_req = upstream_req.header("X-Forwarded-Host", host);
            }
            if let Some(ct) = content_type {
                upstream_req = upstream_req.header("Content-Type", ct);
            }
            if let Some(ua) = user_agent {
                upstream_req = upstream_req.header("User-Agent", ua);
            }
            if let Some(b) = body {
                upstream_req = upstream_req.body(b);
            }
            match upstream_req.send().await {
                Ok(resp) => {
                    let status = StatusCode::from_u16(resp.status().as_u16())
                        .unwrap_or(StatusCode::BAD_GATEWAY);
                    let resp_ct = resp.headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("application/octet-stream")
                        .to_string();
                    match resp.bytes().await {
                        Ok(body) => {
                            (status, [("content-type", resp_ct)], body).into_response()
                        }
                        Err(e) => {
                            tracing::warn!("🐪 proxy body error: {e}");
                            (StatusCode::BAD_GATEWAY, format!("proxy body error: {e}")).into_response()
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("🐪 proxy connect error: {e}");
                    (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response()
                }
            }
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
