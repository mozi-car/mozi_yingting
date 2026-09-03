# Native C++/SWIG → Rust Native-API Migration

## Current migration state

The 11 addon targets now have independent Rust N-API crates and are rebuilt into the original filenames. The active sidecar contains only Rust native artifacts. The previous C++/SWIG trees are retained as read-only migration references until the matching vendor regression is signed off; they are excluded from the active Rust build.

| Addon | Rust crate | Current state |
|---|---|---|
| `sa.node` | `native/secure-access` | DLL loading and GenerateKeyEx paths implemented; vendor algorithm regression pending |
| `candle.node` | `native/candle` | SetupAPI, WinUSB control/bulk, timing, timestamp, termination and worker paths implemented; hardware regression pending |
| `peak.node` | `native/peak` | PCAN raw ABI/message/mapping/read-write/callback paths implemented; PCAN DLL regression pending |
| `kvaser.node` | `native/kvaser` | CANlib classic/FD, output wrappers, callback and cyclic paths implemented; hardware regression pending |
| `zlg.node` | `native/zlg` | ZLG device/channel/classic/FD, Rust-owned receive arrays and callback paths implemented; hardware regression pending |
| `vector.node` | `native/vector` | XL driver/config/classic/FD event paths implemented; hardware regression pending |
| `toomoss.node` | `native/toomoss` | ControlCAN device/channel/classic/FD paths and vendor struct packing implemented; hardware regression pending |
| `kvaserLin.node` | `native/kvaserLin` | LINlib channel/read/write/request/update/wakeup and callback paths implemented; hardware regression pending |
| `peakLin.node` | `native/peakLin` | PLIN client, hardware buffer, frames, read/write and callback paths implemented; hardware regression pending |
| `toomossLin.node` | `native/toomossLin` | USB2LIN frame, slave/send and callback paths implemented; hardware regression pending |
| `vsomeip.node` | `native/vsomeip` | Rust SOME/IP/SD socket runtime, service/message/callback/periodic lifecycle implemented; target configuration interoperability pending |

`[~]` is intentional: a release build and source-level ABI check cannot prove behavior against a vendor DLL or physical bus.

## Build and verification gates

The authoritative commands are:

```bash
npm run test:native
npm run build:sidecar
npm run build:renderer
npm run build
npm run test:cli
```

`npm run test:native` first rebuilds all Rust addons, then runs API parity, missing-DLL/error semantics, wrapper Buffer/object lifecycle checks, and the vSomeIP regression. `scripts/native-regression.mjs` runs vendor DLL loading checks when `YT_VENDOR_DLL_<MODULE>` variables are supplied by a hardware CI runner. It never treats an absent DLL or device as success.

`npm run build` was verified through the Tauri NSIS installer stage on Windows GNU. `npm run test:cli` rebuilds the CLI, copies every Rust `.node` addon into both `out/cli/native` and the deployed `resources/lib/native`, and verifies `node out/cli/myt.cjs --help`. The active sidecar gate scans `native/` and `out/sidecar/`; retained legacy trees are explicitly reference-only and are not copied into the product.

## Hardware acceptance matrix

For each addon, the target machine must provide the vendor DLL and hardware and record:

1. DLL load, required symbol lookup and reported error codes.
2. Device/channel enumeration and open/close.
3. Bus configuration and struct field/packing validation.
4. Classic CAN, CAN-FD, LIN or SOME/IP send/receive and timestamps.
5. Error/status paths, callback delivery and worker stop/join.
6. Repeated reset/close/reopen without leaked handles or threads.

The GUI enumeration path was verified through the same IPC request used by `canNode` and recorded in [`vector-gui-path-20260903.json`](./vector-gui-path-20260903.json). A vendor smoke run was recorded in [`native-vendor-smoke-20260903.json`](./native-vendor-smoke-20260903.json). It includes a real Vector VN5620: nine physical channels were enumerated and CAN Channel 5 was opened, activated, polled, deactivated, and closed. No CAN peer/frame was present, so the log does not qualify as complete bus behavior acceptance.

Only after the complete evidence is available may the corresponding `[~]` become `[x]`, followed by removal of that module's C++/SWIG reference tree. No hardware result is claimed by the repository-only tests.
