# 持久记忆与约束

## 项目事实

- 工作目录：`D:/500-project/101-pesionProject/mozi/mozi_yingting`
- 构建目标当前为 Windows GNU：`x86_64-pc-windows-gnu`
- Rust N-API 使用 napi 3.x。
- Native 文件必须放到 sidecar 的 `native/` 目录。
- `build-sidecar.mjs` 会把 `.node` 引用转为运行时从 `__dirname/native/<原文件名>` 加载。

## 重要纠正

- SWIG 生成的数千行 wrapper 不是业务实现；迁移时必须区分“手写业务 C++”与“自动生成绑定代码”。
- “Rust crate 能编译”不等于“模块完成”。
- `native/candle` 当前曾有多个返回 `false` / 空值的骨架函数，不能作为完成版本。
- `native/peak/src/lib.rs` 已覆盖当前 TypeScript 检查到的 119 个导出/原型符号并使用独立 raw ABI 结构；真实 PCAN DLL/硬件行为仍未验证，不能删除 PEAK 参考实现。
- 未完成模块不得从构建或运行时路径中伪装成已迁移模块；缺失 DLL、设备和周期发送前置条件必须返回错误。

## API 名称规则

Rust napi 默认会把 snake_case 转成 camelCase；若原 JS API 需要保留大小写/下划线，必须使用 `#[napi(js_name = "原名称")]`。

## 验证规则

每个模块完成前至少验证：

1. Rust release build。
2. `node require()` 能加载精确原名称。
3. 导出符号与 TypeScript 使用集合一致。
4. 动态库/设备真实调用路径存在。
5. 错误和资源释放行为有测试或明确验证。
6. sidecar build 成功。

## 当前原实现状态

- `sa` 原手写实现很小，主要是 LoadLibrary/GetProcAddress 和两个函数调用；SWIG wrapper 很大但属于绑定层。
- `candle` 原实现包含 SetupAPI、WinUSB、控制请求、帧收发和 TSFN，迁移不能只复制接口。
- `peak` 原实现依赖 PCAN-ISO-TP DLL；TypeScript 使用了大量常量、结构体、消息 API、mapping、周期发送和回调。
