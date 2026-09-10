# Node Runtime Map

> Phase 01 inventory. This file records actual source responsibilities and paths; it does not change runtime behavior.

## Entry and process boundary

| Path | Actual responsibility | Calls / called by | Rust Core destination |
|---|---|---|---|
| `src/main/index.ts` | Sidecar startup, logging directory/transport, i18n, Casdoor, `electron-store-*`, RPC startup | imports `ipc/index.ts`; starts `rpc.ts` | `core::runtime`, `core::config`, typed Tauri commands |
| `src/main/rpc.ts` | stdin/stdout JSON-lines protocol, channel registry, request response, `emit`, fake Electron event | `src-tauri/src/bridge.rs`; all `src/main/ipc/*` handlers | temporary compatibility adapter, then `commands/events` |
| `src/main/native.ts` | `YT_NATIVE_DIR`/`__dirname/native` resolution and `createRequire` for `.node` | every vendor adapter under `docan`, `dolin`, `serial`, `vsomeip`, `secureAccess` | `drivers` adapter; eventually direct Rust driver traits |
| `src/main/nodeItem.ts` | high-fan-in device/test orchestrator; CAN/LIN/DoIP/Serial/PWM/SOME-IP; UDS; logs; worker handlers | `ipc/uds.ts`, `workerClient.ts`, plugin workers, renderer events | `DeviceManager`, `Transport`, `IsoTp`, `Uds`, `Doip`, `Someip` |
| `src/main/workerClient.ts` | one Node `Worker` per script/test, pending RPC map, stdout/stderr, worker events, handler dispatch | `NodeClass`, `worker/uds.ts`, `worker/someip.ts`, `worker/cantp.ts` | `core::task`, `core::event_bus`, plugin compatibility |
| `src/main/pluginCilent.ts` | plugin `exec`, plugin event/log lifecycle, `NodeClass` ownership | `ipc/plugin.ts`, renderer plugin SDK | `plugin::compat` |
| `src/main/ipc/index.ts` | imports all IPC registrations; vendor/version and Electron compatibility channels | renderer stores/preload | typed `commands` |

## Module-by-module inventory

### `src/main/transport/`

- `asc.ts`: ASC text formatting and replay/log output for CAN/LIN frames.
- `blf.ts`: BLF writer/container encoder, CAN/CAN-FD/error object serialization, timestamp conversion and asynchronous close/flush.
- No vendor driver is loaded here. Target: `core::replay` and `core::trace`; high-rate writes should use a Rust buffered writer.

### `src/main/uds/`

- `service.ts`: service metadata, supported service IDs, built-in script/plugin paths.
- The protocol implementation is primarily in `docan/uds.ts`, `worker/uds.ts`, `docan/cantp.ts` and `dolin/lintp.ts`; `uds/service.ts` is configuration/catalog data.
- Target: `core::uds` and a typed service catalog; retain JS test/plugin API in compatibility layer.

### `src/main/docan/`

- `base.ts`: `CanBase` lifecycle, message EventEmitter, timestamps, logging and common errors.
- `can.ts`: device discovery/open dispatch and vendor version/device aggregation.
- `candle/index.ts`: CandleLight setup, timing, termination, CAN/CAN-FD send/receive, timestamp and shared-device close.
- `kvaser/index.ts`: CANlib load/channel, classic/FD bus timing, read/write/status, cyclic send and callback.
- `peak/index.ts`: PCAN-ISO-TP initialization, mappings, message ABI, read/write and cyclic send.
- `zlg/index.ts`: ZLG device/channel/CAN-FD setup, receive arrays and callback.
- `toomoss/index.ts`: USB2XXX scan/open/config/CAN/CAN-FD send/receive/callback.
- `vector/index.ts`: XL driver configuration, CAN/CAN-FD events, channel lifecycle and callbacks.
- `slcan/index.ts`: serial-line CAN/ASCII framing; target is Serial Transport + CAN framing.
- `cantp.ts`: CAN ISO-TP session/frame state machine and timeout handling.
- `uds.ts`: UDS service request/response orchestration over CAN-TP and script APIs.
- `simulate/index.ts`: simulation backend; should implement the same future `Transport` trait.

Native API examples are enumerated in `scripts/native-interface-manifest.json` and checked by `scripts/check-native-api.mjs`.

### `src/main/dolin/`

- `base.ts`: LIN device abstraction, EventEmitter, logging and common lifecycle.
- `index.ts`: LIN device discovery/open dispatch and vendor version aggregation.
- `kvaser/index.ts`: LINlib channel/baud/read/write/schedule/wakeup/status.
- `peak/index.ts`: PLIN client/hardware/frame/read/write/callback operations.
- `toomoss/index.ts`: USB2LIN scan/open/close/frame/slave/callback operations.
- `vector/index.ts`: XL LIN channel parameters, schedule/request/wakeup/event handling.
- `lintp.ts`: LIN ISO-TP session and timeout behavior.
- `ecubus/index.ts`: shared/compatibility LIN adapter used by Ecubus paths.
- Target: `core::transport::lin` plus shared `core::isotp`.

### `src/main/doip/`

`index.ts` owns Ethernet discovery, UDP vehicle identification, TCP/TLS sockets, routing activation, alive checks, diagnostic payload parsing and close/error events. Target: `core::transport::doip`; retain TLS/certificate configuration at the Rust boundary.

### `src/main/serial/`

- `rust.ts`: Rust `serial.node` wrapper (`open`, `write`, `drain`, `flush`, `close`, `isOpen`, `list`) and EventEmitter conversion.
- `index.ts`: SerialBase domain adapter and message event lifecycle.
- Target: `core::transport::serial`; Node compatibility remains for plugin scripts.

### `src/main/vsomeip/`

- `client.ts`: Rust-backed Runtime/Application/Send wrapper and callback conversion.
- `index.ts`: config generation, routing manager facade, service/event/request orchestration and logging.
- `worker.ts`: child-process message switch for request, notify, offer, subscribe, periodic and stop operations.
- Target: `core::someip`; first Rust implementation must preserve the worker protocol before removing the child process.

### `src/main/worker/`

- `worker/uds.ts`: public plugin/test SDK, UDS helpers, CAN/LIN/Serial/SOME-IP event registration and test lifecycle.
- `worker/cantp.ts`: CAN-TP worker-facing API.
- `worker/someip.ts`: SOME/IP worker-facing API.
- `worker/canopen/`: JavaScript CANopen EDS/device/protocol implementation (NMT, PDO, SDO, EMCY, LSS, SYNC, TIME).
- `worker/secureAccess/`: SecureAccess compatibility wrapper over `sa.node`.
- `worker/ostrace.ts`, `worker/fuzz.ts`, `worker/cryptoExt.ts`, `worker/crc.ts`: script/runtime utilities.
- Target split: Rust Core protocols and task runtime, but preserve a JS plugin compatibility ABI until dual-run tests pass.

### `src/main/replay/`

- `ascReader.ts`: line-oriented ASC parser, timestamp/delay normalization and replay progress.
- `blfReader.ts`: binary BLF container/object parser for classic CAN, CAN-FD and error frames.
- `index.ts`: replay source orchestration and frame delivery.
- Target: `core::replay`, with output converted into the same `Frame`/EventBus path as live traffic.

## High-frequency paths

```text
vendor callback / serial read / replay frame
  -> TypeScript adapter EventEmitter
  -> CAN-TP/LIN-TP/LIN scheduler or NodeClass
  -> workerClient.postMessage()
  -> user script / NodeItem
  -> rpc.emit()
  -> Tauri bridge stdout reader
  -> renderer/plugin event
```

Rust migration must keep frame streams, DAQ and LIN schedules inside Core and cross Tauri IPC only as batches or state/diagnostic events.
