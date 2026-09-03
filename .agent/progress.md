# 当前进度

更新时间：2026-09-03

## 已完成

- [x] Rust Native 构建脚本和 sidecar 原 addon 文件名保持机制
- [x] 11 个 Native crate 均可使用 Windows GNU release 构建
- [x] sidecar 使用统一 `native/<原文件名>.node` 加载
- [x] API parity 检查会重建/加载 addon，并从 TypeScript 实际调用点检查导出符号
- [x] Native wrapper regression：11 个 addon 的加载、缺失 DLL 错误、基础 Buffer/对象边界测试
- [x] vSomeIP 协议回归测试
- [x] 完整 `npm run build`：native、sidecar、renderer、Tauri NSIS installer 均通过（2026-09-03 最新构建）
- [x] 使用仓库内 vendor DLL 完成 DLL LoadLibrary、safe enumeration/error smoke；日志保存于 `docs/native-vendor-smoke-20260903.json`
- [x] Vector VN5620 实机已被 `vxlapi64.dll` 枚举：9 个 VN5620 通道，serial `13455`；CAN Channel 5/6 可识别，Channel 5 已完成 `xlOpenPort → xlActivateChannel → xlCanReceive(no frame) → xlDeactivateChannel → xlClosePort`，结果已写入 smoke log
- [x] CLI 部署目录同步全部 `.node`：`npm run build:cli && node out/cli/myt.cjs --help` 通过
- [x] 修复打包 EXE 的 resource root：`target/release/sidecar` 现在正确解析到 `target/release/resources`，不再错误寻找 `target/resources`
- [x] Vector CAN channel bus capability 按 XL bitmask 正确识别；VN5620 handle 改为包含 hwType/hwIndex/channelIndex/hwChannel
- [x] ETH tab 增加 Vector Ethernet channel discovery；按 `XL_BUS_COMPATIBLE_ETHERNET` 动态显示/消失，未知 USB/PnP 不再伪装为 simulate
- [x] 硬件树首次加载时所有总线驱动自动展开；用户手动收起后，轮询刷新不会再次展开
- [x] ETH 标签改为 `ETH`，DoIP 作为其上的协议；ETH/Serial 增加独立 `PC` 分类，普通电脑网卡/串口不再归入 SIMULATE
- [x] 已配置通道卡片优先显示驱动返回的硬件名称，不再用用户输入的通道名称覆盖 VN5620 等硬件名
- [x] 误生成的 `NUL` 文件已删除
- [x] 尚未完成模块的原 C++/SWIG 参考实现已恢复，作为只读迁移参照；它们不参与 Rust sidecar 构建
- [x] Vector LIN 实际 TypeScript 调用已接入 Rust XL LIN ABI（channel params、DLC、slave、request、wakeup、LIN event）
- [x] ZLG receive array 和 Toomoss/Peak LIN 数据字段已从假 pointer 访问改为 Rust-owned buffer/显式 ABI 参数
- [x] PEAK/Vector TSFN 不再预读并吞掉 vendor 队列消息；JavaScript handler 负责实际 Read

## 已实现但仍需厂商 DLL/硬件回归

- [~] `sa.node`：LoadDLL、IsLoaded、GenerateKeyEx、GenerateKeyExOpt；错误码/输出容量和真实算法 DLL 仍需验证
- [~] `candle.node`：SetupAPI、WinUSB open/close、控制请求、termination、timestamp、CAN/CAN-FD 收发和接收线程；真实 CandleLight 硬件仍需验证
- [~] `peak.node`：PCAN-ISO-TP 原始 ABI 结构、消息分配/初始化/读写/释放、mapping、时间戳、callback 和 cyclic；真实 PCAN DLL/消息布局仍需验证
- [~] `kvaser.node`：canlib DLL、CAN 收发、输出参数 wrapper、callback/cyclic；真实 canlib DLL/硬件仍需验证
- [~] `zlg.node`：ZLG DLL、设备/通道、CAN/CAN-FD、Rust-owned receive arrays、callback；真实 ZLG DLL/硬件仍需验证
- [~] `vector.node`：XL driver/config、CAN/CAN-FD、Rust-owned event 收发、callback；VN5620 枚举/open/activate/receive-empty/close 已实测，尚缺实际总线帧、错误帧、时间戳和 callback 收发证据
- [~] `toomoss.node`：设备、CAN/CAN-FD、struct packing、callback；真实 USB2CAN DLL/硬件仍需验证
- [~] `kvaserLin.node`：linlib DLL、LIN 收发、callback；真实 LIN 硬件仍需验证
- [~] `peakLin.node`：PLin API、client/output buffer、LIN 收发、callback；真实 PLin DLL/硬件仍需验证
- [~] `toomossLin.node`：Toomoss LIN struct、收发、callback；真实 USB2LIN DLL/硬件仍需验证
- [~] `vsomeip.node`：SOME/IP/SD socket runtime、service、request/response/event、callback、周期发送和关闭；与目标 vsomeip 版本/系统配置的互操作仍需验证

## 未完成原因与禁止事项

- 当前环境已连接 Vector VN5620；已验证打包 sidecar 的 CAN/ETH IPC 枚举和 Channel 5 open/config/activate/close。当前没有 CAN 总线对端/可确认的测试帧；其余 vendor 设备也未连接，不能伪造 11 个模块的“硬件行为等价”结论。
- `scripts/native-regression.mjs` 在设置 `YT_VENDOR_DLL_<MODULE>` 时会执行真实 DLL LoadDll 检查；设备收发回归需在硬件 CI/目标机执行。
- 在上述回归完成前，不得删除恢复的 C++/SWIG 参考树，也不得把 `[~]` 改成 `[x]`。
