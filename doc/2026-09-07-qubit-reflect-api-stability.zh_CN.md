# `qubit-reflect` API 稳定性

- 日期：2026-09-07
- 状态：规范性的内部兼容性政策
- 范围：反射运行时、derive 生成代码、扩展 capability 与实现细节

本政策明确 `qubit-reflect` 的四类使用者和扩展者所面对的兼容性边界。
表中的路径是代表性示例；决定稳定级别的是边界及其承诺，而不是某一个
文件名本身。

| 层级 | 示例路径 | 兼容承诺 | 允许的变更 | 迁移要求 |
| --- | --- | --- | --- | --- |
| Stable Application | `src/lib.rs`、`src/descriptor/`、`src/value/`、`tests/descriptor/` | 公共 descriptor、typed view、受检操作、结构化错误类别和文档化宏入口，在同一 minor release 系列内保持源码兼容。除非 API 明确说明，稳定身份只在当前进程内有效。 | 可以新增公共 API、typed view、错误细节和不改变安全性及既有有效结果的修复。 | 应匹配公共错误类别而不是 Display 文本；不得把 `type_name()`/`TypeId` 放入持久化协议；应在下一条 breaking release 前采用弃用项。 |
| Extension Author | `src/capability/`、`src/identity/capability_id.rs`、`tests/descriptor/capability_tests.rs` | 带命名空间的 `CapabilityId`、typed `CapabilityKey`、adapter 契约和确定性的 capability 枚举构成扩展边界。扩展不得获得绕过检查的 descriptor 私有状态访问。 | 可以新增自有命名空间的 capability 和安全 adapter，也可以新增诊断及 opt-in helper，但不得改变既有 ID 或 adapter 契约。 | 扩展必须永久持有其 ID、保持 adapter 类型契约；替换 capability 或 adapter 时必须发布兼容性说明。 |
| Versioned Codegen | `derive/src/`、`src/private/codegen_v3/`、`derive/tests/`、`test-crates/model-facade-app/` | 生成代码与其私有协议成对版本化。`__private::codegen_v3` 这类协议只与声明它的运行时 generation 兼容；公共宏行为由 compile-pass/fail 测试持续覆盖。 | 可以引入新的私有协议 generation、改进诊断并增加生成元数据；已公告迁移窗口内保留旧 generation。 | 必须重跑 derive 和下游 facade 测试，更新固定的 runtime/derive 配对；协议 generation 变化时迁移生成产物。 |
| Internal | `src/private/`、`src/registry/interner.rs`、`tests/internal/`、`benches/` | 内部缓存、interner、登记 plumbing、benchmark 布局和仅测试使用的 helper 不承诺下游兼容性，但仍须保持已文档化的公共安全与确定性。 | 在公共行为和安全契约不变时，可以重构、拆分、替换或删除内部实现。 | 使用者无需迁移；维护者必须在同一变更中更新内部测试、benchmark 和追踪证据。 |

## 如何使用本政策

普通使用者默认位于 Stable Application 边界。扩展作者应把 capability ID
和 adapter 类型视为自己的兼容性表面；Versioned Codegen 则明确允许协调的
runtime/derive 版本变更。Internal 名称可以自由变化，但如果变化泄漏到公共
路径或生成 token stream，就必须按对应的更高稳定级别审查。

需求规范和追踪矩阵记录这些边界的可执行证据。特别是，`REQ-TYPE-030`
要求严格 capability 查询保留 missing、fact-only、adapter mismatch 和 found
四种状态；旧的 `Option` 便利方法仍可以有意折叠这些状态。
