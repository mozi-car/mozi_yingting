# Rust Core Migration Status

## Phase 01 — architecture inventory

Status: **complete**

Scope analyzed without changing runtime source:

```text
src/main/
src-tauri/src/
native/
src/
scripts/
```

Generated documents:

- `architecture.md`
- `node-runtime-map.md`
- `dependency-map.md`
- `api-map.md`
- `event-map.md`
- `threading-map.md`
- `plugin-compatibility.md`
- `migration-status.md`

The inventory identifies actual source paths, Native/Rust APIs, RPC channels, event names, workers, processes, high-frequency paths, synchronous/asynchronous boundaries, Vue/Plugin callers and Node-to-Rust destinations. Existing Node behavior and existing Rust Native drivers were not changed during Phase 01.

## Phase 02 status

**In progress: architecture skeleton only.** `src-tauri/src/core/` now contains empty module boundaries and a compile-only `CoreSkeleton` marker. No business behavior, driver adapter, protocol implementation, Node replacement or behavior change has been introduced. Phase 03 infrastructure is not started. The following phases remain a plan only.

## Current facts

- Product runtime still starts `out/sidecar/index.cjs`.
- `src-tauri/src/bridge.rs` owns the Node child process and JSON-lines bridge.
- `src-tauri/src/hardware.rs` is event-driven PnP discovery only, not a Core device manager.
- `src/main/nodeItem.ts` is the highest-fan-in business orchestrator.
- `src/main/rpc.ts` is the largest compatibility boundary.
- `src/main/workerClient.ts` is the main script/test worker boundary.
- `src/main/pluginCilent.ts` is the plugin compatibility boundary.
- Native Rust N-API crates are driver boundaries, not yet a unified Rust Core.

## Planned implementation order

| Phase | Deliverable | Code change? | Exit criteria |
|---|---|---:|---|
| 02 | Rust Core skeleton and typed Error/Logger/Config | yes | compiles; no runtime behavior change |
| 03 | EventBus and TaskRuntime | yes | cancellation, join and event tests |
| 04 | DeviceManager and discovery adapter | yes | PnP/vendor scan parity and typed events |
| 05 | Transport traits and CAN adapter | yes | Rust/Node dual-run comparison |
| 06 | LIN and Serial adapters | yes | API/error/lifecycle parity |
| 07 | DoIP transport | yes | socket/error/timeout parity |
| 08 | ISO-TP | yes | frame/session/timer tests |
| 09 | UDS | yes | request/response/security/timeout tests |
| 10 | XCP | yes | DAQ ring-buffer and command tests |
| 11 | SOME/IP | yes | service discovery/request/event lifecycle tests |
| 12 | Plugin compatibility layer | yes | existing plugin suite passes |
| 13 | Rust default / Node fallback | yes | feature-gated dual-run and rollback |
| 14 | Remove Node product runtime | yes | no `build:sidecar`, no Node child in release |

## Validation policy for future phases

Every phase must provide:

1. unit tests for the new Core contract;
2. integration tests against the existing Node behavior where both paths exist;
3. explicit event/thread/cancellation tests;
4. a rollback/fallback path until the Rust path is default;
5. no Vue/plugin API break unless separately approved.

## Current Phase 02 implementation boundary

The current task only establishes module boundaries in `src-tauri/src/core/`, `src-tauri/src/drivers/`, `src-tauri/src/commands/` and `src-tauri/src/events/`. It does not implement Error/Logging/Config/EventBus/Task Runtime or DeviceManager. Those belong to Phase 03 and Phase 04 respectively. Existing Node behavior remains the product runtime and existing Native crates remain unchanged.

## Phase 01 final report A–H

### A. Current real architecture

Vue/TypeScript and plugin UI call Tauri IPC; `bridge.rs` launches `out/sidecar/index.cjs`; Node `rpc.ts` dispatches `ipc/*`; `nodeItem`/workers own domain orchestration; TypeScript adapters load completed Rust Native `.node` crates; those crates call vendor DLL/WinUSB/system APIs.

### B. Target Rust Core architecture

Vue/Plugin UI → typed Tauri commands/events → Rust Core services → Drivers/FFI → vendor DLL/SDK. Node remains only as build tooling and a Plugin Compatibility fallback until later phases.

### C. Node → Rust mapping

| Node responsibility | Rust destination |
|---|---|
| `rpc.ts` / `ipc/*` | typed `commands` and event adapters |
| `nodeItem.ts` | `DeviceManager`, `Transport`, protocol services |
| `docan/*` / `dolin/*` / `serial/*` / `doip/*` | `Transport` implementations |
| `cantp.ts` / `lintp.ts` | shared `IsoTp` |
| `uds.ts` / worker UDS | `Uds` service |
| `vsomeip/*` | `SomeIp` service |
| `replay/*` / `ostrace/*` | `Replay` / `Trace` services |
| `workerClient.ts` / plugin SDK | `TaskRuntime` and `PluginCompat` |
| `share/*` | versioned Core/UI wire schemas |

### D. Recommended migration order

Error → Logging → Config → EventBus → Task/Worker → DeviceManager → Discovery → Transport → CAN → LIN → Serial → ISO-TP → UDS → XCP → SOME/IP → PluginCompat → dual-run → Rust default → remove Node.

### E. Module dependencies

The detailed dependency graph is in `dependency-map.md`; Native crate boundaries are listed against each Node adapter, and `ipc/*` dependencies are separated into hardware/domain, host/tooling and plugin groups.

### F. Risk levels

The risk matrix is in `node-runtime-map.md`: `rpc/index`, `nodeItem`, `workerClient`, `ipc/uds` are P0; CAN/LIN/Serial/DoIP/SOME-IP are P1; replay/trace/tooling/share are P2.

### G. Node fallback that must remain

Keep Node fallback for generic RPC, `nodeItem`, worker/plugin SDK, UDS script API, CAN/LIN/Serial compatibility wrappers, SOME/IP worker, replay/trace and Python/package tooling until their Rust replacement has dual-run coverage.

### H. Phase 02 scope

Phase 02 is not started. After approval, it may create only the Rust Core contracts/skeleton and tests; it must not yet migrate business modules, delete Node, modify Native, change Vue, or alter plugin behavior.
