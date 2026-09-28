# Derive 契约与覆盖矩阵

此矩阵区分编译器侧宏契约和 runtime 覆盖率。`scripts/check-derive-coverage.sh` 产生的 derive 行覆盖率目前仅作观察，不设百分比门槛。configure/parse/validate/expand 任一阶段缺少可归属文件或执行计数时，报告必须失败，不能显示为满覆盖。`cargo-llvm-cov` 统计 derive 库测试；由 rustc 执行的过程宏行为仍以 UI 测试验证。

| 契约 | 成功/失败 fixture | feature 或配置 | 预期结果 |
| --- | --- | --- | --- |
| 条件 trait/impl 成员 | `tests/descriptor/conditional_compilation_tests.rs`；`tests/ui/pass/conditional_compilation_tests.rs`；`test-crates/conditional-feature` | host target、feature 开启/关闭、`cfg(any())`、有效 pointer-width `cfg`、`cfg_attr` | 禁用成员消失；启用的 `no_invoke` 元数据保留，但不生成调用适配器。 |
| 禁用成员上的无效 helper | 条件编译 UI pass fixture | helper 位于 `cfg(any())` 下 | 不对禁用 helper 执行语义校验。 |
| 启用成员上的无效 helper | 现有 `tests/ui/fail/` helper 诊断 | default features | 在源码 helper 处编译失败。 |
| 签名和调用策略 | `tests/ui/fail/` 中 invocation、unwind、线程安全 fixture | all-features CI | 不支持签名产生源码位置诊断；支持的策略可编译。 |
| 泛型特化 | `tests/ui/fail/generic_impl_unsatisfied_specialization_tests.rs`；integration fixture | `generic-models` feature | 无效特化编译失败；已注册具体特化保持方法与适配器映射一致。 |
| Facade 与重命名/直接依赖 | `test-crates/model-facade-*`；`test-crates/macro-runtime-only` | facade；关闭 runtime 默认 feature 的直接 derive | 内部支持宏按公开协议解析。 |
| Runtime-only feature 边界 | `test-crates/macro-runtime-only` | runtime 不启用默认 feature | 仍可直接使用 derive 宏，不必启用 runtime derive 导出。 |

覆盖采集器在 `target/derive-coverage/` 保存各阶段源码文件、执行行数、百分比、HTML、原始 llvm-cov JSON 和机器可读摘要。覆盖百分比不代表所有宏展开路径，也不能替代 UI 契约 fixture。
