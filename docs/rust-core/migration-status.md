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

**Not started.** No `src-tauri/src/core/` implementation is part of this architecture-inventory phase. The following phases are a plan only.

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

## First implementation task after this inventory

Only after this architecture inventory is reviewed and approved should the next task implement the Rust Core skeleton and `DeviceManager` interfaces in `src-tauri/src/core/`. That future task must not change existing Node behavior. The first adapter should mirror the current Tauri PnP events and expose a typed command/event surface; device driver integration should follow after the Core contracts are tested.
