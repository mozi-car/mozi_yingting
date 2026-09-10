# Architecture Issues Recorded During Phase 01

These are observations for later phases. Phase 01 records them but does not fix them.

## ISSUE-001 — Node sidecar is still the product runtime

- Evidence: `package.json.main`, `src-tauri/src/bridge.rs`, `scripts/build-sidecar.mjs`.
- Impact: Rust Core cannot become the default until command/event and plugin compatibility contracts exist.
- Planned phase: 15–17.

## ISSUE-002 — Generic JSON RPC has weak typing and cancellation

- Evidence: `src/main/rpc.ts`, `src-tauri/src/bridge.rs`.
- Impact: channel payloads are untyped; long-running operations use a fixed bridge timeout; cancellation is not a first-class protocol concept.
- Planned phase: 02–03, then channel-by-channel migration.

## ISSUE-003 — `nodeItem.ts` is a high-fan-in orchestration object

- Evidence: `src/main/nodeItem.ts`, `src/main/ipc/uds.ts`, `src/main/workerClient.ts`.
- Impact: direct file translation would reproduce Node coupling in Rust; services must be extracted by domain.
- Planned phase: DeviceManager/Transport/UDS phases.

## ISSUE-004 — Worker/plugin API is broader than hardware API

- Evidence: `src/main/workerClient.ts`, `src/main/worker/uds.ts`, `src/main/plugin-sdk/index.ts`.
- Impact: deleting Node before a compatibility layer would break existing plugins and scripts.
- Planned phase: 14–15.

## ISSUE-005 — Hardware PnP and driver discovery are separate

- Evidence: `src-tauri/src/hardware.rs` emits USB arrival/removal only; vendor adapters separately scan/open devices.
- Impact: DeviceManager must merge PnP events with vendor discovery and explicit open/close state.
- Planned phase: 04–05.

## ISSUE-006 — High-frequency frames currently cross multiple JavaScript event/worker boundaries

- Evidence: native callback/read → TypeScript adapters → EventEmitter → `workerClient` → RPC/event output.
- Impact: per-frame Tauri IPC would be too expensive; Core needs bounded channels, batching and aggregation.
- Planned phase: EventBus/Transport/CAN/XCP phases.

## ISSUE-007 — Python and bundled package tooling are host services

- Evidence: `src/main/python.ts`, `ipc/cdd.ts`, `ipc/odx.ts`, `ipc/pnpm.ts`, `resources/lib/myt`.
- Impact: these should not be pulled into the first Rust hardware Core; they need an explicit host/tooling boundary.
- Planned phase: plugin/tooling compatibility phase.
