# Plugin Compatibility Map

## Current plugin contract

`src/main/pluginCilent.ts` wraps a `NodeClass` and exposes:

- plugin construction from a JavaScript file;
- plugin execution through `nodeItem.pool.exec('plugin.<method>', params)`;
- plugin event delivery through `pluginEvent`;
- start/stop/close lifecycle;
- plugin logging through `PluginLOG`.

`src/main/ipc/plugin.ts`, `workerClient.ts` and `nodeItem.ts` provide the surrounding handler and device APIs. Existing plugins may assume Node modules, filesystem paths, timers, Buffer and the current channel names.

## Compatibility boundary

```text
Rust Core
  │ typed device/transport/diagnostic API
  ▼
PluginCompat
  │ stable legacy names and JSON/Buffer conversion
  ▼
Existing JavaScript plugin worker
```

The compatibility layer must remain until plugin consumers have a versioned Rust-facing API. It should not expose vendor DLL paths or N-API object instances.

## Migration rules

1. Preserve plugin method names and event names first.
2. Convert Core events to the existing `pluginEvent` envelope during the transition.
3. Give each plugin a capability-scoped handle rather than the full Core object.
4. Keep plugin workers isolated from Core memory and shutdown them with a cancellation token.
5. Version the compatibility protocol before removing Node runtime support.
6. Do not migrate the plugin ecosystem in the same change as DeviceManager or Transport.

## Candidate Rust-facing plugin API

```text
plugin.open_device(id)
plugin.send_frame(transport, frame)
plugin.subscribe(filter)
plugin.request_diagnostic(request)
plugin.emit(event)
plugin.close()
```

The first implementation may serialize these calls through the existing Node compatibility worker; the long-term implementation should call Rust Core commands/events directly.
