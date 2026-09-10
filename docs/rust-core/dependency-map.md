# Dependency Map

## Current process graph

```text
Vue / plugin UI
  -> preload/shim
  -> Tauri invoke/rpc_invoke
  -> src-tauri/src/bridge.rs
  -> child Node: out/sidecar/index.cjs
  -> src/main/rpc.ts
  -> src/main/ipc/* and nodeItem.ts
  -> TypeScript domain adapters / workers
  -> loadNative(name)
  -> native/<name>.node Rust N-API
  -> vendor DLL / WinUSB / Windows API
```

## `src-tauri/src`

| Path | Responsibility | Dependencies | Future Core relation |
|---|---|---|---|
| `lib.rs` | Tauri setup, dialogs, local-resource protocol, single-instance, pending project | Tauri plugins, `bridge`, `hardware` | keep host-only; register Core later |
| `bridge.rs` | Node child process, JSON stdin/stdout, pending RPC map, Tauri event forwarding | `serde_json`, `tauri`, bundled Node/runtime | temporary compatibility transport; eventually direct commands/events |
| `hardware.rs` | Windows PnP USB arrival/removal and registry | cfgmgr32, mpsc, Tauri emitter | becomes `core::discovery` adapter |
| `main.rs` | desktop entrypoint | `yingting_lib::run` | unchanged host entry |

## Native crates

| Crate | API boundary | Current consumers | Target dependency |
|---|---|---|---|
| `native/candle` | SetupAPI/WinUSB CAN/CAN-FD | `docan/candle` | `drivers::candle`, then `Transport::Can` |
| `native/peak` | PCAN-ISO-TP raw ABI | `docan/peak` | `drivers::peak`, then `Transport`/`IsoTp` |
| `native/kvaser` | CANlib raw ABI | `docan/kvaser` | `drivers::kvaser` |
| `native/zlg` | ZLG raw ABI | `docan/zlg` | `drivers::zlg` |
| `native/vector` | XL CAN/CAN-FD/LIN raw ABI | docan/dolin/vector | `drivers::vector` |
| `native/toomoss` | USB2XXX CAN-FD ABI | `docan/toomoss` | `drivers::toomoss` |
| `native/kvaserLin` | LINlib ABI | `dolin/kvaser` | `drivers::kvaser_lin` |
| `native/peakLin` | PLIN ABI | `dolin/peak`, ecubus | `drivers::peak_lin` |
| `native/toomossLin` | USB2LIN ABI | `dolin/toomoss` | `drivers::toomoss_lin` |
| `native/serial` | Windows serial API | `serial/rust` | `Transport::Serial` |
| `native/secure-access` | SecureAccess seed/key DLL | `worker/secureAccess` | `core::uds::security` |
| `native/vsomeip` | SOME/IP/SD Rust transport | `vsomeip/client` | `core::someip` |

## `scripts/`

| Script family | Responsibility | Migration impact |
|---|---|---|
| `build-native.mjs` | Cargo builds 12 N-API crates and copies `.node` artifacts | keep as development/package tooling until Node runtime removed |
| `build-sidecar.mjs` | bundles `src/main/index.ts`, emits `vsomeip.js`, copies native artifacts | removed only after Node runtime removal |
| `build-cli.mjs` | bundles CLI and synchronizes native resources | keep as CLI/tooling; later point CLI to Rust Core |
| `check-native-api.mjs` | runtime export/API parity | retain as driver regression or replace with Rust integration tests |
| `native-interface-regression.mjs` | complete N-API boundary checks | retain during dual-run |
| `native-regression.mjs`, `vendor-dll-regression.mjs`, `vendor-smoke.mjs` | missing-DLL/error/vendor smoke | become driver CI gates |
| `verify-zero-cpp.mjs`, `native-audit.mjs` | active tree and call-chain policy | retain as migration gates |
| `vector-e2e.mjs`, `vsomeip-regression.mjs`, `secure-access-regression.mjs` | module-specific regression | retain and add Core-level equivalents |

## Remaining application-module dependencies

| Module | Direct dependencies | Direct callers | Proposed Core owner |
|---|---|---|---|
| `ipc/*` | Electron compatibility, store, filesystem, Node domain adapters, Python/resources | renderer stores, preload, plugin SDK | `commands` adapters; filesystem/auth/update may remain host services |
| `pwm/*` | `NodeItem`, Ecubus, worker UDS, renderer hardware/network views | `ipc/uds.ts`, `nodeItem.ts`, plugin/CLI | `device` capability |
| `share/*` | imported by CAN/LIN/DoIP/UDS/SOME-IP/serial/replay/CLI | almost every domain adapter and renderer types | shared wire/domain schemas |
| `canmartix.ts` | `iconv-lite`, Python parser, project store | `ipc/canmartix.ts`, project store | project tooling or Core parser service |
| `ostrace/*` | filesystem streams, parser callbacks, worker, renderer timeline | `ipc/ostrace.ts`, `ipc/uds.ts`, trace views | `core::trace` |
| `python.ts` | bundled Python executable and parser assets | CDD/ODX/can matrix IPC | host/tooling service |
| `replay/*` | ASC/BLF binary/text parsing, frame/shared CAN types, logging | `ipc/uds.ts`, NodeItem, replay UI | `core::replay` |
| `worker/canopen/*` | JS protocol/EDS types, Node worker APIs | plugin SDK and user scripts | compatibility first; later `core::canopen` |

## Stage dependency and validation matrix

| Stage | Rust boundary | Node fallback | Required validation |
|---|---|---|---|
| 01 Inventory | none | unchanged | docs cover paths, callers, events, threads, APIs |
| 02 Core contracts | Error/Config/EventBus/Task types | all behavior remains Node | Rust unit tests; no release path change |
| 03 DeviceManager | PnP + typed device registry | Node discovery comparison | add/remove/list/open/close/state event comparison |
| 04 Transport | CAN/LIN/Serial/DoIP trait | existing adapters | open/send/receive/subscribe/close/error/timeout dual-run |
| 05 CAN/LIN | existing Rust native drivers | Node vendor adapters | identical frame/status/timestamp/callback sequences |
| 06 ISO-TP/UDS | Core session/state machine | `docan/cantp`, `dolin/lintp`, `worker/uds` | request/response, P2/P2*, security, transfer and cancellation |
| 07 XCP | Core DAQ/ring buffer | plugin/Node XCP if present | command parity, DAQ ordering/drop/backpressure |
| 08 SOME/IP | Core service/message lifecycle | `vsomeip/index.ts` + worker | offer/request/notify/subscribe/stop/error parity |
| 09 Plugin compat | typed bridge + worker adapter | existing JS plugins | create/exec/event/stop/close and existing plugin suite |
| 10 Rust default | Tauri commands/events | feature-gated fallback | release smoke, rollback switch, sidecar disabled in test mode |
| 11 Remove sidecar | no Node product runtime | none | package has no sidecar runtime dependency; Tauri-only E2E |

## Target dependency graph

```text
core::runtime
  ├── core::error / logger / config
  ├── core::event_bus / task
  ├── core::device / discovery
  ├── core::transport
  │   ├── can / lin / serial / doip
  │   └── drivers::*
  ├── core::isotp
  ├── core::uds
  ├── core::xcp
  ├── core::someip
  ├── core::replay
  └── core::plugin::compat
```

Dependencies flow downward: UI/commands → Core services → transport/protocol → driver/FFI. Core must not depend on Node module paths or vendor-specific TypeScript classes.
