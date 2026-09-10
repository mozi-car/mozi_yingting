# API Map

> All paths and names below are extracted from the current source tree. The proposed Rust API is a destination; current TypeScript/RPC names remain compatibility contracts.

## Tauri host commands and events

Defined in `src-tauri/src/lib.rs` and `src-tauri/src/bridge.rs`:

| API | Current producer/consumer | Payload / behavior | Future owner |
|---|---|---|---|
| `rpc_invoke` | renderer shim → `bridge.rs` → sidecar | channel + JSON args, 30s response wait | typed Core commands with compatibility adapter |
| `rpc_emit` | renderer → sidecar stdin | one-way channel event | Core EventBus adapter |
| `app_info` | renderer → Tauri | name/version/platform | Tauri host |
| `take_pending_open` | renderer startup → Tauri state | pending `.mytproject` path | Core config/open-project |
| `read_local_resource` | plugin/renderer → Tauri | file bytes | Tauri resource service |
| `yt-sidecar-event` | bridge stdout reader → renderer | `{channel, payload[]}` | typed Tauri events |
| `hardware-added` | `src-tauri/src/hardware.rs` → renderer | discovered PnP device | `CoreEvent::DeviceAdded` |
| `hardware-removed` | `src-tauri/src/hardware.rs` → renderer | device ID | `CoreEvent::DeviceRemoved` |

## Sidecar RPC channels

Registered in `src/main/ipc/*.ts` and called from renderer/preload paths:

### Runtime/device/diagnostic

```text
ipc-global-start             ipc-global-stop
ipc-get-can-devices          ipc-get-lin-devices
ipc-get-serial-devices       ipc-get-serial-port-list
ipc-get-pwm-devices          ipc-get-vendor
ipc-get-version              ipc-get-build-status
ipc-create-project           ipc-build-project
ipc-delete-node              ipc-delete-tester
ipc-get-test                 ipc-get-test-info
ipc-get-test-report          ipc-service-detail
ipc-send-can                 ipc-send-lin
ipc-send-eth                 ipc-send-serial
ipc-send-someip              ipc-send-someip-period
ipc-start-schedule           ipc-stop-schedule
ipc-start-can-period         ipc-stop-can-period
ipc-update-can-period        ipc-update-can-signal
ipc-update-lin-signals       ipc-update-someip-period
ipc-eth-start-period         ipc-signal-set
ipc-var-set                  ipc-get-user-info
```

### Project/files/parsers

```text
ipc-fs-exist                 ipc-fs-mkdir
ipc-fs-readFile              ipc-fs-readdir
ipc-fs-rmdir                 ipc-fs-stat
ipc-fs-writeFile             ipc-glob
ipc-path-parse               ipc-path-relative
ipcCddParse                  ipcCddParseTesterInfo
ipcOdxParse                  ipcOdxParseTesterInfo
ipc-canmartix-parse          ipc-canmartix-exportOtherFile
ipc-examples                 ipc-create-example
ipc-pnpm-init                ipc-pnpm-install
ipc-build-project
```

### Plugin/account/update/UI compatibility

```text
ipc-plugin-create             ipc-plugin-close
ipc-plugin-exec               ipc-plugin-lib-path
ipc-install-plugin-from-zip   ipc-install-remote-plugin
ipc-list-plugin-dirs          ipc-get-remote-plugins
ipc-authenticated-request     ipc-auto-login
ipc-logout                    refreshToken
ipc-show-open-dialog          ipc-show-save-dialog
ipc-show-message-box          icp-show-error-box
ipc-check-update              ipc-install-update
ipc-start-update              ipc-update-releases-note
set-language                  open-project
rpc_invoke                    take_pending_open
```

These names are current compatibility channels, not the final Rust Core API. Each channel must eventually map to a typed command or typed EventBus subscription.

## Native/Rust APIs currently used by TypeScript

| TypeScript path | Addon | Actual API surface |
|---|---|---|
| `docan/candle/index.ts` | `candle` | `scanDevices`, `candle_dev_open/close`, timing/termination controls, `candle_frame_t`, `SendCANMsg`, `CreateTSFN/FreeTSFN` |
| `docan/kvaser/index.ts` | `kvaser` | `LoadDll`, channel/bus params, CAN/CAN-FD read/write/status, channel data/version, cyclic send, TSFN |
| `docan/peak/index.ts` | `peak` | PCAN constants, `cantp_msg`, initialize/read/write/mapping/value APIs, data accessors, cyclic and callbacks |
| `docan/zlg/index.ts` | `zlg` | `LoadDll`, device/channel open, transmit/receive, CAN-FD arrays and callbacks |
| `docan/toomoss/index.ts` | `toomoss` | `LoadDll`, USB scan/open/close, CAN init/send/get, CAN-FD structs and callbacks |
| `docan/vector/index.ts`, `dolin/vector/index.ts` | `vector` | XL config/port/channel, CAN/CAN-FD events, LIN params/events, transmit/receive, callbacks |
| `dolin/kvaser/index.ts` | `kvaserLin` | LINlib load/channel/baud/read/write/schedule/wakeup/status/callback |
| `dolin/peak/index.ts`, `dolin/ecubus/index.ts` | `peakLin` | PLIN client/hardware/frame/read/write/register/callback APIs |
| `dolin/toomoss/index.ts` | `toomossLin` | USB2LIN scan/open/close, LIN frame/slave/send/get/callback APIs |
| `serial/rust.ts` | `serial` | `Serial`, `list`, open/write/drain/flush/close/isOpen |
| `worker/secureAccess/index.ts` | `sa` | `SeedKey`, `LoadDLL`, `Unload`, `IsLoaded`, `GenerateKeyEx/Opt` |
| `vsomeip/client.ts` | `vsomeip` | Runtime/Application/SomeipMessage/Send, service/event/request/subscribe, callback and periodic lifecycle |

The complete case-sensitive export contract is in `scripts/native-interface-manifest.json`; `scripts/check-native-api.mjs` compares it against actual loaded addons.

## Detailed IPC registry by source

The following registrations are in `src/main/ipc/` and their renderer/plugin callers were extracted from imports and `ipcRenderer.invoke/send` sites:

- `ipc/uds.ts`: `ipc-service-detail`, `ipc-open-script-api`, `ipc-get-build-status`, `ipc-create-project`, `ipc-build-project`, `ipc-get-test-info`, `ipc-run-test`, `ipc-get-test-report`, `ipc-stop-test`, `ipc-get-test`, `ipc-delete-node`, `ipc-delete-tester`, `ipc-get-can-devices`, `ipc-get-eth-devices`, `ipc-get-lin-devices`, `ipc-get-pwm-devices`, `ipc-global-start`, `ipc-global-stop`, `ipc-start-schedule`, `ipc-stop-schedule`, `ipc-get-schedule`, `ipc-run-sequence`, `ipc-stop-sequence`, `ipc-replay-start/stop/pause/resume/get-state`, `ipc-send-can`, `ipc-send-lin`, `ipc-send-eth`, `ipc-send-serial`, `ipc-send-someip-period`, `ipc-start/stop/update-*period`, `ipc-update-can-signal`, `ipc-update-lin-signals`, `ipc-switch-tester-present`.
- `ipc/plugin.ts`: `ipc-plugin-lib-path`, `ipc-get-plugins-dir`, `ipc-open-plugin-path`, `ipc-list-plugin-dirs`, `ipc-plugin-create`, `ipc-plugin-close`, `ipc-plugin-exec`, `ipc-get-remote-plugins`, `ipc-install-remote-plugin`, `ipc-install-plugin-from-zip`; callers are renderer plugin store/SDK and `PluginClient`.
- `ipc/fs.ts`: `ipc-path-parse`, `ipc-path-relative`, `ipc-glob`, `ipc-open-path`, `ipc-fs-readFile/writeFile/readdir/mkdir/exist/stat/rmdir`; callers are project/plugin stores and preload.
- `ipc/serialPort.ts`: `ipc-get-serial-port-list`, `ipc-get-serial-devices`; caller is renderer hardware/serial UI.
- `ipc/canmartix.ts`: `ipc-canmartix-parse`, `ipc-canmartix-exportOtherFile`; caller is project store; implementation is `canmartix.ts` plus optional Python parser.
- `ipc/cdd.ts`/`ipc/odx.ts`: `ipcCddParse`, `ipcCddParseTesterInfo`, `ipcOdxParse`, `ipcOdxParseTesterInfo`; parser subprocess boundary is `python.ts`.
- `ipc/dialog.ts`: `ipc-show-open-dialog`, `ipc-show-save-dialog`, `ipc-show-message-box`, `icp-show-error-box`; Tauri already intercepts the open/save dialog subset.
- `ipc/casdoor.ts`: `ipc-auto-login`, `ipc-get-user-info`, `refreshToken`, `ipc-logout`, `ipc-get-casdoor-config`, `ipc-authenticated-request`.
- `ipc/i18n.ts`: `get-all-translations`, `get-supported-languages`, `set-language`.
- `ipc/pnpm.ts`: `ipc-pnpm-init`, `ipc-pnpm-install`, `ipc-pnpm-uninstall`, `ipc-pnpm-read`; uses bundled `resources/lib/myt`.
- `ipc/var.ts`: `ipc-var-set`, `ipc-signal-set`.
- `ipc/examples.ts`, `ipc/axios.ts`, `ipc/key.ts`, `ipc/update.ts`: example/open-link, HTTP, key and update channels.

For each channel, the migration unit is: preserve name/payload → add typed Core command → run Node/Rust dual comparison → switch renderer caller → remove sidecar handler.

## Proposed typed Rust Core API

```rust
DeviceManager::{discover, list, open, close, state, subscribe}
Transport::{open, close, send, receive, subscribe}
IsoTp::{send, receive, cancel}
Uds::{request, response, timeout, security, transfer}
Xcp::{connect, upload, download, daq_subscribe}
SomeIp::{offer, request, notify, subscribe, stop}
PluginCompat::{exec, emit, stop, close}
```

`#[tauri::command]` functions should only deserialize requests, call these Core services and serialize results. Protocol logic must not be placed in command handlers.
