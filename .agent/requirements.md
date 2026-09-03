# 需求基线

## 总目标

将项目中自有的 C++ / SWIG / node-gyp Native 模块逐个迁移为 Rust Native-API 模块，保持现有 TypeScript 业务层行为和 addon 文件名不变。

## 模块清单

1. `sa.node`：SecureAccess DLL seed/key 模块
2. `candle.node`：CandleLight WinUSB CAN/CAN-FD 模块
3. `peak.node`：PEAK PCAN-ISO-TP 模块
4. `kvaser.node`：Kvaser CAN 模块
5. `zlg.node`：ZLG CAN 模块
6. `vector.node`：Vector CAN 模块
7. `toomoss.node`：Toomoss CAN 模块
8. `kvaserLin.node`：Kvaser LIN 模块
9. `peakLin.node`：PEAK LIN 模块
10. `toomossLin.node`：Toomoss LIN 模块
11. `vsomeip.node`：SOME/IP 模块

## 必须保持

- 原始 addon 名称及加载语义
- TypeScript 现有调用方式和导出符号
- Windows 厂商 DLL / WinUSB 的真实调用
- 原有错误码、Buffer、时间戳、CAN/CAN-FD、LIN、SOME/IP 语义
- 线程、回调、句柄、动态库生命周期

## 禁止

- 用 `false`、空数组、空字符串或 `available()` 伪造未实现功能
- 只创建能编译的空壳就删除 C++
- 用一个通用 crate 代替多个独立模块
- 将 SWIG 生成代码行数等同于业务逻辑完成度
- 未验证硬件/厂商 DLL 行为就勾选完成
