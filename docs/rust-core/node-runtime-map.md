# Node Runtime Map

## Entry and process boundary

| Module | Current responsibility | Boundary | Rust Core destination | Migration priority |
|---|---|---|---|---|
| `src/main/index.ts` | sidecar initialization, logging, i18n, Casdoor, store bridge, RPC startup | process startup | `core::runtime`, `core::config`, `commands` | P0 |
| `src/main/rpc.ts` | line-delimited JSON protocol, handler registry, invoke/response, emit | stdin/stdout with Tauri bridge | `commands::rpc` compatibility adapter, then native Tauri commands | P0 |
| `src/main/native.ts` | loads `<name>.node` from sidecar native directory | N-API module loading | `drivers` adapter or temporary compatibility boundary | P1 |
| `src/main/ipc/*` | Electron IPC-compatible channel handlers for filesystem, UDS, plugins, serial, dialogs and project data | RPC channel names | typed `commands` calling Core services | P0/P1 |
| `src/main/nodeItem.ts` | device object, CAN/LIN/DoIP/Serial/SOME-IP orchestration, diagnostics, scripts, logs | high-level domain object | `DeviceManager`, `Transport`, `IsoTp`, `Uds`, `Doip`, `Someip` | P1 |
| `src/main/workerClient.ts` | script worker thread, test execution, script-to-host RPC, test events | `worker_threads` messages | `core::task`, `core::plugin`, `core::event_bus` | P1 |
| `src/main/pluginCilent.ts` | plugin lifecycle, plugin event logging, plugin calls through NodeClass | plugin compatibility API | `plugin::compat` first; native plugin API later | P2 |
| `src/main/docan/*` | CAN device orchestration, CAN-TP and UDS-facing CAN adapter | Rust N-API drivers | `transport::can`, `isotp`, `uds` | P1 |
| `src/main/dolin/*` | LIN device orchestration and LIN-TP | Rust N-API drivers | `transport::lin`, `isotp` | P1 |
| `src/main/doip/*` | TCP/UDP DoIP discovery and diagnostic transport | Node `net`/`dgram` | `transport::doip` | P2 |
| `src/main/serial/*` | serial options and serialport-compatible API | Rust serial N-API | `transport::serial` | P1 |
| `src/main/vsomeip/*` | SOME/IP facade, worker process, event/service compatibility | Rust SOME/IP N-API | `someip` | P2 |
| `src/main/replay/*` | ASC/BLF readers and replay sources | file I/O | `replay` | P2 |
| `src/main/ostrace/*` | trace parsing and worker | worker/files | `trace` or keep UI-side parser | P3 |
| `src/main/worker/*` | UDS/CANopen/script helper workers | worker threads and JS APIs | split by domain; plugin compatibility first | P2/P3 |

## High-frequency paths

1. CAN/LIN receive → driver callback/read → TypeScript EventEmitter → `nodeItem` → RPC emit → renderer.
2. Diagnostic request → `NodeClass`/`UDSTester` → CAN-TP/LIN-TP/DoIP → driver → response matching and timeout handling.
3. Script worker → `workerClient` message → NodeItem handler → device operation → worker response/event.
4. SOME/IP worker → Rust-backed client → callback → Node compatibility facade → renderer/plugin event.
5. Replay reader → frame stream → same device/logging/event path as live traffic.

The first Rust Core transport implementation must keep receive data inside Rust until a batch/event boundary. One Tauri IPC call per frame is prohibited.

## Synchronous/asynchronous boundaries

- Tauri `rpc_invoke` is synchronous from the caller's promise perspective but executes the sidecar call in a Rust blocking task.
- Sidecar stdin/stdout RPC is request/response and has a 30-second bridge timeout.
- Node worker threads are asynchronous and message-based.
- Device callbacks and EventEmitter handlers are asynchronous and may outlive an RPC request.
- Plugin calls may start independent workers and must not block the Core runtime thread.
