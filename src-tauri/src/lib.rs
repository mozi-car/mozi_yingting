pub mod commands;
pub mod core;
pub mod drivers;
pub mod events;
mod bridge;
mod hardware;

use std::sync::Mutex;
use percent_encoding::percent_decode_str;
use tauri::{AppHandle, Emitter, Manager, RunEvent};

fn local_resource_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("local-resource")
        .register_uri_scheme_protocol("local-resource", |_ctx, request| {
            let raw = request.uri().path();
            let decoded = percent_decode_str(raw).decode_utf8_lossy();
            let mut file_path = decoded.to_string();
            // local-resource:///C:/... has an extra slash before the Windows drive.
            if cfg!(windows) && file_path.starts_with('/') && file_path.as_bytes().get(2) == Some(&b':') {
                file_path.remove(0);
            }
            let file_path = std::path::PathBuf::from(file_path);
            let content_type = match file_path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "html" => "text/html; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "js" | "mjs" => "text/javascript; charset=utf-8",
                "json" => "application/json",
                "svg" => "image/svg+xml",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "woff" | "woff2" => "font/woff2",
                _ => "application/octet-stream",
            };
            match std::fs::read(&file_path) {
                Ok(body) => http::Response::builder()
                    .header("Content-Type", content_type)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(body)
                    .unwrap(),
                Err(_) => http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap(),
            }
        })
        .build()
}

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "婴听 YingTing",
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS
    })
}

/// 双击 .mytproject 文件启动时暂存的待打开工程路径
pub struct PendingOpen(pub Mutex<Option<String>>);

#[tauri::command]
fn take_pending_open(state: tauri::State<'_, PendingOpen>) -> Option<String> {
    state.0.lock().unwrap().take()
}

#[tauri::command]
fn read_local_resource(path: String) -> Result<Vec<u8>, String> {
    // local-resource:/// is used by plugin pages and README previews.
    // Decode the URL path, then read only the requested absolute file.
    let decoded = percent_decode_str(&path).decode_utf8().map_err(|e| e.to_string())?;
    let candidate = decoded.trim_start_matches('/').replace('/', std::path::MAIN_SEPARATOR_STR);
    let path = if cfg!(windows) && candidate.len() >= 2 && candidate.as_bytes()[1] == b':' {
        std::path::PathBuf::from(candidate)
    } else {
        std::path::PathBuf::from(format!("{}{}", std::path::MAIN_SEPARATOR, candidate))
    };
    std::fs::read(path).map_err(|e| e.to_string())
}

fn first_mytproject_arg(argv: &[String]) -> Option<String> {
    argv.iter()
        .find(|a| a.ends_with(".mytproject") && std::path::Path::new(a).exists())
        .cloned()
}

fn dispatch_open(app: &AppHandle, file: String) {
    let _ = app.emit(
        "yt-sidecar-event",
        serde_json::json!({ "channel": "open-project", "payload": file }),
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(f) = first_mytproject_arg(&std::env::args().collect::<Vec<_>>()) {
                app.state::<PendingOpen>().0.lock().unwrap().replace(f);
            }
            bridge::register(app);
            hardware::start(app.handle());
            Ok(())
        })
        .manage(PendingOpen(Mutex::new(None)))
        .plugin(local_resource_plugin())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(f) = first_mytproject_arg(&argv) {
                dispatch_open(app, f);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            bridge::rpc_invoke,
            bridge::rpc_emit,
            app_info,
            take_pending_open,
            read_local_resource
        ])
        .build(tauri::generate_context!())
        .expect("error while building yingting")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                if let Some(bridge) =
                    app_handle.try_state::<std::sync::Arc<bridge::SidecarBridge>>()
                {
                    bridge.shutdown();
                }
            }
        });
}
