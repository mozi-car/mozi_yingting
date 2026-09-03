# 并行 Subtask 调度

目标：将剩余 Native 模块拆成相互独立的工作流，允许并行实现；共享构建脚本、Sidecar 路径和状态文件由主流程统一合并，避免冲突。

## 并行组 A：CAN 模块

### A1 — candle

- 负责人范围：`native/candle/**`
- 目标：SetupAPI、WinUSB、控制请求、CAN/CAN-FD、TSFN、resistor、timestamp
- 依赖：无
- 完成标志：真实硬件路径、TypeScript API parity、构建和资源生命周期验证
- 当前：`[~]`

### A2 — peak

- 负责人范围：`native/peak/**`
- 目标：PCAN-ISO-TP DLL、消息 ABI、read/write、mapping、TSFN、cyclic
- 依赖：无
- 完成标志：完整 TypeScript API parity 和 PCAN 验证
- 当前：`[~]`

### A3 — kvaser

- 负责人范围：`native/kvaser/**`
- 目标：canlib DLL、channel、CAN/CAN-FD、timestamp、callbacks
- 依赖：无
- 当前：`[~]`（canlib FFI、Buffer wrapper、TypeScript parity、周期任务已建立；Windows event/TSFN 接收和硬件验证未完成）

### A4 — zlg

- 负责人范围：`native/zlg/**`
- 目标：ZLG vendor API、设备、通道、CAN/CAN-FD、callbacks
- 依赖：无
- 当前：`[~]`（ZLG FFI 和 TypeScript 使用符号 parity 已覆盖；底层接收数组/指针 wrapper、TSFN、硬件验证未完成）

### A5 — vector

- 负责人范围：`native/vector/**`
- 目标：Vector XL API、channel、CAN/CAN-FD、events
- 依赖：无
- 当前：`[~]`（crate/API inventory scaffold，未实现 vendor ABI）

### A6 — toomoss

- 负责人范围：`native/toomoss/**`
- 目标：Toomoss DLL、设备、通道、CAN/CAN-FD、callbacks
- 依赖：无
- 当前：`[~]`（Rust ABI 和 callback 路径已建立；真实硬件/DLL smoke test 未执行，C++/SWIG 参考已恢复）

## 并行组 B：LIN 模块

### B1 — kvaserLin

- 负责人范围：`native/kvaserLin/**`
- 目标：linlib、master/slave、frame、schedule、callbacks
- 当前：`[ ]`

### B2 — peakLin

- 负责人范围：`native/peakLin/**`
- 目标：PEAK LIN API、frame、schedule、callbacks
- 当前：`[ ]`

### B3 — toomossLin

- 负责人范围：`native/toomossLin/**`
- 目标：Toomoss LIN API、frame、schedule、callbacks
- 当前：`[~]`（crate/API inventory scaffold，未实现 vendor ABI）

## 并行组 C：SOME/IP 和基础验证

### C1 — vsomeip

- 负责人范围：`native/vsomeip/**`
- 目标：runtime/application、service、subscribe、request/response/event、shutdown
- 当前：`[ ]`

### C2 — API parity verifier

- 负责人范围：`scripts/check-native-api.mjs`、`.agent/API.md`
- 目标：提取 TypeScript `.node` 使用符号，与 Rust 导出符号对比
- 当前：`[ ]`

### C3 — build/integration verifier

- 负责人范围：`scripts/build-native.mjs`、`scripts/build-sidecar.mjs`、测试脚本
- 目标：统一构建、精确 addon 文件名验证、Node load smoke test
- 注意：实际修改必须由主流程合并，避免与模块 subtask 冲突
- 当前：`[~]`

## 合并规则

1. 每个 subtask 只能修改自己的 `native/<module>/**`，除非属于 C2/C3。
2. 不允许修改或删除其他模块的 C++/SWIG 文件。
3. 不允许用空壳、`false`、空数组或 `available()` 伪造完成。
4. 每个 subtask 必须先完成 API inventory，再实现 ABI，再实现生命周期，最后接入 TypeScript。
5. 主流程逐个审查并更新 `.agent/progress.md`、`.agent/plan.md`。
6. 只有真实功能和验证完成后，状态才能从 `[~]` 改为 `[x]`。

## 实际执行策略

- 可以并行处理“模块代码分析、API 清单、FFI 类型建模、测试准备”。
- 不能把多个会修改同一构建脚本或同一 TypeScript 文件的工作强行并行。
- 当前主线继续推进 `peak`，同时启动 `candle`、`kvaser`、`zlg`、`vector`、`toomoss`、三个 LIN、`vsomeip` 的独立 API inventory。

## 当前加速 Sprint（2026-09-03）

主流程按以下互不重叠的工作包推进。每个工作包必须先在自己的目录完成实现和 cargo build，再由主流程统一执行全量集成测试。

### S1 — PEAK CAN ABI 与生命周期（主流程）
- 范围：`native/peak/**`、`src/main/docan/peak/index.ts`
- 任务：原始 C ABI 结构、消息 alloc/init/read/write/free、时间戳、mapping、callback、cyclic。
- 验收：无 Rust Vec/Buffer 地址传入 PCAN DLL；所有 TS 调用符号和大小写一致；release build。

### S2 — Kvaser CAN/LIN 指针包装
- 范围：`native/kvaser/**`、`native/kvaserLin/**`
- 任务：消除 `cast()` 返回假指针；改为 Rust wrapper 直接承载输出；接收线程和停止/Join 生命周期。
- 验收：`canRead`、`canWrite`、通道枚举、LIN read/write 在无硬件时给出明确 DLL/设备错误，不返回伪造数据。

### S3 — ZLG CAN/CAN-FD 数据数组
- 范围：`native/zlg/**`、`src/main/docan/zlg/index.ts`
- 任务：ReceiveDataArray/FD 数组直接由 Rust 填充；发送 frame 使用显式 setter；错误信息和句柄释放。
- 验收：不再依赖 `ByteArray.frompointer` 假数据或 `output=0`；classic/FD 收发路径均有真实 DLL 调用。

### S4 — Vector XL wrapper
- 范围：`native/vector/**`、`src/main/docan/vector/index.ts`
- 任务：XL driver config、event array、UINT32/UINT8 输出参数、FD 配置和 TSFN 的真实内存传递。
- 验收：移除 `getitem`/`cast` 假实现；接收和发送的 event 字段来自 DLL buffer。

### S5 — Candle WinUSB 完整控制请求
- 范围：`native/candle/**`、`src/main/docan/candle/index.ts`
- 任务：枚举、open/close、termination、timestamp、CAN/CAN-FD 控制请求、接收 worker。
- 验收：所有 bool 返回值对应真实请求结果；失败时保留失败，不用空设备或 false 伪造功能。

### S6 — Toomoss CAN/LIN
- 范围：`native/toomoss/**`、`native/toomossLin/**`、对应 TS 文件
- 任务：校验 vendor struct packing、设备扫描输出、classic/FD/LIN 收发、callback close。
- 验收：发送数据长度和 struct layout 与头文件一致；停止后 worker 可 Join。

### S7 — PEAK LIN
- 范围：`native/peakLin/**`、`src/main/dolin/peak/index.ts`
- 任务：硬件枚举输出参数、GetHardwareParam buffer、CreateTSFN/FreeTSFN、错误码。
- 验收：已注册 client handle 正确回写；无硬件时不会把成功状态当作 handle。

### S8 — SOME/IP、Serial 与集成验证
- 范围：`native/vsomeip/**`、`native/serial/**`、`scripts/**`、`.agent/**`
- 任务：socket/runtime 关闭、callback 清理、serial worker stop、API parity、zero-C++、sidecar load smoke。
- 验收：全量脚本一次通过；不修改其他模块 Native 目录。

### 合并顺序
1. S7 已修复并完成构建/API parity，但保持 `[~]` 直到 PLIN DLL/硬件回归。
2. S2/S3/S4/S5/S6 已完成一轮假指针和伪成功路径清理，继续补齐 vendor regression。
3. 主流程继续完成 S1 的真实 ABI 回归。
4. 最后执行 S8，并只根据真实实现和硬件证据更新 `[x]` 状态。
