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
