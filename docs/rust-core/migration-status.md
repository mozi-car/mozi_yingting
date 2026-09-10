# Rust Core Migration Status

## Phase 01 — Node Runtime architecture inventory

Status: **complete**

Scope analyzed without changing runtime source:

```text
src/main/
src-tauri/src/
native/
src/
scripts/
package.json
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
- `architecture-issues.md`

The inventory records actual source paths, Native/Rust APIs, RPC channels, Vue/Plugin callers, event names, workers, processes, polling/timers, high-frequency paths and synchronous/asynchronous boundaries. Existing Node behavior and existing Rust Native drivers were not changed during Phase 01.

## Current facts

- Product runtime still starts `out/sidecar/index.cjs`.
- `src-tauri/src/bridge.rs` owns the Node child process and JSON-lines bridge.
- `src-tauri/src/hardware.rs` is event-driven PnP discovery only, not a Core DeviceManager.
- `src/main/nodeItem.ts` is the highest-fan-in business orchestrator.
- `src/main/rpc.ts` is the largest compatibility boundary.
- `src/main/workerClient.ts` is the main script/test worker boundary.
- `src/main/pluginCilent.ts` is the plugin compatibility boundary.
- Native Rust N-API crates are completed driver boundaries, not yet a unified Rust Core.

## Strict 18-phase plan

| Phase | Scope | Current status | Exit criteria |
|---:|---|---|---|
| 1 | Analyze Node Runtime architecture | **complete** | 8 migration maps, risk/issues list and A–H report committed |
| 2 | Establish Rust Core architecture skeleton only | **not started** | module boundaries compile; no business behavior changed |
| 3 | Migrate Error / Logging / Config / EventBus / Task Runtime | not started | infrastructure contracts and unit tests pass |
| 4 | DeviceManager | not started | typed device list/open/close/state contract and dual-run tests |
| 5 | Device Discovery | not started | PnP/vendor discovery merged into DeviceManager events |
| 6 | Transport abstraction | not started | `open/send/receive/close/subscribe` contract tested |
| 7 | CAN / CAN-FD | not started | Rust Core transport with Node fallback and frame parity |
| 8 | LIN | not started | Rust Core LIN transport/scheduler parity |
| 9 | Serial | not started | Rust Core serial transport and plugin compatibility |
| 10 | ISO-TP | not started | CAN/LIN/DoIP shared session/frame/timer tests |
| 11 | UDS | not started | request/response/security/timeout/transfer parity |
| 12 | XCP | not started | command and DAQ ring-buffer tests |
| 13 | SOME/IP | not started | service discovery/request/event lifecycle parity |
| 14 | Plugin Compatibility | not started | existing JavaScript plugin API remains functional through adapter |
| 15 | Node/Rust dual-run tests | not started | identical scripted results/events/errors and rollback path |
| 16 | Rust Core becomes default | not started | feature-gated Node fallback, Rust default release smoke |
| 17 | Delete Node Runtime / Sidecar | not started | no Node child or sidecar product dependency |
| 18 | Clean build system | not started | remove runtime sidecar build/copy paths; keep Node only for tooling |

### Ordering constraints

- Phase 2 is only an architecture skeleton. It must not implement Phase 3 infrastructure or Phase 4 DeviceManager.
- Phase 3 must finish Error, Logging, Config, EventBus and Task Runtime before DeviceManager work begins.
- Phase 4 DeviceManager is separate from Phase 5 Device Discovery.
- Phase 6 Transport precedes CAN, LIN and Serial.
- ISO-TP precedes UDS; UDS/Transport behavior must remain 1:1.
- Node fallback remains until Phase 15 dual-run validation passes.
- Phase 17/18 cannot begin before Phase 16 Rust-default validation.

## Validation policy

Every implementation phase must provide:

1. unit tests for the new Core contract;
2. integration tests against existing Node behavior where both paths exist;
3. explicit event/thread/cancellation tests;
4. a rollback/fallback path until Rust becomes default;
5. no Vue/plugin API break unless separately approved;
6. a documented command, result and residual-risk report.

## Phase 01 final report A–H

### A. Current real architecture

Vue/TypeScript and plugin UI call Tauri IPC; `bridge.rs` launches `out/sidecar/index.cjs`; Node `rpc.ts` dispatches `ipc/*`; `nodeItem`/workers own domain orchestration; TypeScript adapters load completed Rust Native `.node` crates; those crates call vendor DLL/WinUSB/system APIs.

### B. Target Rust Core architecture

Vue/Plugin UI → typed Tauri commands/events → Rust Core services → Drivers/FFI → vendor DLL/SDK. Node remains only as build tooling and a Plugin Compatibility fallback until Phase 17.

### C. Node → Rust mapping

| Node responsibility | Rust destination | Planned phase |
|---|---|---:|
| `rpc.ts` / `ipc/*` | typed `commands` and event adapters | 3, 15–17 |
| `nodeItem.ts` | DeviceManager, Transport and protocol services | 4–11 |
| `docan/*` / `dolin/*` / `serial/*` / `doip/*` | Transport implementations | 6–9 |
| `cantp.ts` / `lintp.ts` | shared ISO-TP | 10 |
| `uds.ts` / worker UDS | UDS service | 11 |
| `vsomeip/*` | SOME/IP service | 13 |
| `replay/*` / `ostrace/*` | Replay/Trace services | 6–15 |
| `workerClient.ts` / plugin SDK | Task Runtime and PluginCompat | 3, 14–15 |
| `share/*` | versioned Core/UI wire schemas | 3–6 |

### D. Recommended migration order

Exactly the 18 phases listed above: inventory → skeleton → infrastructure → DeviceManager → Discovery → Transport → CAN → LIN → Serial → ISO-TP → UDS → XCP → SOME/IP → Plugin Compatibility → dual-run → Rust default → remove Node → clean build.

### E. Module dependencies

`dependency-map.md` contains the current process graph, `src-tauri/src` responsibilities, all Native crate boundaries, remaining application module dependencies and the stage dependency/validation matrix.

### F. Risk levels

`node-runtime-map.md` contains the risk matrix: RPC/index, NodeItem, WorkerClient and `ipc/uds` are P0; domain transports/protocols are P1; replay/trace/tooling/share are P2.

### G. Node fallback that must remain

Keep Node fallback for generic RPC, NodeItem, worker/plugin SDK, UDS script API, CAN/LIN/Serial compatibility wrappers, SOME/IP worker, replay/trace, Python parser tooling and package tooling until Phase 15 passes.

### H. Phase 2 scope

Phase 2 is **not started**. When approved, it may only create the Rust Core module boundaries under `src-tauri/src/core`, `drivers`, `commands` and `events`, with compile-only tests. It must not implement Phase 3 infrastructure, DeviceManager, transports, protocols, delete Node, modify Native, change Vue or alter plugin behavior.
