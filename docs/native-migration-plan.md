# Native C++/SWIG → Rust Native-API Migration

## Scope

The project uses one independent Rust N-API crate per native module. Existing TypeScript class names and addon filenames remain stable; the implementation is now loaded through `src/main/native.ts` from the canonical `native/<name>.node` directory.

| Addon | Rust crate | Vendor/runtime boundary |
|---|---|---|
| `sa.node` | `native/secure-access` | SecureAccess DLL via `LoadLibraryA`/`GetProcAddress` |
| `candle.node` | `native/candle` | Windows SetupAPI/WinUSB |
| `peak.node` | `native/peak` | PCAN/PCAN-ISO-TP DLL via raw ABI |
| `kvaser.node` | `native/kvaser` | CANlib DLL via raw ABI |
| `zlg.node` | `native/zlg` | ZLG CAN DLL via raw ABI |
| `vector.node` | `native/vector` | XL driver DLL via raw ABI |
| `toomoss.node` | `native/toomoss` | USB2XXX DLL via raw ABI |
| `kvaserLin.node` | `native/kvaserLin` | LINlib DLL via raw ABI |
| `peakLin.node` | `native/peakLin` | PLIN API DLL via raw ABI |
| `toomossLin.node` | `native/toomossLin` | USB2XXX DLL via raw ABI |
| `vsomeip.node` | `native/vsomeip` | Rust SOME/IP/SD transport and lifecycle |
| `serial.node` | `native/serial` | Rust Windows serial implementation |

## Source and packaging rule

After a module's Rust implementation and TypeScript call-chain switch are complete, its C/C++, SWIG, `binding.gyp`, and old `.node` source is deleted. The repository contains no active or archived legacy driver source. Vendor runtime DLLs remain under `resources/lib`; vendor import `.lib` files are retained under `resources/lib/vendor-import-libs` for ABI/reference compatibility only and are not linked by Cargo.

The sidecar and CLI use only:

```text
out/sidecar/native/<addon>.node
out/cli/native/<addon>.node
resources/lib/native/<addon>.node
```

`resources/lib/native` is synchronized from the sidecar output. The old `resources/lib/js/sa.node` path is forbidden.

## Build and verification gates

```bash
npm run build:native
npm run build:sidecar
npm run test:native
npm run test:vendor:dll
npm run test:vendor
npm run build
npm run test:cli
```

`npm run test:native` rebuilds all Rust addons, checks production exports against the TypeScript call sites, checks the complete interface manifest, verifies missing-DLL/error semantics and Buffer/object boundaries, runs SecureAccess with the repository example DLL, and runs the vSomeIP protocol regression.

`npm run test:vendor:dll` supplies the tracked vendor DLLs to every CAN/LIN Rust loader and to SecureAccess. It proves DLL loading and symbol lookup paths; it does not claim a physical bus is connected.

`scripts/native-audit.mjs` is the source-of-truth call-chain gate. It fails if a Rust crate or `loadNative()` entrypoint is missing, if canonical native artifacts drift, if a legacy C/C++ input appears in the active tree, if the vSomeIP worker artifact is missing, or if a manifest-listed vendor runtime is absent.

## Software acceptance matrix

The migration completion gate is software-based. Each module must pass Rust build, TypeScript API parity, vendor DLL/WinUSB loading, ABI/Buffer/struct checks, error paths, protocol regression, callback/lifecycle and close/reopen/thread-join software tests. This is sufficient for `[x]` software replacement status.

Hardware/bus validation is optional enhanced evidence and does not block the Rust migration. Vector has additional VN5620 physical lifecycle and Virtual Channel loopback evidence in `docs/vector-e2e-20260904.json`. The per-module optional hardware status and current vendor smoke output remain recorded in `docs/native-hardware-acceptance.json` and `docs/native-vendor-smoke-20260909.json`.
