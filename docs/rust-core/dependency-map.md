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

## Strict Phase 1–18 dependency and validation matrix

| Phase | Rust boundary/dependency | Node fallback | Required validation |
|---:|---|---|---|
| 1 | architecture inventory only | unchanged | docs cover paths, callers, APIs, events, threads, polling and risks |
| 2 | Core module skeleton only | all behavior remains Node | compiles; no runtime path or behavior change |
| 3 | Error, Logging, Config, EventBus, Task Runtime | Node services remain default | unit tests, cancellation/join tests, event serialization tests |
| 4 | DeviceManager | NodeItem/device classes | list/open/close/state ownership tests |
| 5 | DeviceManager + Discovery adapter | Node/PnP/vendor discovery | add/remove/list parity and event ordering |
| 6 | Transport trait: open/send/receive/close/subscribe | existing CAN/LIN/Serial/DoIP adapters | typed transport contract and error/timeout tests |
| 7 | CAN/CAN-FD Core transport → existing Rust Native drivers | Node CAN adapters | discover/open/config/send/receive/status/timestamp dual-run |
| 8 | LIN Core transport/scheduler → existing Rust Native drivers | Node LIN adapters | schedule/request/wakeup/read/write/callback parity |
| 9 | Serial Core transport → existing Rust serial Native | `serial/index.ts` compatibility | open/write/read/list/close/error parity |
| 10 | shared ISO-TP session/frame/timer Core | `docan/cantp`, `dolin/lintp` | CAN/LIN/DoIP frame/session/timeout tests |
| 11 | UDS Core over Transport/ISO-TP/DoIP | `docan/uds`, `worker/uds` | request/response/security/P2/P2*/transfer/DTC parity |
| 12 | XCP Core and DAQ ring buffer | existing Node/plugin path | command parity, DAQ ordering/drop/backpressure |
| 13 | SOME/IP Core service/message lifecycle | `vsomeip/index.ts` + worker | offer/request/notify/subscribe/stop/error parity |
| 14 | Plugin Compatibility adapter | existing JS plugin workers | create/exec/event/stop/close and plugin suite |
| 15 | Rust/Node dual-run comparison harness | Node remains fallback | identical scripted result/event/error traces |
| 16 | Tauri commands/events call Rust Core by default | feature-gated Node fallback | release smoke, rollback switch, sidecar fallback test |
| 17 | remove Node product runtime/Sidecar | none in product release | no Node child, no `build:sidecar`, Tauri-only E2E |
| 18 | clean packaging/build dependencies | Node only dev tooling | package/resource audit and final installer build |

Dependencies flow downward: UI/commands → Core services → transport/protocol → driver/FFI. Core must not depend on Node module paths or vendor-specific TypeScript classes.

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
