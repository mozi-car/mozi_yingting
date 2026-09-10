# Plugin Compatibility Map

## Current call chain

```text
Renderer plugin UI
  -> src/renderer/src/plugin-sdk/index.ts
  -> ipc-plugin-exec / ipc-plugin-create / ipc-plugin-close
  -> src/main/ipc/plugin.ts
  -> src/main/pluginCilent.ts::PluginClient
  -> NodeClass (src/main/nodeItem.ts)
  -> UdsTester (src/main/workerClient.ts)
  -> Node Worker loading plugin script
  -> src/main/plugin-sdk/index.ts / worker SDK
```

## Current host API

| API | Source | Current behavior | Rust destination |
|---|---|---|---|
| `PluginClient.exec(method, ...params)` | `src/main/pluginCilent.ts` | calls `nodeItem.pool.exec('plugin.' + method, params)` | `PluginCompat::exec` |
| `PluginClient.stop()` | `pluginCilent.ts` | clears/re-registers plugin event handler and stops emits | compatibility lifecycle |
| `PluginClient.close()` | `pluginCilent.ts` | closes log and NodeClass/worker | Core task cancellation + compatibility shutdown |
| `registerService(name, fn)` | `src/main/plugin-sdk/index.ts` | worker registers `plugin.<name>` | plugin method registry |
| `emitEvent(name, data)` | worker plugin SDK | emits `{event:'pluginEvent',data:{name,data}}` | typed `CoreEvent::Plugin` adapter |
| `getPluginPath()` | worker plugin SDK | reads `workerData.pluginPath` | capability-scoped plugin context |
| `ipc-plugin-exec` | `src/main/ipc/plugin.ts` / renderer SDK | invokes arbitrary plugin method | typed/compat command |
| `ipc-plugin-create/close` | `src/main/ipc/plugin.ts` | creates/closes plugin NodeClass | compatibility manager |
| `ipc-list-plugin-dirs` | plugin store | filesystem plugin discovery | Rust config/plugin service |

## Plugin-visible APIs

Current plugin SDK re-exports:

```text
worker/uds.ts       Util, diagnostics, variables, signal, test hooks, worker events
worker/someip.ts    SOME/IP request/notify/subscribe helpers
worker/cantp.ts     CAN-TP helpers
worker/crc.ts       CRC helpers
worker/cryptoExt.ts crypto helpers
worker/canopen/     CANopen EDS/NMT/PDO/SDO/EMCY/LSS/SYNC/TIME
```

Listener families include:

```text
OnCan / OnCanOnce / OffCan
OnLin / OnLinOnce / OffLin
OnSerial
OnSignal / OnSignalOnce / OffSignal
OnVar / OnVarOnce / OffVar
OnKey / OnKeyOnce / OffKey
```

## Rust compatibility boundary

```text
Rust Core typed API
  -> PluginCompat JSON/Buffer adapter
  -> existing JavaScript plugin worker
```

The adapter must preserve method names, event names, Buffer semantics, error serialization and stop/close ordering. It must not expose `.node` paths, vendor DLL paths or internal Core locks.

## Migration rules

1. Keep existing JavaScript plugin workers until Rust Core and plugin compatibility tests pass.
2. Give each plugin a capability-scoped `PluginContext`; do not pass the full `Core` object.
3. Convert high-rate frame listeners to bounded subscription/batch APIs.
4. Preserve `pluginEvent` during transition, then add typed plugin events without removing the legacy name.
5. Version the compatibility protocol before changing worker RPC payloads.
6. Plugin migration is separate from DeviceManager/Transport migration.

## Verification

- Existing plugin smoke/load test: `npm run test:plugin-load`.
- Create/exec/emit/stop/close compatibility test for representative plugins.
- Compare Node and Rust responses/errors for the same plugin method calls.
- Verify worker cancellation and no events after `PluginClient.close()`.
