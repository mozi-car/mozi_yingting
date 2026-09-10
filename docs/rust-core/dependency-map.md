# Dependency Map

## Runtime dependency graph

```text
Vue renderer
  └── Tauri invoke / event listener
      └── src-tauri/src/bridge.rs
          └── Node runtime: out/sidecar/index.cjs
              ├── rpc.ts
              ├── ipc/*
              │   ├── nodeItem / device APIs
              │   ├── filesystem/config/plugin APIs
              │   └── serial/UDS/trace APIs
              ├── nodeItem.ts
              │   ├── docan/*
              │   ├── dolin/*
              │   ├── doip/*
              │   ├── serial/*
              │   ├── vsomeip/*
              │   └── workerClient.ts
              └── native.ts
                  └── native/<module>.node
                      └── vendor DLL / WinUSB / Windows API
```

## Domain dependencies

| Domain | Current Node modules | Existing lower boundary | Target Core module |
|---|---|---|---|
| Device discovery | `docan/can.ts`, `dolin/index.ts`, `doip/index.ts`, Tauri `hardware.rs` | Rust PnP + driver scan APIs | `device` + `discovery` |
| CAN | `docan/base.ts`, vendor classes | `candle`, `peak`, `kvaser`, `zlg`, `vector`, `toomoss` | `transport::can` |
| LIN | `dolin/base.ts`, vendor classes | `kvaserLin`, `peakLin`, `toomossLin`, Vector | `transport::lin` |
| Serial | `serial/index.ts`, `serial/rust.ts` | `serial` Rust N-API | `transport::serial` |
| DoIP | `doip/index.ts` | Node `net`/`dgram` | `transport::doip` |
| ISO-TP | `docan/cantp.ts`, `dolin/lintp.ts` | CAN/LIN classes | `isotp` |
| UDS | `docan/uds.ts`, `worker/uds.ts`, `ipc/uds.ts` | CAN-TP/LIN-TP/DoIP | `uds` |
| SOME/IP | `vsomeip/client.ts`, `index.ts`, `worker.ts` | Rust SOME/IP N-API | `someip` |
| Replay | `replay/ascReader.ts`, `blfReader.ts` | filesystem and frame APIs | `replay` |
| Plugins | `pluginCilent.ts`, `ipc/plugin.ts`, `workerClient.ts` | Node worker/plugin API | `plugin::compat` |

## Dependency constraints

- Core must depend on typed driver traits, not on N-API object shapes.
- Native N-API crates can remain an adapter during migration, but Core should eventually call shared Rust driver interfaces directly or through a narrow FFI layer.
- UI and plugins must not depend on vendor DLL names or `.node` paths.
- `nodeItem` is a high fan-in legacy orchestrator and should be migrated by extracting services, not by translating the file wholesale.
