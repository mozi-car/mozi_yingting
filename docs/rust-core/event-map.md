# Event Map

> Event names below are current source names. Frequency is classified from the producer path; exact rates depend on bus/configuration.

## Tauri / sidecar events

| Event | Producer | Consumer | Payload | Frequency |
|---|---|---|---|---|
| `yt-sidecar-event` | `src-tauri/src/bridge.rs` | renderer shim/UI | `{channel, payload[]}` | every sidecar emit |
| `hardware-added` | `src-tauri/src/hardware.rs` PnP worker | renderer hardware store | `{device}` with path/VID/PID | device arrival only |
| `hardware-removed` | `src-tauri/src/hardware.rs` PnP worker | renderer hardware store | `{id}` | device removal only |
| `open-project` | Tauri single-instance callback | renderer main/store | project path | application launch |
| `ready` | `src/main/rpc.ts` | bridge reader/log | platform string | once per sidecar |
| `emit` | `src/main/rpc.ts` | bridge reader | channel plus variadic payload | channel-dependent |

## Device and protocol events

| Event | Producer | Consumer | Payload | Frequency |
|---|---|---|---|---|
| `can-frame` | `docan/base.ts` and driver adapters | `NodeItem`, logs, UDS/TP | CAN frame + timestamp | per received/transmitted frame |
| `lin-frame` | `dolin/base.ts` and LIN adapters | `NodeItem`, LIN-TP, scripts | LIN message/frame | per frame/schedule slot |
| `serial-message` / `data` | `serial/index.ts`, `serial/rust.ts` | NodeItem/scripts/UI | byte Buffer | per read chunk |
| `someip-frame` | `vsomeip/client.ts` callback | NodeItem/log/plugin | SOME/IP message | per message |
| `someip-service-valid` | vSomeIP availability callback | request waiters/UI | service/instance availability | service discovery changes |
| `subscription` | vSomeIP callback | plugin/UI | subscription info | subscription state changes |
| `subscription_status` | vSomeIP callback | plugin/UI | subscription status | subscribe/unsubscribe |
| `watchdog` | vSomeIP callback | plugin/UI | no payload | watchdog condition |
| `trace` | vSomeIP callback / routing manager | SOME/IP log | trace header/data | per trace message |
| `can-frame` / `lin-frame` worker aliases | `worker/uds.ts` | user script listeners | script-facing frame payload | per frame; high frequency |
| `__canMsg` | `worker/uds.ts` | worker script | internal CAN message | per frame |
| `__linMsg` | `worker/uds.ts` | worker script | internal LIN message | per frame |
| `__serialMsg` | `worker/uds.ts` | worker script | serial bytes | per read chunk |
| `__someipMsg` | `worker/uds.ts` | worker script | SOME/IP message | per message |
| `__someipServiceValid` | `worker/uds.ts` | worker script | availability | state change |
| `__varUpdate` / `__varFc` | worker variable layer | scripts/UI | variable value/control | variable update/control |
| `pluginEvent` | `worker/plugin SDK` | `PluginClient`, renderer plugin bus | `{name,data}` | plugin-defined |
| `error` | driver adapters, DoIP, serial, workerClient | logs/UI | Error | exceptional |
| `close` / `closed` / `exit` | device adapters, workers, RPC | lifecycle owners | optional reason/code | lifecycle |

## Protocol/internal events

- LIN scheduler events: schedule start/stop, power control, baud-rate change, diagnostic pending/read.
- CAN/LIN TP events: pending request, response timeout, transport error, session close.
- DoIP events: UDP vehicle identification, routing activation, alive check, TCP data, connection close/error.
- Replay events: frame output, progress, end, error, flush.
- CANopen worker events: `emergency`, `heartbeat`, `changeState`, `changeMode`, `changeDeviceId` and protocol-specific messages.

## IPC event producers and consumers

| Producer | Event/channel | Consumer | Payload / boundary |
|---|---|---|---|
| `ipc/uds.ts` | `ipc-send-can`, `ipc-update-can-period`, `ipc-send-someip-period`, `ipc-update-lin-signals` and related channels | renderer stores/views → Node domain objects | command request; response is RPC result |
| `ipc/var.ts` | `ipc-var-set`, `ipc-signal-set` | renderer data store → Node variable layer | variable/signal name and scalar/array value |
| `ipc/plugin.ts` | `ipc-plugin-exec`, `ipc-plugin-create/close` | renderer plugin SDK → `PluginClient` | method, params, plugin ID |
| `ipc/serialPort.ts` | `ipc-get-serial-devices`, `ipc-get-serial-port-list` | renderer hardware/serial UI | device list |
| `ipc/casdoor.ts` | `refreshToken`, `ipc-auto-login`, `ipc-authenticated-request` | renderer user store | auth/user request/response |
| `ipc/ostrace.ts` + `ostrace/worker.ts` | trace data/progress/error/end | UDS/ostrace renderer views | trace records and progress |
| `canmartix.ts` + `ipc/canmartix.ts` | parse/export response | project store | matrix JSON/file result |
| `python.ts` via CDD/ODX IPC | parser process result/error | project/UDS stores | parsed JSON/tester info |

## Protocol-internal event traceability

| Event | Producer path | Consumer path | Payload |
|---|---|---|---|
| `can-frame` | `docan/base.ts` adapter after driver read/callback | `log.ts`, `nodeItem.ts`, CAN-TP and worker listeners | `CanMessage` + timestamp |
| `lin-frame` | `dolin/base.ts` adapter/scheduler | LIN-TP, `nodeItem.ts`, worker listeners | `LinMsg` |
| `__canMsg` | `worker/uds.ts` worker event bridge | plugin/test `Util.OnCan*` | script-facing CAN object |
| `__linMsg` | `worker/uds.ts` | plugin/test `Util.OnLin*` | script-facing LIN object |
| `__serialMsg` | `worker/uds.ts` / `serial/index.ts` | `Util.OnSerial` | byte array/Buffer |
| `__someipMsg` | `worker/uds.ts` / `vsomeip/client.ts` | SOME/IP plugin listeners | `SomeipMessage` |
| `__someipServiceValid` | `vsomeip/client.ts` availability callback | request waiters and plugin listeners | service/instance availability |
| `__varUpdate` | Node variable/event layer | `Util.OnVar*`, renderer variable store | variable name/value |
| `__varFc` | worker flow-control layer | diagnostic script | flow-control state |
| `pluginEvent` | plugin worker SDK `emitEvent` | `PluginClient.eventHandler`, renderer plugin bus | `{name, data}` |
| `hardware-added` | `hardware.rs::process_event` | renderer hardware store | `HardwareDevice` |
| `hardware-removed` | `hardware.rs::process_event` | renderer hardware store | `{id}` |

Frequency classes: frame events are per message/read chunk; worker/plugin events are script-defined; state/error/close events are low frequency. The Core adapter must batch frame events and preserve correlation IDs for diagnostic responses.

## Target Rust EventBus

Current string events should be normalized into typed variants:

```rust
CoreEvent::DeviceAdded(DeviceInfo)
CoreEvent::DeviceRemoved(DeviceId)
CoreEvent::Frame(FrameEvent)
CoreEvent::Diagnostic(DiagnosticEvent)
CoreEvent::Transport(TransportEvent)
CoreEvent::Plugin(PluginEvent)
CoreEvent::Trace(TraceEvent)
CoreEvent::Error(CoreError)
```

### High-frequency policy

CAN/LIN/Serial/XCP frame events must not each cross Tauri IPC. Rust should:

1. receive into a bounded queue/ring buffer;
2. perform filtering, TP assembly, DAQ aggregation and logging in Core;
3. emit batches or state changes at a UI-safe cadence;
4. retain correlation IDs for diagnostic request/response events.
