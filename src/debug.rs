//! Debug request/response logging to individual files.
//!
//! Disabled by default. Enable with:
//! 1. Compile feature: `features = ["debug-logging"]`
//! 2. Set env var: `YTMAPI_DEBUG_DIR=/path/to/logs`
//!
//! Each request writes a timestamped file containing method, URL, headers,
//! body, and response details.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

static REQUEST_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Log a POST request and its response to a file in `YTMAPI_DEBUG_DIR`.
/// No-op if the env var is not set.
pub fn log_post_request(
    url: &str,
    headers: &[(&str, std::borrow::Cow<'_, str>)],
    body: &serde_json::Value,
    params: &[(&str, std::borrow::Cow<'_, str>)],
    response_status: u16,
    response_headers: &[(String, String)],
    response_body: &str,
) {
    let Some(dir) = std::env::var_os("YTMAPI_DEBUG_DIR") else {
        return;
    };
    let dir = Path::new(&dir);
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("[ytmapi-debug] Failed to create log dir: {e}");
        return;
    }
    let seq = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("{ts}_{seq:04}_POST.log");
    let path = dir.join(&filename);

    let mut content = String::new();
    content.push_str(&format!("=== REQUEST ===\n"));
    content.push_str(&format!("Method: POST\n"));
    content.push_str(&format!("URL: {url}\n"));
    if !params.is_empty() {
        content.push_str("Params:\n");
        for (k, v) in params {
            content.push_str(&format!("  {k}: {v}\n"));
        }
    }
    content.push_str("Headers:\n");
    for (k, v) in headers {
        content.push_str(&format!("  {k}: {v}\n"));
    }
    content.push_str(&format!(
        "Body:\n{}\n",
        serde_json::to_string_pretty(body).unwrap_or_default()
    ));
    content.push_str(&format!("\n=== RESPONSE ===\n"));
    content.push_str(&format!("Status: {response_status}\n"));
    content.push_str("Headers:\n");
    for (k, v) in response_headers {
        content.push_str(&format!("  {k}: {v}\n"));
    }
    content.push_str(&format!("Body:\n{response_body}\n"));

    match fs::File::create(&path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(content.as_bytes()) {
                eprintln!("[ytmapi-debug] Failed to write {filename}: {e}");
            }
        }
        Err(e) => eprintln!("[ytmapi-debug] Failed to create {filename}: {e}"),
    }
}

/// Log a GET request and its response to a file in `YTMAPI_DEBUG_DIR`.
/// No-op if the env var is not set.
pub fn log_get_request(
    url: &str,
    headers: &[(&str, std::borrow::Cow<'_, str>)],
    params: &[(&str, std::borrow::Cow<'_, str>)],
    response_status: u16,
    response_headers: &[(String, String)],
    response_body: &str,
) {
    let Some(dir) = std::env::var_os("YTMAPI_DEBUG_DIR") else {
        return;
    };
    let dir = Path::new(&dir);
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("[ytmapi-debug] Failed to create log dir: {e}");
        return;
    }
    let seq = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("{ts}_{seq:04}_GET.log");
    let path = dir.join(&filename);

    let mut content = String::new();
    content.push_str(&format!("=== REQUEST ===\n"));
    content.push_str(&format!("Method: GET\n"));
    content.push_str(&format!("URL: {url}\n"));
    if !params.is_empty() {
        content.push_str("Params:\n");
        for (k, v) in params {
            content.push_str(&format!("  {k}: {v}\n"));
        }
    }
    content.push_str("Headers:\n");
    for (k, v) in headers {
        content.push_str(&format!("  {k}: {v}\n"));
    }
    content.push_str(&format!("\n=== RESPONSE ===\n"));
    content.push_str(&format!("Status: {response_status}\n"));
    content.push_str("Headers:\n");
    for (k, v) in response_headers {
        content.push_str(&format!("  {k}: {v}\n"));
    }
    content.push_str(&format!("Body:\n{response_body}\n"));

    match fs::File::create(&path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(content.as_bytes()) {
                eprintln!("[ytmapi-debug] Failed to write {filename}: {e}");
            }
        }
        Err(e) => eprintln!("[ytmapi-debug] Failed to create {filename}: {e}"),
    }
}
