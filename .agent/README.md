# Native API Migration Agent State

本目录是 Native C++/SWIG → Rust Native-API 迁移的持续工作状态目录。

每次继续工作前必须先读取：

1. `requirements.md`
2. `plan.md`
3. `memory.md`
4. `progress.md`
5. `decisions.md`
6. `subtasks.md`

每完成一个可验证阶段，必须更新 `progress.md`；改变方案时更新 `decisions.md`；发现新约束时更新 `memory.md`。并行工作按 `subtasks.md` 的目录边界执行。

## 强制规则

- 不停下来询问用户下一步，按 `plan.md` 继续执行。
- 不把“能编译”当作“模块完成”。
- 只有真实实现、API 对等、资源生命周期正确、TypeScript 集成通过、验证完成后，才能勾选模块。
- 在模块完成前，不删除对应的 C++、SWIG、binding.gyp 或厂商接口文件。
- Native addon 文件名严格保持原名称：`sa.node`、`candle.node`、`peak.node` 等。
- 每个模块必须是独立 Rust crate，不能把所有模块塞入一个 `lib.rs`。
