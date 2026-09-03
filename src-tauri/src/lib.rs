mod bridge;
mod hardware;

use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent};

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
            take_pending_open
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
