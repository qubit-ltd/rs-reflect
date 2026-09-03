# `qubit-reflect` 详细设计

- 日期：2026-09-01
- 最近审核：2026-09-02
- 状态：破坏性边界重构已实现，并通过分阶段验证；完整发布矩阵由仓库 CI 持续执行
- 英文版：[English design](2026-09-01-qubit-reflect-design.md)
- 依据：[最终需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md)
- 适用仓库：`rs-reflect`
- 对应协议：`qubit-reflect 0.1` / `__private::codegen_v1`

## 1. 目的与边界

`qubit-reflect` 提供业务无关的静态反射描述、受检动态访问、调用、构造与链接期注册。模型元数据、
校验、编解码和脱敏属于下游语义；依赖方向始终是 `rs-model-* -> qubit-reflect`。

设计从五个不可妥协的不变量出发：

1. 公开 API 不能构造结构非法的表达式或描述符。
2. 宏先完成解析、校验和语义分析，再生成 token。
3. 运行时内核不隐式绑定生态类型或 Qubit 业务类型。
4. 生成代码只依赖窄且版本化的隐藏协议。
5. 声明支持的 feature 组合必须在 package 粒度真实执行。

本轮允许破坏公开字段和隐藏 ABI，不保留兼容别名。破坏只用于消除非法状态、依赖污染或生成协议耦合，
不重新设计已经稳定的动态值、registry 查询和失败恢复语义。

## 2. 仓库与依赖架构

### 2.1 双 crate

```text
rs-reflect/
├── Cargo.toml                 # qubit-reflect runtime + workspace
├── src/
├── derive/                    # qubit-reflect-derive proc-macro
├── tests/
├── test-crates/               # facade 与跨 crate 注册夹具
├── fuzz/
├── benches/
└── doc/
```

Rust 要求过程宏位于 `proc-macro` crate，因此 runtime 与 derive 保持两个发布单元。derive 不依赖 runtime，
只生成指向调用方 facade 的路径，避免 Cargo 环。

不创建第三个 codegen crate。只有同时出现至少两个独立宏 crate、共享同一纯分析逻辑、存在可测的重复实现，
且拆分能减少而非转移诊断 span 与发布复杂度时，才重新评估该选择。当前下游模型宏只消费版本化 runtime 工厂，
不共享 reflect derive 的分析器，因此未达到门槛。

### 2.2 feature 与依赖预算

| 配置 | 含义 | 直接影响 |
| --- | --- | --- |
| `default = ["derive"]` | 常规宏使用 | 启用 `qubit-reflect-derive` |
| `default-features = false` | 纯 runtime | 不编译 derive 与外部类型族 |
| `ecosystem-types` | 生态类型反射 | 显式启用 `bigdecimal`、`chrono`、`uuid` |
| `qubit-types` | Qubit 类型反射 | 显式启用 `qubit-datatype`、`qubit-id` |
| `bench-internals` | 内部基准入口 | 不改变普通用户 API |

核心 runtime 的直接依赖是 `inventory` 与 `thiserror`。外部类型依赖全部 optional。Rust 的 orphan rule
要求 `Reflect` 的外部类型实现位于拥有 trait 的 `qubit-reflect` crate；feature 模块既满足该规则，
又避免把这些实现和依赖塞入默认内核。

derive 的直接依赖限于 `proc-macro2`、`quote`、`syn` 和 `proc-macro-crate`。
仓库使用 Rust 2024，MSRV 为 1.94，runtime 与生成代码均禁止 unsafe。

## 3. runtime 模块

```text
src/
├── access/                     # 字段与 variant 动态访问及恢复
├── builtin/
│   ├── ecosystem.rs            # cfg(feature = "ecosystem-types")
│   ├── qubit.rs                # cfg(feature = "qubit-types")
│   └── internal/               # 内建实现共享宏
├── capability/                 # 稳定 capability ID、adapter 与集合
├── construct/                  # 命名、元组、unit 构造与 update
├── descriptor/                 # 类型、字段、variant、trait、impl、method
├── error/                      # registry 与类型错误
├── expression/                 # 受检结构化类型/const/predicate 表达式
├── identity/                   # fragment、member、trait、visibility 身份
├── invoke/                     # 动态调用、future 与失败恢复
├── private/
│   └── codegen_v1/             # 唯一生成协议入口
├── registry/
│   ├── registry_builder.rs     # 冲突检查与冻结前聚合
│   ├── registry.rs             # 只读查询
│   └── effective_type_view.rs  # 已解析的 impl/method 视图
└── value/                      # local/thread-safe 动态值模式
```

descriptor 与 expression 的结构字段保持私有。公开构造器在边界验证空路径、并行 slice 长度、
重复 identity 等不变量；只读访问器暴露导航事实。诊断文本不参与 `Eq` 或 `Hash`，
结构身份不会随格式化文本变化。

registry 先收集全部 fragment，再检查重复 identity、类型/trait/impl/capability 冲突，最后一次性冻结索引。
失败不会发布部分状态；候选和索引顺序由稳定 fragment 顺序决定。复杂有效方法视图在冻结阶段构造，
查询路径不执行用户代码。

## 4. derive 流水线

```text
proc-macro entry
    │
    ▼
parse ──> validate ──> domain IR
                           │
                           ▼
                    expansion dispatcher
                           │
          ┌────────────────┴────────────────┐
          ▼                                 ▼
 descriptor orchestration          invocation analysis
                                             │
                                             ▼
                                      InvocationPlan
                                             │
                                             ▼
                                        token emitter
```

- `derive/src/entry.rs` 统一入口错误聚合。
- `parse/` 只把 `syn` 输入转换为领域 IR。
- `validate/` 检查声明形状与属性组合。
- `expand/context.rs` 唯一负责 facade 路径和 fragment fingerprint。
- `expand/dispatcher.rs` 分派 struct、enum、trait、impl。
- `expand/expression_codegen.rs` 统一负责结构表达式 token 生成。
- `expand/impls/specialization_codegen.rs` 隔离泛型 impl 特化与 token 替换。
- `expand/invocation/analysis.rs` 判定 receiver、参数、输出、线程安全、catching、async 与不可用原因。
- `expand/invocation/emit.rs` 只根据 `InvocationPlan` 生成不可用原因、参数绑定与
  thread-safe/catching 断言等共享语义片段。

trait 默认方法与 concrete impl 共用同一 invocation 分析和语义 emitter。两者的执行壳保持在各自所有者中：
前者必须生成 `Self` 约束下的默认方法 hook，后者必须生成具体 target 的 adapter 与 registration。把这两种
所有权壳强塞进一个带大量模式开关的 emitter 只会转移复杂度，不建立新抽象。宿主宏代码不向 `quote!` 内生成路径的
风格规则妥协；style checker 明确区分宿主 Rust 与 generated-token scope。

## 5. 版本化生成协议与 facade

生成代码只能通过以下版本化协议根访问 runtime 类型、工厂和注册钩子：

```text
facade::__private::codegen_v1
```

协议按领域精确暴露 `access`、`capability`、`construct`、`descriptor`、`error`、
`expression`、`identity`、`invoke`、`registration`、`value`，以及生成 impl 所需的根反射类型。
这些是生成协议符号，不是供业务代码手写调用的 API；协议也不会重导出 runtime 的完整公开模块。
根 `__private` 不平铺这些符号。协议发生不兼容变化时新增兄弟版本，而不是静默扩大 v1。

下游 facade 为生成代码精确暴露 `__private::codegen_v1`，并独立逐项重导出它向业务代码承诺的公开符号。
不得使用 `pub use qubit_reflect::*`、`pub use qubit_reflect::__private::*`，也不应仅为满足宏展开而重导出
runtime 的完整模块。

`qubit-model-metadata` 另外维护自己的 `__private::v2` 模型元数据 ABI。它将 reflect 协议精确别名为
`reflect_codegen_v1`，使模型 ABI 与反射 ABI 的所有权、版本号和迁移原因保持正交。

## 6. 动态安全边界

动态值有 local 与 thread-safe 两套模式。类型擦除仍受准确 `TypeId`、借用生命周期以及
`Send + Sync` bound 约束。字段访问、方法调用和构造遵循相同事务顺序：

1. 验证目标 descriptor、receiver、参数数量和精确类型；
2. 验证访问策略、可调用性和线程模式；
3. 所有检查成功后才进入生成 adapter；
4. adapter 前失败返回未消费的 owned 输入；
5. panic catching 只在显式声明且 unwind-safety bound 可证明时生成。

库不做数值转换、字符串解析或 `Into` 推导。反射边界描述并执行准确 Rust 语义，不建立第二套隐式类型系统。

## 7. 错误与诊断

可预期失败使用结构化错误：registry 冲突携带双方 fragment，类型不匹配携带 expected/actual，
调用与构造错误携带阶段和可恢复输入。宏输入错误保留原始 span，并尽量一次报告彼此独立的问题。

静态 descriptor 工厂接收生成器已验证的数据；用户可提供的数据一律通过受检构造器。内部 `expect`
仅可表达由同一生成器在更早阶段证明的事实，不能代替外部输入验证。

## 8. 验证矩阵

| 层次 | 主要验证 |
| --- | --- |
| runtime-only | package check/test/doctest，依赖树不含 derive 和外部类型族 |
| default | derive facade、普通 doctest、集成测试 |
| all-features | 生态/Qubit 类型、workspace tests、Clippy、Rustdoc |
| derive | parser/analysis 单元测试、trybuild pass/fail、invocation 集成 |
| registry | 跨 crate 聚合、冲突、冻结、稳定排序、并发初始化 |
| ABI/facade | 重命名依赖、显式 facade、`codegen_v1` 与模型 `v2` |
| robustness | coverage、有限 fuzz smoke、benchmark compile、Miri/sanitizer（环境允许时） |

覆盖率验证同时执行 crate 全局阈值和 `.rs-ci-critical-coverage.json` 中的高风险逐文件阈值；后者防止
关键路径被高覆盖率的简单模块掩盖。

仓库的 `.rs-ci-cargo-matrix.json` 是 feature 支持矩阵的机器可执行来源。
[需求追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md) 保持 284 个需求 ID 一一对应，
并验证其中引用的代码与测试路径存在。

## 9. 明确不做的事情

- 不引入模型、codec、validator 或 redact 语义。
- 不自动发现外部类型实现；feature 是依赖和 trait 实现边界。
- 不建立第三个 codegen crate。
- 不保留公开字段、旧平铺 `__private` 或 deprecated shim。
- 不为了缩短文件制造一次性 helper 或一类型一文件的机械碎片。
- 不承诺 `no_std`；当前运行时依赖 `std`。
