//! dromedary-static — static file serving with compression.
//!
//! DNA from: petalTongue (tower-http fs, gzip, brotli).
//!
//! Replaces `python3 -m http.server` with Rust.
//! Serves the 1,960 data files, RSS feed, HUD site.
//! Gzip + Brotli compression. Proper MIME types. ETags. Cache headers.

/// Placeholder — static serving is inline in dromedary-core::server for Phase 1.
/// This crate will grow to add:
///   - Directory listing (optional)
///   - ETag generation
///   - Cache-Control headers
///   - Gzip/Brotli pre-compression
///   - Range requests (for large files)

pub fn version() -> &'static str {
    "0.1.0"
}
