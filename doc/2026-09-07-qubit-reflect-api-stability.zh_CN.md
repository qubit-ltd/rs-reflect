# `qubit-reflect` API 稳定性

<!-- reflect-contract: dispatch.input_recovery=original-input -->

- 日期：2026-09-07
- 状态：当前内部 `0.2.0` 源码开发线的兼容性政策；本文不验证 registry 发布状态
- 范围：反射运行时、derive 生成代码、扩展 capability 与实现细节
- 用户手册：[中文版](user_guide.zh_CN.md) · [English](user_guide.md)

本政策明确 `qubit-reflect` 的四类使用者和扩展者所面对的兼容性边界。
本政策针对当前内部 `0.2.0` 源码开发线；在明确公告稳定发布政策之前，
可以协调实施破坏这些边界的变更，但必须同步更新运行时、派生宏、下游外观库
及其契约测试。表中的路径是代表性示例；决定预期稳定级别的是边界及其承诺，
而不是某一个文件名本身。只有未来稳定发布线的政策明确公告时，才承诺
迁移窗口或继续支持旧协议。

| 层级 | 示例路径 | 兼容承诺 | 允许的变更 | 迁移要求 |
| --- | --- | --- | --- | --- |
| Stable Application | `src/lib.rs`、`src/descriptor/`、`src/value/`、`tests/descriptor/` | 在已公告的稳定发布线中，公共 descriptor、typed view、受检操作、结构化错误类别和文档化宏入口，在同一 minor release 系列内保持源码兼容。除非 API 明确说明，稳定身份只在当前进程内有效。 | 公开稳定版发布前允许协调的破坏性变更；在稳定发布线中，新增公共 API、typed view、错误细节及修复必须保持安全性和既有有效结果。 | 应匹配公共错误类别而不是 Display 文本；不得把 `type_name()`/`TypeId` 放入持久化协议；应在下一条 breaking release 前采用弃用项。 |
| Extension Author | `src/capability/`、`src/identity/capability_id.rs`、`tests/descriptor/capability_tests.rs` | 带命名空间的 `CapabilityId`、typed `CapabilityKey`、adapter 契约和确定性的 capability 枚举构成扩展边界。扩展不得获得绕过检查的 descriptor 私有状态访问。 | 公开稳定版发布前，既有 ID 或 adapter 契约的变更须与下游使用者协调；在稳定发布线中，可新增自有命名空间的 capability、安全 adapter、诊断及 opt-in helper，但不得改变既有契约。 | ID 始终由扩展自身持有；在已公告的稳定发布线中保持 adapter 类型契约；替换 capability 或 adapter 时必须发布兼容性说明。 |
| Versioned Codegen | `derive/src/`、`src/private/codegen_v3/`、`derive/tests/`、`test-crates/model-facade-app/` | 生成代码与其私有协议成对版本化。`__private::codegen_v3` 这类协议只与声明它的运行时 generation 兼容；公共宏行为由 compile-pass/fail 测试持续覆盖。 | 公开稳定版发布前，协调后的变更可以替换或移除旧生成协议，无须保留兼容层。只有发布政策明确公告迁移窗口时，才在该窗口内保留旧 generation。 | 必须同步更新运行时、派生宏、下游 `rs-model-metadata` 外观库及其契约测试；重跑 derive 和下游 facade 测试、更新固定的 runtime/derive 配对，并在协议 generation 变化时迁移生成产物。 |
| Internal | `src/private/`、`src/registry/interner.rs`、`tests/internal/`、`benches/` | 内部缓存、interner、登记 plumbing、benchmark 布局和仅测试使用的 helper 不承诺下游兼容性，但仍须保持已文档化的公共安全与确定性。 | 在公共行为和安全契约不变时，可以重构、拆分、替换或删除内部实现。 | 使用者无需迁移；维护者必须在同一变更中更新内部测试、benchmark 和追踪证据。 |

## 2026-09-29：调用返回类型的破坏性变更

本次变更破坏源码兼容性。`descriptor::InvocationAdapter` 和
`MethodInstanceDescriptor` 各有六个消费调用方法，将外层 `Option` 改为
`InvocationDispatchResult<I, R>`，共十二个：`invoke_local`、
`invoke_thread_safe`、`invoke_catching_local`、`invoke_catching_thread_safe`、
`invoke_pinned_ref_local`、`invoke_pinned_mut_local`。既有 `None`/`Some` 分支和
`Option` 便利操作都必须迁移；此处不承诺兼容层。

外层 `Err(InvocationUnavailable<I>)` 在绑定与执行前保留完整原始输入。
`reason()` 给出分派原因，`into_invocation()` 或 `into_parts()` 用于取回输入。
外层 `Ok` 保留原有校验和方法 panic 的结果层：普通校验失败仍走
`InvocationRecovery` 或 pinned recovery，catching 仍将 `InvocationPanic`
与校验错误分开。Pinned 接收者的 `T` 必须精确匹配；类型不符会返还原始
`Pin` 和调用方顺序的绑定。进入方法后，不恢复已消费输入，也不撤销副作用。

运行时函数指针别名（包括 `invoke::InvocationAdapter`）、生成适配器 ABI、
`__private::codegen_v3`、模型 ABI v7 及 `definition_provider_v2` 均不变。
描述符适配器类型与运行时函数指针别名是两个不同 API。本说明描述当前源码，
不能作为候选版本已从 registry 发布的证据。

## 如何使用本政策

普通使用者默认位于 Stable Application 边界。扩展作者应把 capability ID
和 adapter 类型视为自己的兼容性表面；Versioned Codegen 则明确允许协调的
runtime/derive/facade 版本变更。Internal 名称可以自由变化，但如果变化泄漏到公共
路径或生成 token stream，就必须按对应的更高稳定级别审查。

需求规范和追踪矩阵记录这些边界的可执行证据。特别是，`REQ-TYPE-030`
要求严格 capability 查询保留 `Missing`、`FactOnly`、`AdapterTypeMismatch` 和 `Found`
四种状态；可失败的便利查询将 `Missing` 映射为 `Ok(None)`、`Found` 映射为
`Ok(Some(adapter))`，而 `FactOnly` 和 `AdapterTypeMismatch` 仍是错误。注册表的
`capability_origin` 与 `capability_source` 另外保留能力来自 intrinsic 还是注册片段的来源信息。
