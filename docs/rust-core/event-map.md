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
