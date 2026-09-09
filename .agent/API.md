# API 对等记录

`npm run test:native` 会在重建全部 addon 后，从 TypeScript 实际调用点提取符号，并检查 Rust addon 的顶层导出及 class prototype。源码/API parity 不是厂商行为等价证明。

| Addon | 状态 | 当前 API 检查 |
|---|---|---|
| sa.node | `[x]` | `SeedKey`、LoadDLL、Unload、IsLoaded、GenerateKeyEx/Opt |
| candle.node | `[x]` | 18 个 TypeScript 实际使用导出 |
| peak.node | `[x]` | 119 个 TypeScript 实际使用导出，包含 PCAN 常量、消息类和 CANTP 函数 |
| kvaser.node | `[x]` | 51 个实际使用导出 |
| zlg.node | `[x]` | 31 个实际使用导出 |
| vector.node | `[x]` | 33 个实际使用导出 |
| toomoss.node | `[x]` | 23 个实际使用导出 |
| kvaserLin.node | `[x]` | 17 个实际使用导出 |
| peakLin.node | `[x]` | 31 个实际使用导出 |
| toomossLin.node | `[x]` | 15 个实际使用导出 |
| vsomeip.node | `[x]` | 22 个实际使用导出/原型方法 |
| `serial.node` | `[x]` | `Serial`、list、open/write/read/close 软件 API |

## 可选硬件证据

软件验收要求每个模块通过真实 vendor DLL 加载（或 WinUSB/系统 API）、API parity、错误路径、Buffer/struct、协议、callback、close/reopen 和线程生命周期回归，即可标记 `[x]`。硬件/总线验收是可选增强证据，不阻塞软件迁移；旧 C++/SWIG 实现已删除，不得重新加入活动树。
