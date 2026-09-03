# API 对等记录

`npm run test:native` 会在重建全部 addon 后，从 TypeScript 实际调用点提取符号，并检查 Rust addon 的顶层导出及 class prototype。源码/API parity 不是厂商行为等价证明。

| Addon | 状态 | 当前 API 检查 |
|---|---|---|
| sa.node | `[~]` | `SeedKey`、LoadDLL、IsLoaded、GenerateKeyEx/Opt |
| candle.node | `[~]` | 18 个 TypeScript 实际使用导出 |
| peak.node | `[~]` | 119 个 TypeScript 实际使用导出，包含 PCAN 常量、消息类和 CANTP 函数 |
| kvaser.node | `[~]` | 51 个实际使用导出 |
| zlg.node | `[~]` | 31 个实际使用导出 |
| vector.node | `[~]` | 33 个实际使用导出 |
| toomoss.node | `[~]` | 23 个实际使用导出 |
| kvaserLin.node | `[~]` | 17 个实际使用导出 |
| peakLin.node | `[~]` | 31 个实际使用导出 |
| toomossLin.node | `[~]` | 15 个实际使用导出 |
| vsomeip.node | `[~]` | 22 个实际使用导出/原型方法 |

## 仍需外部证据

每个模块必须在真实 vendor DLL 和硬件上验证 LoadDll、枚举、open/config、收发、时间戳、错误码、callback、close/reopen 和线程 Join。未提供这些证据前，不能将 `[~]` 改为 `[x]`，也不能删除该模块的 C++/SWIG 参考实现。
