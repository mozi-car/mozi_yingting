# Event Map

## Current event paths

### Sidecar → Tauri → renderer

```text
Node rpc.emit(channel, ...args)
  → stdout JSON { method: "emit", params: [channel, ...args] }
  → bridge.rs reader thread
  → Tauri event "yt-sidecar-event"
  → renderer shim restores channel/payload
```

The Rust bridge preserves variadic payloads by forwarding `params[1..]` as an array. This compatibility behavior is part of the current contract.

### Hardware discovery

```text
Windows PnP callback
  → hardware.rs mpsc channel
  → HardwareRegistry
  → Tauri events: hardware-added / hardware-removed
  → Vue hardware tree
```

`hardware.rs` currently discovers USB interface paths only; it does not open a driver or configure a bus.

### Device and frame events

```text
Rust native callback / read worker
  → TypeScript native wrapper
  → EventEmitter / driver class event
  → NodeItem / diagnostic transport
  → rpc.emit
  → renderer/plugin
```

### Worker events

```text
worker_threads Worker.postMessage
  → workerClient pending request/event map
  → NodeItem HandlerMap
  → device operation or event
  → Worker response / event
```

## Target EventBus

```rust
pub enum CoreEvent {
    DeviceAdded(DeviceInfo),
    DeviceRemoved(DeviceId),
    DeviceState(DeviceState),
    Frame(FrameEvent),
    Diagnostic(DiagnosticEvent),
    Plugin(PluginEvent),
    Trace(TraceEvent),
    Error(CoreError),
}
```

The EventBus should provide:

- typed subscriptions with cancellation;
- bounded queues for high-rate frames;
- batch/coalescing policy before Tauri emission;
- correlation IDs for request/response diagnostics;
- shutdown broadcast for all workers;
- one adapter that emits legacy channel names during migration.

## Event migration rule

Do not send every CAN/LIN/XCP frame through Tauri IPC. Rust should aggregate or filter in Core, then emit batches, diagnostic responses and state changes. The legacy RPC/event adapter remains until every consumer is migrated.
