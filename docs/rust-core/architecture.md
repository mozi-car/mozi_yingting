# Rust Core Architecture

## Scope

This document is the Phase 01 architecture inventory. It intentionally does not change runtime code. The existing native Rust N-API crates are treated as completed driver boundaries; this phase maps the remaining Node.js sidecar responsibilities before moving them into `src-tauri/src/core`.

## Current runtime

```text
Vue / TypeScript UI and plugins
        │ Tauri invoke/events
        ▼
src-tauri/src/bridge.rs
        │ JSON-lines stdin/stdout
        ▼
out/sidecar/index.cjs
        │
src/main/index.ts + src/main/rpc.ts
        │
        ├── src/main/ipc/*              IPC channel adapters
        ├── src/main/nodeItem.ts         device/test orchestration
        ├── src/main/workerClient.ts     worker_threads script runtime
        ├── src/main/pluginCilent.ts     plugin compatibility
        ├── src/main/docan/*             CAN orchestration
        ├── src/main/dolin/*             LIN orchestration
        ├── src/main/doip/*              DoIP/TCP orchestration
        ├── src/main/serial/*            serial orchestration
        ├── src/main/vsomeip/*           SOME/IP compatibility worker
        └── src/main/replay/*            ASC/BLF replay
                │
                ▼
        native/<module>.node Rust N-API crates
                │
                ▼
        Vendor DLL / WinUSB / Windows APIs
```

The Tauri host is currently a process bridge and window host. The Node sidecar is still the product runtime and owns most business orchestration.

## Target runtime

```text
Vue / TypeScript UI and plugin compatibility surface
        │ Tauri commands/events
        ▼
Rust Core
  ├── Runtime / Error / Logger / Config
  ├── EventBus / TaskRuntime
  ├── DeviceManager / Discovery
  ├── Transport: CAN / CAN-FD / LIN / Serial / DoIP
  ├── ISO-TP
  ├── UDS
  ├── XCP
  ├── SOME/IP
  ├── Replay
  └── Plugin compatibility boundary
        │
        ▼
Rust drivers / existing Rust N-API or direct FFI
        │
        ▼
Vendor DLL / WinUSB / Windows APIs
```

Node remains in the development toolchain, Vite, TypeScript, plugin compatibility layer and build scripts until each runtime responsibility has a Rust replacement and a dual-run comparison.

## Architectural rules

1. `core` owns domain behavior; `#[tauri::command]` functions only decode input, call Core, and serialize output.
2. Drivers produce typed `DeviceEvent`, `Frame`, `TransportError` and status values; UI never polls vendor APIs directly.
3. High-frequency CAN/LIN/XCP traffic stays inside Rust and crosses Tauri IPC only through batches, subscriptions or ring-buffer snapshots.
4. Node modules are removed one boundary at a time only after Rust/default and Node/fallback behavior comparisons pass.
5. Existing plugin APIs are preserved through an explicit compatibility layer; plugin migration is not mixed with the first Core extraction.
