use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};

static REQ_ID: AtomicU64 = AtomicU64::new(1);

pub struct SidecarBridge {
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    pending: Mutex<std::collections::HashMap<u64, Sender<Result<Value, String>>>>,
    app: AppHandle,
}

fn sidecar_command_path() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    // tauri dev 的 cwd 是 src-tauri/，npm 直跑时是项目根
    let root = if cwd.file_name().and_then(|n| n.to_str()) == Some("src-tauri") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd
    };
    root.join("out/sidecar/index.cjs")
}

/// 优先使用打包资源内路径（release），否则回退开发路径
fn resolve_runtime(app: &AppHandle) -> (std::path::PathBuf, std::path::PathBuf) {
    let res = app.path().resource_dir();
    eprintln!("[bridge] resource_dir = {:?}", res);
    if let Ok(res) = res {
        #[cfg(windows)]
        let node = res.join("bin/node.exe");
        #[cfg(not(windows))]
        let node = res.join("bin/node");
        let cjs = res.join("sidecar/index.cjs");
        eprintln!(
            "[bridge] node {:?} exists={}, cjs {:?} exists={}",
            node,
            node.exists(),
            cjs,
            cjs.exists()
        );
        if node.exists() && cjs.exists() {
            return (node, cjs);
        }
    }
    let dev = sidecar_command_path();
    eprintln!("[bridge] fallback dev cjs = {:?}", dev);
    (std::path::PathBuf::from("node"), dev)
}

fn sidecar_command(app: &AppHandle) -> Command {
    let (node, cjs) = resolve_runtime(app);
    let mut cmd = Command::new(&node);
    cmd.arg(&cjs);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::inherit());
    cmd.stdin(Stdio::piped());
    cmd.env(
        "YT_RESOURCES",
        cjs.parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(".")),
    );
    cmd
}

impl SidecarBridge {
    pub fn spawn(app: AppHandle) -> std::io::Result<Arc<Self>> {
        let mut child = sidecar_command(&app).spawn()?;
        let stdin = child.stdin.take().expect("sidecar stdin");
        let stdout = child.stdout.take().expect("sidecar stdout");
        let bridge = Arc::new(Self {
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(Some(stdin)),
            pending: Mutex::new(Default::default()),
            app,
        });

        let reader = Arc::clone(&bridge);
        thread::spawn(move || {
            let buf_reader = BufReader::new(stdout);
            for line in buf_reader.lines().map_while(Result::ok) {
                reader.handle_stdout_line(&line);
            }
        });

        Ok(bridge)
    }

    fn handle_stdout_line(&self, line: &str) {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return,
        };
        match v.get("method").and_then(|m| m.as_str()) {
            Some("emit") => {
                let params = v.get("params").cloned().unwrap_or_else(|| json!([]));
                let channel = params.get(0).and_then(|c| c.as_str()).unwrap_or("");
                let payload = params.get(1).cloned().unwrap_or(Value::Null);
                let _ = self.app.emit(
                    "yt-sidecar-event",
                    json!({ "channel": channel, "payload": payload }),
                );
            }
            Some("ready") => {
                log::info!("sidecar ready");
            }
            _ => {
                if let Some(id) = v.get("id").and_then(|i| i.as_u64()) {
                    let sender = self.pending.lock().unwrap().remove(&id);
                    if let Some(tx) = sender {
                        if let Some(err) = v.get("error") {
                            let _ = tx.send(Err(err.to_string()));
                        } else {
                            let result = v.get("result").cloned().unwrap_or(Value::Null);
                            let _ = tx.send(Ok(result));
                        }
                    }
                }
            }
        }
    }

    /// Invoke a channel on the sidecar, returning the JSON result.
    pub fn invoke(&self, channel: &str, args: &Value) -> Result<Value, String> {
        let id = REQ_ID.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);

        let req = json!({ "id": id, "method": "invoke", "params": [channel, args] });
        let line = req.to_string() + "\n";
        {
            let mut stdin = self.stdin.lock().unwrap();
            let stream = stdin
                .as_mut()
                .ok_or_else(|| "sidecar stdin closed".to_string())?;
            stream
                .write_all(line.as_bytes())
                .map_err(|e| format!("sidecar write: {e}"))?;
            stream.flush().map_err(|e| format!("sidecar flush: {e}"))?;
        }

        rx.recv_timeout(std::time::Duration::from_secs(30))
            .map_err(|e| format!("sidecar invoke timeout: {e}"))?
    }

    pub fn emit(&self, channel: &str, payload: Value) {
        let req = json!({ "method": "emit", "params": [channel, payload] });
        let line = req.to_string() + "\n";
        let mut stdin = self.stdin.lock().unwrap();
        if let Some(stream) = stdin.as_mut() {
            let _ = stream.write_all(line.as_bytes());
            let _ = stream.flush();
        }
    }

    pub fn shutdown(&self) {
        let req = json!({ "method": "shutdown" });
        let line = req.to_string() + "\n";
        let mut stdin = self.stdin.lock().unwrap();
        if let Some(stream) = stdin.as_mut() {
            let _ = stream.write_all(line.as_bytes());
            let _ = stream.flush();
        }
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
        }
    }
}

#[tauri::command]
pub async fn rpc_invoke(
    bridge: State<'_, Arc<SidecarBridge>>,
    channel: String,
    args: Value,
) -> Result<Value, String> {
    // 原生对话框在 Rust 侧处理（sidecar 无 UI）
    if let Some(v) = native_dialog(&bridge.app, &channel, &args) {
        return Ok(v);
    }
    let bridge = Arc::clone(&bridge);
    tauri::async_runtime::spawn_blocking(move || bridge.invoke(&channel, &args))
        .await
        .map_err(|e| e.to_string())?
}

/// 原生对话框（对齐 Electron dialog 返回结构）
fn native_dialog(app: &tauri::AppHandle, channel: &str, args: &Value) -> Option<Value> {
    use tauri_plugin_dialog::DialogExt;
    match channel {
        "ipc-show-open-dialog" => {
            let props = args.get(0).and_then(|v| v.as_object());
            let mut picker = app.dialog().file();
            let mut multi = false;
            if let Some(p) = props {
                if let Some(pros) = p.get("properties").and_then(|v| v.as_array()) {
                    multi = pros
                        .iter()
                        .any(|v| v.as_str() == Some("multiSelections"));
                }
                if let Some(title) = p.get("title").and_then(|v| v.as_str()) {
                    picker = picker.set_title(title);
                }
                if let Some(filters) = p.get("filters").and_then(|v| v.as_array()) {
                    if let Some(f) = filters.first().and_then(|v| v.as_object()) {
                        if let Some(name) = f.get("name").and_then(|v| v.as_str()) {
                            picker = picker.add_filter(name, &["*"]);
                        }
                    }
                }
            }
            if multi {
                let files = picker.blocking_pick_files();
                Some(json!({ "canceled": files.is_none(), "filePaths": files.unwrap_or_default().iter().map(|f| f.clone().into_path().map(|p| p.display().to_string()).unwrap_or_default()).collect::<Vec<_>>() }))
            } else {
                let file = picker.blocking_pick_file();
                Some(json!({ "canceled": file.is_none(), "filePaths": vec![file.map(|f| f.into_path().map(|p| p.display().to_string()).unwrap_or_default()).unwrap_or_default()] }))
            }
        }
        "ipc-show-save-dialog" => {
            let props = args.get(0).and_then(|v| v.as_object());
            let mut picker = app.dialog().file();
            if let Some(p) = props {
                if let Some(title) = p.get("title").and_then(|v| v.as_str()) {
                    picker = picker.set_title(title);
                }
                if let Some(dp) = p.get("defaultPath").and_then(|v| v.as_str()) {
                    picker = picker.set_file_name(dp);
                }
                if let Some(filters) = p.get("filters").and_then(|v| v.as_array()) {
                    if let Some(f) = filters.first().and_then(|v| v.as_object()) {
                        if let Some(exts) = f.get("extensions").and_then(|v| v.as_array()) {
                            let exts: Vec<&str> = exts.iter().filter_map(|v| v.as_str()).collect();
                            let name = f
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("file");
                            picker = picker.add_filter(name, &exts);
                        }
                    }
                }
            }
            let file = picker.blocking_save_file();
            Some(json!({ "canceled": file.is_none(), "filePath": file.map(|f| f.into_path().map(|p| p.display().to_string()).unwrap_or_default()) }))
        }
        "ipc-show-message-box" => {
            let props = args.get(0).and_then(|v| v.as_object());
            let title = props.and_then(|p| p.get("title").and_then(|v| v.as_str())).unwrap_or("yingting");
            let message = props.and_then(|p| p.get("message").and_then(|v| v.as_str())).unwrap_or("").to_string();
            let _ = app.dialog().message(message).title(title).blocking_show();
            Some(json!({ "response": 1 }))
        }
        "icp-show-error-box" => {
            let title = args.get(0).and_then(|v| v.as_str()).unwrap_or("Error").to_string();
            let content = args.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let _ = app.dialog().message(content).title(title).blocking_show();
            Some(json!({ "response": 1 }))
        }
        _ => None,
    }
}

#[tauri::command]
pub fn rpc_emit(bridge: State<'_, Arc<SidecarBridge>>, channel: String, payload: Value) {
    bridge.emit(&channel, payload);
}

pub fn register(app: &mut tauri::App) {
    let handle = app.handle().clone();
    match SidecarBridge::spawn(handle) {
        Ok(bridge) => {
            app.manage(bridge);
        }
        Err(e) => {
            eprintln!("[bridge] failed to start sidecar: {e}");
            log::error!("failed to start sidecar: {e}");
        }
    }
}
