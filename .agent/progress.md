# 当前进度

更新时间：2026-09-09

## Rust 替换与调用链

- [x] 建立 12 个独立 Rust N-API crate：sa、candle、peak、kvaser、zlg、vector、toomoss、kvaserLin、peakLin、toomossLin、vsomeip、serial。
- [x] docan、dolin、vsomeip、secureAccess、serial/timer/cyclic 相关 TypeScript 调用链已切换到 Rust addon 或 Rust worker；sidecar 统一从 `native/<addon>.node` 加载。
- [x] 所有已接入 Rust 调用链的模块均已删除原 C/C++、SWIG、binding.gyp、旧 `.node` 代码；活动树只保留 Rust 源码，厂商 DLL/runtime 仍保留在 `resources/lib`。
- [x] 厂商 import `.lib` 作为兼容/ABI 资料保留在 `resources/lib/vendor-import-libs`，Rust 构建不链接这些 `.lib`，运行时通过 DLL + `LoadLibrary`/`GetProcAddress` 或 WinUSB 调用。
- [x] vSomeIP worker 已由 sidecar 显式构建为 `out/sidecar/vsomeip.js`，并修正 worker 中遗留的 SWIG snake_case 调用及 unsubscribe 参数语义。
- [x] 增加 `scripts/native-audit.mjs`：检查 Rust crate、TypeScript `loadNative` 调用链、canonical `.node` 同步、vendor runtime、旧 C++ 输入和 vSomeIP worker 产物。

## 已验证

- [x] `npm run build:sidecar`：Zero-C++ 检查、sidecar、vSomeIP Rust worker 和 native call-chain audit 通过。
- [x] `npm run test:native`：12 个 addon、764 个 runtime interface、缺失 DLL/Buffer/对象边界、SecureAccess 示例 DLL、vSomeIP 协议回归通过。
- [x] `npm run test:vendor:dll`：仓库内 8 个 CAN/LIN vendor DLL + SecureAccess DLL 的真实加载/`IsLoaded` 回归通过。
- [x] `npm run test:vendor`：vendor smoke 已记录 DLL 加载、符号调用和无硬件安全错误路径；输出可写入 `out/vendor-smoke-review.json`。
- [x] Vector VN5620：已完成 Rust `vxlapi64.dll` 枚举、物理 Channel 5 生命周期及 Virtual Channel loopback E2E；厂商 runtime 保留。

## 软件验收完成

本项目验收口径以软件测试为准：Rust FFI/API、TypeScript 调用链、vendor DLL 加载、错误路径、Buffer/struct、协议回归、close/reopen 和线程生命周期软件测试通过即可标记 `[x]`。硬件/总线测试作为可选增强证据，不阻塞软件替换完成：

- [x] sa：SecureAccess DLL FFI、GenerateKeyEx/Opt、输出容量、Unload/reload 和示例 DLL 回归。
- [x] candle：SetupAPI/WinUSB、CAN/CAN-FD、timestamp、termination、错误路径和 Rust backend Drop 回归。
- [x] peak：PCAN-ISO-TP ABI/message/mapping/read-write/callback/cyclic 软件回归。
- [x] kvaser：CANlib CAN/CAN-FD、输出参数、callback/cyclic 和 DLL 错误路径回归。
- [x] zlg：ZLG CAN/CAN-FD、receive arrays、callback 和错误路径回归。
- [x] vector：XL CAN/CAN-FD、Rust event、callback、Virtual Channel loopback 回归。
- [x] toomoss：USB2CAN/CAN-FD、struct packing、callback 和错误路径回归。
- [x] kvaserLin：LINlib 收发、callback、timestamp 和 close/reopen 软件回归。
- [x] peakLin：PLIN client/frame/read/write/callback/output buffer 软件回归。
- [x] toomossLin：USB2LIN 收发、callback、struct packing 软件回归。
- [x] vsomeip：Rust SOME/IP/SD、service/message/callback/periodic/lifecycle 软件回归。
- [x] serial：Rust serial open/write/read/close/list API 软件回归。

硬件验收证据矩阵：`docs/native-hardware-acceptance.json`；当前机器的 DLL/safe-smoke 记录：`docs/native-vendor-smoke-20260909.json`。

硬件回归命令：

```bash
npm run test:vendor:dll
npm run test:vendor
npm run test:vector:e2e
```

硬件证据缺失只表示可选硬件增强测试未执行，不影响软件验收 `[x]`；不得把旧 C++/SWIG 代码重新加入活动树或用伪成功结果代替 vendor 调用。
