# API Map

## Current public boundary

### Tauri commands

The Rust host currently exposes a small set of commands:

- `rpc_invoke(channel, args)` — forwards renderer calls to the sidecar, with native dialog interception.
- `rpc_emit(channel, payload)` — sends renderer events to the sidecar.
- `app_info()` — returns host metadata.
- `take_pending_open()` — retrieves a pending `.mytproject` path.
- `read_local_resource(path)` — reads plugin/local resources.

### Sidecar RPC

`src/main/rpc.ts` owns a line-delimited protocol:

```json
{"id":1,"method":"invoke","params":["channel",{"args":[]}]}
{"id":1,"result":{}}
{"id":1,"error":"message"}
{"method":"emit","params":["channel",payload]}
{"method":"ready","params":["win32"]}
```

The channel registry is populated by `src/main/ipc/index.ts` and its imported modules. Important channel groups are filesystem, dialogs, project/config, UDS, serial, plugin, variable, i18n, CAN matrix, ODX/CDD, PNPM and trace services.

### Domain APIs

- `nodeItem` exposes CAN/LIN/DoIP/Serial/PWM/SOME-IP and UDS orchestration to worker scripts.
- `workerClient` exposes script RPC operations such as `output`, `sendDiag`, `setSignal`, `runUdsSeq`, `linApi`, `canApi`, `serialApi` and `someipApi`.
- Native module APIs are checked by `scripts/native-interface-manifest.json` and `scripts/check-native-api.mjs`.

## Proposed Rust Core API

```rust
pub struct Core {
    pub devices: DeviceManager,
    pub transports: TransportRegistry,
    pub diagnostics: DiagnosticService,
    pub plugins: PluginCompat,
    pub events: EventBus,
}

pub trait Transport {
    fn open(&mut self, config: OpenConfig) -> Result<DeviceHandle>;
    fn close(&mut self, handle: DeviceHandle) -> Result<()>;
    fn send(&self, handle: DeviceHandle, frame: Frame) -> Result<()>;
    fn subscribe(&self, handle: DeviceHandle, sink: EventSink) -> Result<Subscription>;
}
```

Core commands should use typed request/response structs rather than generic JSON wherever the UI contract is stable. A compatibility `rpc_invoke` adapter may remain while channels are migrated one group at a time.

## API migration order

1. `app_info`, pending-open, local resources and dialogs remain host commands.
2. Device discovery/open/close/status moves to `DeviceManager` commands.
3. CAN/LIN/Serial operations move to typed Transport commands.
4. ISO-TP and UDS use a shared `DiagnosticService`.
5. Plugin and worker messages use `PluginCompat` until existing plugins have a stable Rust-facing contract.
6. Generic sidecar RPC is deleted only after all channel users are migrated.
