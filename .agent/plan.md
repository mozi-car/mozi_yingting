# 执行计划

## 已完成的基础工作

- [x] 建立独立 Rust Native-API 构建脚本
- [x] 保持原 addon 文件名及 sidecar 加载路径
- [x] 建立统一 API parity / behavior regression / zero-C++ active-tree 检查
- [x] 所有 11 个模块建立独立 Rust crate 并通过 Windows GNU release build
- [x] 完整 `npm run build` 和 NSIS installer build

## 当前模块状态

以下模块已建立真实 DLL/WinUSB 调用路径，但未连接厂商 DLL/硬件完成行为回归，因此保持 `[~]`：

- `[~]` sa.node：SecureAccess DLL LoadLibrary/GetProcAddress/GenerateKeyEx
- `[~]` candle.node：SetupAPI/WinUSB/control transfer/CAN frame/worker
- `[~]` peak.node：PCAN-ISO-TP ABI/message/mapping/read-write/callback/cyclic
- `[~]` kvaser.node：CANlib/channel/CAN-CANFD/read-write/callback/cyclic
- `[~]` zlg.node：ZLG device/channel/CAN-CANFD/receive arrays/callback
- `[~]` vector.node：XL driver/channel/CAN-CANFD/event/callback
- `[~]` toomoss.node：USB2CAN/CAN-CANFD/struct/callback
- `[~]` kvaserLin.node：linlib/LIN frame/read-write/callback
- `[~]` peakLin.node：PLin client/hardware/LIN frame/callback
- `[~]` toomossLin.node：USB2LIN/LIN frame/read-write/callback
- `[~]` vsomeip.node：SOME/IP socket runtime/service/message/callback/lifecycle

## 第 1 批已修复内容

- [x] PEAK 消息不再将 Rust `Vec`/Buffer 对象地址直接传给 DLL，改用独立 C ABI 结构
- [x] PEAK 顶层 CANTP 导出、常量、时间戳和消息数据访问已补齐
- [x] PEAK callback 不再用空 `Vec` 作为假消息通知
- [x] Candle termination/timestamp 改为真实 WinUSB 请求
- [x] Vector 接收改为 Rust event wrapper，去除业务层假 pointer 读取
- [x] ZLG receive arrays 改为 Rust 直接填充，发送改用显式 frame setter
- [x] Kvaser 输出参数和 Buffer 调用改为传递 wrapper 对象
- [x] PEAK LIN client、hardware-name buffer、TSFN 生命周期已补齐
- [x] Toomoss CAN-FD 配置字段按 vendor header 修正为字节布局
- [x] 缺失 DLL 不再返回伪成功

## 第 2 批：硬件验证门禁

1. 为每个模块设置 `YT_VENDOR_DLL_<MODULE>` 并执行 `npm run test:native`。
2. 在目标设备上验证 scan/open/config/send/receive/error/close/worker Join。
3. 保存 vendor DLL/设备测试日志，确认返回码、Buffer、timestamp、struct packing。
4. 每模块通过后才把 `.agent/progress.md` 对应项从 `[~]` 改为 `[x]`。
5. 全部通过后删除对应旧 C++/SWIG 参考目录，恢复的参考实现不得提前删除。
