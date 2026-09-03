# 决策记录

## D-001：每个 addon 独立 crate

决定：每个原生模块单独建立 Rust crate。

原因：模块边界清晰、独立构建、独立验证，避免一个巨大 `lib.rs`。

## D-002：保留原 addon 文件名

决定：Rust crate 的 package name 可以使用合法 Rust 名称，但复制到 sidecar 时必须使用原 addon 名称。

示例：`secure-access` crate 输出 `sa.node`，不能输出 `rust-sa.node`。

## D-003：未完成不删除旧实现

决定：只有模块通过 API、功能、生命周期、构建和集成验证后，才删除其 C++/SWIG/binding.gyp 文件。

## D-004：构建脚本不负责制造假完成

决定：`build-native.mjs` 只构建已存在且有实际实现的 Rust crate；未迁移模块不生成 `available() -> false` 的伪 addon。

## D-005：以 TypeScript 使用面为第一版 API 范围

决定：迁移一个模块时，先从 TypeScript 收集实际使用的符号，再对照原头文件和 C++ wrapper 完成 ABI；不能只凭少量函数名做接口壳。

## D-006：完成标准

决定：构建通过只是必要条件，不是完成条件。完成必须包含真实厂商 API/硬件路径、错误语义、线程和资源生命周期、TypeScript sidecar 集成。

## D-007：并行 Subtask 边界

决定：可按独立 crate 并行推进所有模块；每个 subtask 只能修改自己的 `native/<module>/` 目录。共享构建脚本、TypeScript 入口、状态文件由主流程统一修改和审查，避免并发覆盖。

并行不代表降低完成标准；每个模块仍必须单独通过 API parity、真实调用、生命周期和集成验证。
