//! Geisterhand: In-process input fuzzer for Perry UI applications.
//!
//! Embeds a lightweight HTTP server that exposes registered widget callbacks
//! and allows programmatic input firing (click, type, slide, toggle) and
//! chaos-mode random fuzzing.

use std::sync::atomic::{AtomicBool, Ordering};

mod server;
mod chaos;

static RUNNING: AtomicBool = AtomicBool::new(false);

/// Start the geisterhand HTTP server on a background thread.
/// Called from compiled binary's main() when --enable-geisterhand was used.
#[no_mangle]
pub extern "C" fn perry_geisterhand_start(port: i32) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return; // Already running
    }
    let port = if port <= 0 { 7676 } else { port as u16 };
    std::thread::spawn(move || {
        server::run_server(port);
    });
    eprintln!("[geisterhand] listening on http://127.0.0.1:{}", port);
}
