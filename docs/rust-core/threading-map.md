# Threading and Lifecycle Map

## Current threads/processes

| Execution unit | Owner | Work | Shutdown today | Rust Core destination |
|---|---|---|---|---|
| Tauri main thread | `src-tauri` | window, commands, bridge state | Tauri RunEvent::Exit | Core runtime host |
| Sidecar Node process | `bridge.rs` child | all Node business orchestration | JSON shutdown + child kill | temporary compatibility only |
| Sidecar RPC reader | `bridge.rs` thread | stdout line parsing and pending response routing | ends with child stdout | Core command/event adapter |
| Node RPC event loop | `rpc.ts` stdin readline | handler dispatch and response serialization | stdin close/shutdown | Tauri command/event layer |
| Script Worker | `workerClient.ts` | user script/test execution | worker stop/exit/error | `core::task` + plugin compatibility |
| SOME/IP worker process | `vsomeip/index.ts` / `worker.ts` | client/router compatibility and callbacks | stop/kill | `core::someip` task supervisor |
| Native driver workers | Rust N-API crates | receive/cyclic/callback loops | module-specific stop/join | Core driver task handles |
| Vector/CAN/LIN callbacks | native DLL callback boundary | vendor event delivery | callback unregister/drop | typed driver event source |
| Windows PnP monitor | `hardware.rs` thread | USB arrival/removal | channel/registration Drop | `core::discovery` |
| Trace worker | `ostrace/worker.ts` | trace parsing/streaming | worker stop | `core::trace` or UI-side parser |

## Risks

1. Node and Rust can both own lifecycle state during dual-run migration; every handle needs one owner and an explicit close path.
2. Tauri `spawn_blocking` currently waits synchronously for sidecar RPC with a 30-second timeout; high-rate or long-running operations must not use this path.
3. Worker event handlers can outlive a NodeItem. Cancellation tokens must be propagated before Rust Core owns the worker.
4. Vendor callbacks must never call Tauri directly. They should enqueue typed events into Core.
5. Shutdown must stop periodic tasks, unregister callbacks, close transports, join workers, then terminate compatibility processes.

## Target runtime primitives

- `CancellationToken` per device/session/task;
- `JoinSet` or equivalent supervisor for Core workers;
- bounded `tokio::mpsc`/crossbeam channels for frame flow;
- `oneshot` correlation for request/response;
- `broadcast` shutdown signal;
- explicit `DeviceHandle`, `TransportHandle` and `Subscription` ownership.
