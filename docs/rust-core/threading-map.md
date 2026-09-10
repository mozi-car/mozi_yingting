# Threading and Lifecycle Map

## Current process/thread topology

```text
Tauri process
├─ WebView renderer
│  └─ invoke/events
├─ SidecarBridge stdout reader thread
├─ Sidecar child: node out/sidecar/index.cjs
│  ├─ stdin readline/RPC event loop
│  ├─ synchronous N-API driver calls
│  ├─ native TSFN callbacks
│  ├─ worker_threads: one UdsTester per script/test
│  ├─ ostrace workers and timers
│  └─ SOME/IP fork(vsomeip.js)
│     └─ Rust SOME/IP receive thread
├─ Windows PnP monitor thread
└─ RunEvent::Exit -> bridge.shutdown()
```

## Concrete ownership table

| Unit | Producer | Consumer | Message/lifecycle | Risk |
|---|---|---|---|---|
| SidecarBridge reader | `src-tauri/src/bridge.rs` | Tauri event/pending map | stdout JSON lines; ends on child exit | pending calls can timeout when child dies |
| Sidecar RPC loop | `src/main/rpc.ts` | `ipcMain`-style handlers | `invoke`, `emit`, `ready`, `shutdown` | generic JSON loses type/cancel semantics |
| Script worker | `src/main/workerClient.ts` | `NodeClass` handlers | `rpc_response`, `event`, stdout/stderr | worker can outlive device unless cancellation propagates |
| Native CAN callback | Rust N-API TSFN | TS adapter/EventEmitter | frame/status/callback | high-rate IPC amplification |
| LIN schedule worker | LIN adapters | `LinBase`, TP and logs | schedule/frame/pending response | stop/reopen ordering |
| SOME/IP child | `vsomeip/index.ts` | parent client | process messages and callbacks | second process complicates shutdown |
| PnP monitor | `src-tauri/hardware.rs` | Tauri events | mpsc `PnpEvent` | discovery only, no driver lifecycle |
| Replay worker | `ostrace/worker.ts`, BLF/ASC | logs/UI | file stream/progress/end | backpressure and cancellation |
| Plugin worker | `workerClient`/NodeClass | PluginClient | `plugin.<method>`, `pluginEvent` | compatibility contract |

## Concrete event/message names

### Sidecar/Tauri

- `ready`
- `emit`
- `yt-sidecar-event`
- `hardware-added`
- `hardware-removed`
- `open-project`

### Device/protocol

- `can-frame`
- `lin-frame`
- `data`
- `serial-message`
- `someip-frame`
- `someip-service-valid`
- `subscription`
- `subscription_status`
- `watchdog`
- `trace`
- `error`
- `close`, `closed`, `exit`

### Worker/plugin

- `rpc_response`
- `event`
- `pluginEvent`
- `__canMsg`
- `__linMsg`
- `__serialMsg`
- `__someipMsg`
- `__someipServiceValid`
- `__varUpdate`
- `__varFc`
- `__keyDown`
- `__end`

## Target Rust lifecycle

Every Core task/device must own:

```text
CancellationToken
  -> stop receive/schedule loop
  -> unregister vendor callback
  -> close transport
  -> join worker
  -> publish DeviceState::Closed/Error
```

The target supervisor should use bounded channels, typed request/response correlation and a broadcast shutdown signal. Vendor callbacks enqueue into Core; they never call Tauri or Vue directly.

## High-frequency paths

CAN/LIN/Serial/XCP data must not use one Tauri IPC call per frame:

```text
Driver callback/read
  -> bounded ring/channel
  -> Rust TP/DAQ/aggregation/filter
  -> batch FrameEvent or diagnostic response
  -> Tauri event
  -> Vue/plugin
```

## Verification per phase

- Core task tests: cancellation, finish, cancel-all, join ordering.
- Device tests: add/remove/open/close/state events and duplicate arrival handling.
- Transport tests: open/send/receive/subscribe/close and timeout/error propagation.
- Dual-run tests: Node and Rust receive identical scripted frames and compare events/errors.
- Shutdown tests: repeat open/close/reopen and assert no pending task/callback remains.
