# `qubit-reflect` 详细设计

- 日期：2026-09-03
- 最近审核：2026-09-07
- 状态：破坏性边界重构已实现；当前只供内部仓库使用，发布流程明确暂缓
- 英文版：[English design](2026-09-03-qubit-reflect-design.md)
- 演进历史：[中文](2026-09-07-qubit-reflect-evolution.zh_CN.md) · [English](2026-09-07-qubit-reflect-evolution.md)
- 依据：[最终需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md)与[English requirements](2026-09-03-qubit-reflect-requirements.md)
- 适用仓库：`rs-reflect`
- 对应协议：`qubit-reflect 0.1` / `__private::codegen_v3`

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
│   └── codegen_v3/             # 唯一生成协议入口
├── registry/
│   ├── registry_builder.rs     # 冲突检查与冻结前聚合
│   ├── registry_snapshot_builder.rs      # 显式构建隔离 snapshot
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
聚合导航同时提供便捷入口与显式 snapshot 入口：`methods()` 会初始化进程全局 snapshot，
`methods_in`、`impls_in`、`methods_named_in` 则只查询调用方传入的不可变 registry。
因此，一个隔离 snapshot 的查询不会被无关的全局初始化缓存错误污染。

`RegistrySnapshotBuilder` 是为明确拥有 fragment 集合的调用方提供的公共构建入口。它不读取
linker inventory，从空集合开始收集带 `FragmentIdentity` 的类型化 payload，并在 `build()` 中
事务性校验完整集合。类型成员、定义成员和 capability payload 相互独立：只有 capability 的目标
可以被查询，但不会进入 snapshot 的根成员枚举。构建失败时返回完整的 `RegistryError`，不会发布
部分 snapshot。冻结后仍使用同一套不可变查询和索引路径，因此 `methods_in`、`impls_in`、有效类型视图
及 capability 查询都保持在当前 snapshot 内。

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
facade::__private::codegen_v3
```

协议按领域精确暴露 `access`、`capability`、`construct`、`descriptor`、`error`、
`expression`、`identity`、`invoke`、`registration`、`value`，以及生成 impl 所需的根反射类型。
这些是生成协议符号，不是供业务代码手写调用的 API；协议也不会重导出 runtime 的完整公开模块。
根 `__private` 不平铺这些符号。协议发生不兼容变化时新增兄弟版本，而不是静默扩大 v1。

下游 facade 为生成代码精确暴露 `__private::codegen_v3`，并独立逐项重导出它向业务代码承诺的公开符号。
不得使用 `pub use qubit_reflect::*`、`pub use qubit_reflect::__private::*`，也不应仅为满足宏展开而重导出
runtime 的完整模块。

`qubit-model-metadata` 另外维护自己的 `__private::v4` 模型元数据 ABI，并只在该精确私有模块中消费
反射协议，使模型 ABI 与反射 ABI 的所有权、版本号和迁移原因保持正交。

显式 snapshot builder 属于 runtime 公共 API，不会扩大任一生成代码协议。现有 derive 和 facade 继续使用
`__private::codegen_v3`，下游模型代码继续使用独立的 v4 ABI。因此，下游 fixture 或库可以构建确定性的
事实子集，而不必把协议迁移与 registry 所有权绑定在一起。

## 泛型定义与 effective capability

泛型声明是一等 `TypeDefinitionDescriptor`：定义字段只保存符号表达式，不包含运行时适配器；具体
`TypeDescriptor` 链回定义并保存已解析实参。两者都进入同一个冻结 `ReflectRegistry`。该注册表是唯一公开的
effective capability 解析入口，并支持无需分配 capability identity 的类型化与文本查询。


### 可失败查询与结构保真

冻结注册表只验证已链接目标；未注册的具体泛型实例仍可按需查询 intrinsic capability。
`capabilities`、`capability`、`capability_by_id` 返回 `Result`，所有冲突对象完整保留，
注册阶段通过 `RegistryError::intrinsic_conflict` 暴露相同原因。冻结枚举直接查询 index，
不执行 provider。缓存继续按具体 TypeId 保存成功或冲突，factory 在缓存表锁外执行；
provider 必须与 snapshot 无关，不得重入注册表初始化，其 panic 不在本契约捕获范围。

这一 provider 边界同样适用于 `RegistrySnapshotBuilder` 的调用方：provider 只能依赖静态类型事实，不能
查询 snapshot、可变外部状态或 registry 初始化；provider panic 不会被转换成能力缺失。详细 capability
失败会通过 `RegistryError` 保留冲突类别、ID、adapter `TypeId`、target 和来源 fragment；下游包装时，
原始冲突也会作为错误 source 暴露。

`ImplDefinitionDescriptor::implemented_trait()` 只暴露声明已知的链接；符号链接通过
`implemented_trait_in(&registry)` 或 `ReflectRegistry::impl_definition_trait` 解析，解析结果属于
snapshot 索引，构建失败和其他 snapshot 不会修改共享声明。facade 宏使用
`#[reflect(definition_provider_v2 = identifier)]` 选择 facade 所有的 provider 名称；v2 provider 是
无参数函数，返回 `&'static TypeDefinitionDescriptor`，不要求具体单态化，也不推断生成名称。

derive IR 的 `FieldShapeIr` 记录 Unit/Named/Unnamed，具体描述符、泛型定义和构造展开共用它。
空字段数量不再决定结构形状。此内部修订不改变 `codegen_v3` 或模型 v4 协议。
下游元数据和属性查询传播结构化错误，解析器附加根模型、完整路径、来源；
一条基础失败不生成伪 MissingProperty/InvalidValueClosure，也不阻断独立错误收集。

## 6. 动态安全边界

动态值有 local 与 thread-safe 两套模式。类型擦除仍受准确 `TypeId`、借用生命周期以及
`Send + Sync` bound 约束。字段访问、方法调用和构造遵循相同事务顺序：

1. 验证目标 descriptor、receiver、参数数量和精确类型；
2. 验证访问策略、可调用性和线程模式；
3. 所有检查成功后才进入生成 adapter；
4. adapter 前失败返回未消费的 owned 输入；
5. panic catching 只在显式声明且 unwind-safety bound 可证明时生成。

库不做数值转换、字符串解析或 `Into` 推导。反射边界描述并执行准确 Rust 语义，不建立第二套隐式类型系统。
类型级 ThreadSafe 生成覆盖字段 adapter、struct/variant 构造、struct update、owned-to-borrow bridge 与方法调用，
共同形成一个 mode 契约。tuple 与可移植函数指针内建描述明确支持到 arity 32；arity 33 不提供 `Reflect` 实现。

所有普通、catching、thread-safe 和 pinned 调用入口都显式接受
`&ReflectRegistry`。调用方先用 `methods_named_in(registry, ...)` 查找，
再将同一个 registry 传给 `invoke_*(registry, invocation)`。函数指针采用
独立的 `for<'registry, 'call>` 生命周期；输出、future 与 recovery 不保留
registry 借用，但仍受输入生命周期、Local/ThreadSafe 和 Pin 约束。调用不会
初始化全局 registry，也不会回退到全局 capability。

可安全生成的特殊 receiver 总有静态 adapter。所选 snapshot 缺少、类型不匹配
或只声明 fact 的 receiver capability 时，返回 `ReceiverAdapterUnavailable`，
并按调用者顺序保留全部输入；adapter 拒绝与 intrinsic 冲突分别保留结构化原因。
静态不支持的签名仍无入口。`TypeDescriptor` 的 Debug 只输出结构事实，不查询
capability，也不执行 provider。

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
| ABI/facade | 重命名依赖、显式 facade、`codegen_v3` 与模型 `v4` |
| robustness | coverage、有限 fuzz smoke、benchmark compile、Miri/sanitizer（环境允许时） |

覆盖率验证同时执行 crate 全局阈值和 `.rs-ci-critical-coverage.json` 中的高风险逐文件阈值；后者防止
关键路径被高覆盖率的简单模块掩盖。

Markdown 验收以独立 package 和进程执行每个 `rust` 程序。`rust,no_run` 只编译库，
`rust,compile_fail` 必须编译失败，且二者都需要正文解释。未知标记、空块和未闭合围栏均失败。
临时 workspace 会在 `--locked` 构建前核对 `Cargo.lock`；每次运行限时 10 秒，失败时保留日志和锁文件。
覆盖率保留原六项门禁，并按 d929d96 基线向下取整新增四项函数/行/区域阈值：`set` 100/98/99、
`registry` 98/98/98、`registry_builder` 100/97/96、`snapshot_builder` 100/100/100。
有界 `registry_snapshot` fuzz 将输入限制为 4096 字节、32 个 fragment、16 个来源、8 个静态 ID
和 4 个 descriptor，并通过公开 API 验证排序、冲突原子性、capability-only 成员和 snapshot 独立性。

仓库的 `.rs-ci-cargo-matrix.json` 是 feature 支持矩阵的机器可执行来源。
[需求追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md) 保持 284 个需求 ID 一一对应，
并验证其中引用的代码与测试路径存在。

下游门禁校验真实 `rs-model-metadata` workspace（包含 `derive/`）和 `rs-platform`；缺少相邻仓库会报错。
baseline 通道使用清单记录的精确 SHA，head 通道使用各依赖仓库的 `main` 修订；两条通道都显式记录
feature 选择，私有仓库可使用 `DEPENDENCY_TOKEN`。

## 9. 明确不做的事情

- 不引入模型、codec、validator 或 redact 语义。
- 不自动发现外部类型实现；feature 是依赖和 trait 实现边界。
- 不建立第三个 codegen crate。
- 不保留公开字段、旧平铺 `__private` 或 deprecated shim。
- 不为了缩短文件制造一次性 helper 或一类型一文件的机械碎片。
- 不承诺 `no_std`；当前运行时依赖 `std`。
