mod bridge;

use tauri::Manager;

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "婴听 YingTing",
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            bridge::register(app);
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![bridge::rpc_invoke, bridge::rpc_emit, app_info])
        .build(tauri::generate_context!())
        .expect("error while building yingting")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(bridge) =
                    app_handle.try_state::<std::sync::Arc<bridge::SidecarBridge>>()
                {
                    bridge.shutdown();
                }
            }
        });
}
