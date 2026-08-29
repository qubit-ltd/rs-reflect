# `qubit-reflect` 详细设计

- 日期：2026-08-29
- 状态：待设计评审
- 依据：[最终需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md)
- 适用仓库：`rs-reflect`
- 目标版本：需求规范所定义的完整最终能力，不划分功能缩水的“首版”

## 1. 文档目的

本文把最终需求规范转换为可实现的 Rust 架构，确定 crate 边界、公共类型模型、宏展开协议、descriptor 生命周期、
动态值安全边界、调用和构造流程、静态汇聚、错误模型以及测试策略。本文不重新解释或削弱需求；实现细节与需求冲突
时，以需求规范为准并先修订设计，而不是在代码中隐式选择。

`qubit-reflect` 是业务无关的独立反射库。它为后续 `qubit-model-derive`、`qubit-model-metadata` 以及其他框架提供
结构与安全动态操作能力，但不得依赖、识别或特判任何 `rs-model-*` 概念。

## 2. 设计原则

1. **单一结构事实源**：一个实现 `Reflect` 的 concrete Rust 类型只有一个根 `TypeDescriptor`。
2. **描述与执行分离**：所有合法声明尽可能完整描述；只有存在安全 adapter 的操作才可执行。
3. **编译期证明优先**：宏输入内可验证的 target、bound、签名和属性问题全部在编译期失败。
4. **运行时严格受检**：动态输入先完整验证，再进入字段修改、方法调用或值构造。
5. **无业务语义**：text、decimal、identifier、reference、validation、codec、redact 等均由上层 metadata 解释。
6. **不可变查询模型**：公开 descriptor 和完成初始化的 registry 只读、可并发共享、顺序确定。
7. **安全 Rust 边界**：本仓库运行时代码和生成代码保持 `#![forbid(unsafe_code)]`；平台相关 unsafe 只能存在于经过
   验证的第三方安全封装中。
8. **显式能力**：opaque、capability、catching、线程安全调用和 concrete specialization 均由源码显式声明，不做
   依赖查询顺序或不稳定 trait 探测的自动推断。
9. **最小依赖方向**：`rs-model-* -> qubit-reflect`，绝不允许反向依赖或循环依赖。

全仓库使用 Rust 2024 edition，最低支持 Rust 版本严格采用根 `Cargo.toml` 的 `rust-version = "1.94"`，依赖 `std`。
`no_std`/`alloc` 不是兼容目标。

## 3. 仓库与 crate 架构

### 3.1 单仓库、双 crate

Rust 要求过程宏由 `proc-macro` crate 提供，而运行时类型必须位于普通 library crate。两者不能合并为同一个 Cargo
package，因此采用一个仓库中的两个 crate：

```text
rs-reflect/
├── Cargo.toml                         # qubit-reflect package + workspace
├── src/                               # qubit-reflect runtime
├── derive/
│   ├── Cargo.toml                     # qubit-reflect-derive proc-macro
│   └── src/
├── tests/
├── test-crates/                       # 跨 crate 汇聚测试夹具
└── doc/
```

根 `Cargo.toml` 同时声明 `[package]` 和 `[workspace]`，workspace 成员包含根 package 与 `derive`。根 crate 提供默认开启的
`derive` feature：

```toml
[features]
default = ["derive"]
derive = ["dep:qubit-reflect-derive"]
```

启用该 feature 时，`qubit-reflect` 重导出 `Reflect` derive、`reflect` 和 `reflect_impl`。普通用户只声明并导入
`qubit-reflect`；只使用 runtime 的用户可关闭默认 feature。

### 3.2 依赖方向

```text
qubit-reflect ──optional──> qubit-reflect-derive
      │                              │
      │                              └─ 只生成引用 facade 隐藏协议的 token
      │                                 不依赖 qubit-reflect，避免 Cargo 循环
      └─ inventory / std

qubit-model-metadata ──> qubit-reflect
qubit-model-derive   ──> qubit-reflect 的公共宏契约
```

`qubit-reflect-derive` 不直接依赖 `qubit-reflect`。它使用 `proc-macro-crate` 解析 facade 在调用者依赖图中的实际名称，
生成对 `qubit-reflect::__private` 的路径。runtime 与 derive 之间的隐藏协议由跨 crate compile-pass 测试锁定。

不建立第三个 codegen crate。未来模型角色 attribute macro 应在展开后的类型上委托 facade 重导出的 `Reflect` derive，
而不是调用或复制 `qubit-reflect-derive` 的内部实现。

### 3.3 为什么不依赖 `qubit-spi`

`qubit-spi` 是运行时可变的服务提供者 catalog，负责 selector、alias、priority、fallback 和实例创建；反射 registry 是
链接期发现、一次性校验并冻结的 descriptor 快照。二者状态模型和查找语义不同。`qubit-reflect` 不依赖
`qubit-spi`，但采用其以下通用经验：

- 在修改共享状态前取得并验证完整 metadata；
- 冲突失败不产生部分状态；
- descriptor 与索引分离；
- 不在锁内执行用户代码；
- 使用结构化错误表达冲突双方。

### 3.4 依赖预算

runtime 的直接依赖限定为：

- `inventory`：隐藏的静态 fragment 发现；
- `thiserror`：结构化错误实现。

derive 的直接依赖限定为：

- `proc-macro2`、`quote`、`syn`：过程宏解析与 token 生成；
- `proc-macro-crate`：解析 facade 被重命名后的实际路径。

测试可使用 `trybuild` 做 UI 编译测试。除非实施中出现已由成熟通用 crate 完整解决且经过评审的新需求，不增加
`qubit-spi`、`qubit-collections`、异步 runtime、序列化框架或模型层依赖。动态 future 只依赖 `std::future::Future`。

## 4. runtime 模块划分

```text
src/
├── lib.rs
├── descriptor/
│   ├── mod.rs
│   ├── identity.rs
│   ├── type_descriptor.rs
│   ├── type_kind.rs
│   ├── type_ref.rs
│   ├── field_descriptor.rs
│   ├── variant_descriptor.rs
│   ├── method_descriptor.rs
│   ├── trait_descriptor.rs
│   ├── impl_descriptor.rs
│   ├── generic_descriptor.rs
│   ├── type_expression.rs
│   └── capability.rs
├── value/
│   ├── mod.rs
│   ├── mode.rs
│   ├── dynamic_ref.rs
│   ├── dynamic_mut.rs
│   └── dynamic_owned.rs
├── access/
├── invoke/
│   ├── argument.rs
│   ├── receiver.rs
│   ├── output.rs
│   ├── future.rs
│   └── recovery.rs
├── construct/
│   ├── struct_constructor.rs
│   ├── variant_constructor.rs
│   └── update.rs
├── registry/
│   ├── fragment.rs
│   ├── builder.rs
│   ├── registry.rs
│   ├── indexes.rs
│   └── interner.rs
├── builtin/
├── error/
└── private/
```

每个模块只公开需求规范要求的只读查询或安全操作。`private` 通过 `#[doc(hidden)]` 提供宏生成代码所需的工厂、静态登记
和编译期断言入口；它不是普通用户扩展公共 API 的入口，但发布版本必须在语义版本控制中维护其与同版本 derive crate
的兼容性。

## 5. descriptor 核心模型

### 5.1 根对象与导航

```text
TypeDescriptor
  ├─ TypeIdentity / TypeKind / TypeCapabilities
  ├─ FieldDescriptor[] ──> TypeRef
  ├─ VariantDescriptor[] ──> FieldDescriptor[] ──> TypeRef
  ├─ GenericDefinitionDescriptor + GenericArgument[]
  └─ ImplDescriptor[]
       ├─ inherent ──> MethodDescriptor[]
       └─ trait ──> TraitDescriptor ──> MethodDescriptor[]
```

根 `TypeDescriptor` 不平铺所有 kind 的专用方法。`kind()` 返回稳定分类，`as_struct()`、`as_sequence()`、`as_map()`、
`as_function()` 等进入 typed view。错误 kind 返回 `None`，适用集合为空时返回空 slice。

### 5.2 身份与名称

- concrete 类型身份：当前进程 `TypeId`；
- reflected trait 身份：宏生成隐藏 marker 的 `TypeId`；
- external trait 身份：显式命名空间化 `ExternalTraitId`；
- member 身份：所属 descriptor 身份、成员类别、声明序号和 impl fragment 身份的复合值；
- capability 身份：稳定命名空间化 `CapabilityId`。

每个具名 descriptor 分开保存：

- Rust name/path：源码事实和诊断使用，不受 rename 影响；
- query name：查找使用，默认等于 Rust name，可由 `#[reflect(rename = "...")]` 改变；
- `type_name`：来自 `std::any::type_name` 的诊断名，不作为持久化协议 ID。

名称索引一律返回候选集合或显式歧义结果。任何裸名称查询都不得静默选择链接顺序中的第一个成员。

可见性归一化为 public、crate、super、restricted path 和 private；`pub(self)` 归入 private，同时保留原 restricted
path 作诊断。trait item 和 enum variant 字段记录其可达性来自所属声明，不伪造 item 自身的 `pub`。

### 5.3 `TypeRef` 与显式 opaque

字段类型采用：

```rust,ignore
pub enum TypeRef {
    Resolved(&'static TypeDescriptor),
    Opaque(&'static OpaqueTypeDescriptor),
    Symbolic(TypeExpression),
}
```

- 普通 concrete 字段要求字段类型实现 `Reflect`，使用 `TypeRef::Resolved`；
- `#[reflect(opaque)]` 字段只要求 concrete 类型为 `'static`，使用 `TypeRef::Opaque`；
- generic definition 中尚未单态化的字段使用 `TypeRef::Symbolic`；
- opaque member 即使其 concrete 类型在其他位置实现 `Reflect`，也不得自动升级；
- `OpaqueTypeDescriptor` 只保存准确 `TypeId`、诊断名和整体操作 adapter，不是第二个根 descriptor；
- 类型级 `#[reflect(opaque)]` 仍有唯一根 descriptor，但 kind 为 `Opaque`，不公开内部字段或 variant。

### 5.4 类型表达式

`TypeExpression` 和 `LifetimeExpression` 是 runtime 自有的不可变结构化树，不公开 `syn` 类型。表达范围包括：

- concrete path、类型参数、`Self`、关联类型；
- shared/mutable reference、raw pointer、slice、array、tuple；
- function pointer 的 ABI、safety、variadic、参数和返回；
- dyn trait、opaque `impl Trait`、never；
- generic 类型/const/lifetime 实参、bounds、HRTB 和 where predicates。

源码 token 文本只作为诊断补充，不能充当身份或导航模型。

## 6. descriptor 生命周期与递归

### 6.1 统一 concrete interner

所有 `TypeDescriptor::of::<T>()` 通过私有 `DescriptorInterner` 按 `TypeId` 唯一化，包括 derive 类型、builtin、泛型
单态化和组合类型。实现使用短临界区的标准库同步原语：

```text
OnceLock<Mutex<HashMap<TypeId, &'static OnceLock<TypeDescriptor>>>>
```

查询流程：

1. 在 map 锁内查找或插入当前 `TypeId` 对应的、进程生命周期保留的 descriptor cell；
2. 释放 map 锁；
3. 对 cell 执行 `get_or_init`；
4. 返回唯一 `&'static TypeDescriptor`。

descriptor builder 是由宏或 builtin 提供的编译期验证后工厂，因此结构构造为不可失败操作。若 builder panic，cell 保持
未初始化，panic 原样传播；框架不把 panic 伪装成 descriptor 错误。interner 不在执行 builder 时持有全局 map 锁。
短临界区内不调用用户代码；若此前 panic 导致标准库 mutex poisoning，使用 `PoisonError::into_inner` 取得仍满足容器
内存安全保证的 map，再执行正常查找/插入，避免把一次无关 panic 永久升级为所有类型查询失败。

### 6.2 惰性关系句柄

构造一个 descriptor 时不递归解析其字段、关联类型、supertrait 或 generic argument 的目标 descriptor，而存储内部
`TypeHandle`：

```text
FieldDescriptor
  └─ LazyTypeRef
       ├─ source policy: resolved / opaque / symbolic
       ├─ resolver function pointer
       └─ OnceLock<TypeRef>
```

首次导航关系时解析并缓存 `TypeRef`。因此 `Node -> Option<Box<Node>>` 和 `A -> B -> A` 不会在 descriptor 初始化时
递归等待。公开根 descriptor 在返回前已经包含完整成员集合；惰性的是关系目标，不是成员列表本身，其他线程不会看到
半初始化 shape。

### 6.3 定义与 concrete 实例

泛型声明拥有独立 `GenericDefinitionDescriptor`；每个 `'static` concrete 实例拥有自己的根 `TypeDescriptor`，并导航
回同一个定义 descriptor。带 lifetime 参数的类型只为满足 `Self: 'static` 的 concrete 实例实现 `Reflect`。定义级
lifetime 信息完整保留，但不为非 `'static` 根值伪造 `TypeId`。

interner 是内部缓存，不属于公开 registry。按需查询新的 concrete 类型不得改变 registry 枚举与名称索引。

## 7. builtin 类型体系

`builtin` 为需求指定的类型实现 `Reflect`，并通过相同 interner 返回 descriptor：

- primitive：`bool`、`char`、所有固定宽度整数、`isize`、`usize`、`f32`、`f64`；
- text：`String`、`str`；
- tuple：`()` 以及需求支持范围内的 tuple arity；
- array、slice、reference、raw pointer、function pointer；
- `Option<T>`、`Vec<T>`；
- `HashMap`、`BTreeMap`、`HashSet`、`BTreeSet`；
- `Box<T>`、`Rc<T>`、`Arc<T>`；
- 合法 dyn-compatible `dyn Trait` 的生成 descriptor。

typed view 保存完整 family 和类型参数。builtin descriptor 的存在只代表可以描述；除明确 capability adapter 外，不承诺
容器通用动态构造。闭包和不可命名 function item 只在声明级表达式中作为 opaque，不自动实现根反射。

在稳定 Rust 没有 variadic generics 的前提下，根 `Reflect` 实现的精确边界为：

- tuple arity `0..=32`；更高 arity 仍可在 `TypeExpression` 中描述，但没有 builtin 根 `Reflect` impl；
- Rust ABI 以及可移植的 `C`、`C-unwind`、`system`、`system-unwind` function pointer，参数 arity `0..=32`，分别
  覆盖 safe/unsafe；合法 C-variadic 形态由目标工具链支持时按 cfg 提供；
- 其他 target-specific ABI 始终可在声明级 `TypeExpression` 中保留，只有对应 target 的 builtin 模块显式实现后才有
  根 descriptor，框架不得把它们归并成 `C` ABI。

上述 arity 由内部声明宏机械生成实现和测试，不手写重复代码。

## 8. capability 系统

### 8.1 类型化 capability key

capability 不使用全局动态 bit 位。公共扩展采用稳定 ID 与 adapter Rust 类型共同定义的类型化 key：

```rust,ignore
pub struct CapabilityKey<A: 'static> {
    id: CapabilityId,
    adapter_type: TypeId,
    _marker: PhantomData<fn() -> A>,
}
```

`TypeCapabilities::get(key)` 返回 `Option<&A>`。同一 `CapabilityId` 若绑定不同 adapter 类型或不兼容契约，在宏输入内
编译失败，跨 fragment 冲突在 registry 初始化时失败。

公开迭代返回稳定排序的 `CapabilityDescriptor`，至少包含 ID、adapter contract identity 和是否存在操作 adapter；因此
不知道具体 adapter Rust 类型的上层仍可保留、比较和转发扩展 capability，只有执行类型化操作时才需要对应 key。

`CapabilityId` 和 `ExternalTraitId` 都使用点号分隔的非空标识符段；`qubit.reflect.*` 只保留给本库。ID 构造、宏字面量
和静态登记入口共享同一验证器，禁止空段、首尾点号和第三方占用保留命名空间。

### 8.2 内建能力

- `qubit.reflect.send`、`qubit.reflect.sync`：事实能力，不允许据此绕过调用点泛型 bound；
- `qubit.reflect.clone`：携带动态 clone adapter；
- `qubit.reflect.default`：携带动态 default adapter；
- `ConstructFromElements`、`ReceiverAdapter`、`UnsizedValueAdapter` 等以同一扩展机制提供，不作为所有类型的默认能力。

用户类型通过 `#[reflect(capabilities(...))]` 显式声明。生成代码用普通 Rust trait bound 验证声明真实性。capability 缺失
表示未登记，不表示类型在 Rust 中必然没有实现相应 trait。

对于无法在稳定 Rust 上自动探测条件 trait 的 generic/builtin concrete 类型，提供业务无关的显式静态登记宏：

```rust,ignore
qubit_reflect::register_reflected_type!(Page<User>);
qubit_reflect::register_type_capabilities!(
    Vec<User>: Clone, Send, Sync
);
```

`register_reflected_type!` 使已实现 `Reflect` 的 concrete 类型进入公开 registry；它不影响 interner 中的根唯一性。
`register_type_capabilities!` 为准确 concrete `TypeId` 生成 capability fragment，并用 Rust bound 编译期验证。builtin 只
自动登记对其全部合法类型参数都成立的能力；条件能力缺失保持 `NotRegistered`，不得使用 specialization 猜测。

## 9. 过程宏设计

### 9.1 derive crate 内部流水线

```text
TokenStream
  -> parse::Input
  -> ir::Declaration
  -> validate::ValidatedDeclaration
  -> expand::{descriptor, adapters, registration}
  -> TokenStream
```

`parse` 是唯一直接操作 `syn` AST 的层；`ir` 只保存生成所需的语义事实；`validate` 汇总能够同时报告的诊断；`expand`
不得重新解释属性。三种宏共享属性解析、类型表达式转换、身份生成和 adapter 生成组件。

### 9.2 `#[derive(Reflect)]`

derive 生成：

- `Reflect` impl；
- descriptor builder；
- 私有字段和 variant 的安全访问/构造 adapter；
- generic definition 与最小必要 bound；
- capability 编译期断言及 adapter；
- 一个不可变 `RegistrationFragment`。

普通字段递归生成形成 resolved descriptor 所需的 `Reflect` bound；只出现在 opaque 字段中的泛型参数只增加 `'static`
bound。类型级 opaque 不生成字段、variant 或逐字段构造 adapter。

### 9.3 `#[reflect]` trait

trait attribute macro生成：

- 完整 `TraitDefinitionDescriptor` 和注册片段；
- 隐藏 marker 身份；
- 一个带 `where Self: Sized + 'static` 的隐藏默认关联 hook。

该 hook 由具体 impl 的宏调用，用于单态化 trait 默认方法、关联项和 applied trait descriptor。它具有默认实现，不增加
实现者负担，并以 `Self: Sized` 保持原 trait 的 dyn compatibility。生成名称包含宏保留前缀和声明指纹，属性宏在
冲突时给出编译错误。

外部 supertrait/bound 必须在同一个 `#[reflect(...)]` 参数中使用
`external_trait(path, id = "...")` 显式映射；宏不猜测外部 trait 身份。

### 9.4 `#[reflect_impl]`

impl attribute macro生成：

- inherent 或 trait impl fragment；
- 每个受支持 concrete 方法的调用 adapter；
- 泛型定义 descriptor；
- 显式 `specialize(...)` 的 concrete instance fragment。

反射 trait impl 未提供 `external_trait_id` 时，生成代码通过 trait 的隐藏关联 hook取得 marker identity 和完整默认项；
如果目标 trait 未使用 `#[reflect]`，正常 Rust 类型检查会给出缺少隐藏 hook 的编译错误。外部 trait impl 必须显式提供
`external_trait_id`，且只描述当前 impl 输入中可证明的事实。

trait 默认方法由 trait hook 为 concrete `Self` 生成 adapter；impl fragment 中显式出现的方法覆盖相同声明身份并标记
`Overridden`，其他默认方法标记 `Defaulted`。这样无需过程宏跨文件读取 trait 源码。

### 9.5 helper 属性

属性合法目标由一张共享静态表驱动，避免 parse 与 validate 各维护一份规则。核心属性包括：

- 类型：`rename`、`opaque`、`capabilities(...)`；
- 字段：`rename`、`skip`、`opaque`、`read_only`、`no_construct`、`default`、`default = path`；
- variant：`rename`、`skip`、`no_construct`；
- 方法：`rename`、`skip`、`no_invoke`、`catch_unwind`、`thread_safe`、`specialize(...)`；
- impl：`external_trait_id`、`specialize(...)`。

未知键、错误目标、重复键和互斥组合一次宏展开中尽可能合并报告。

`skip` 不删除结构事实，也不改变字段、variant 或方法的源码索引；它只关闭对应动态 adapter 并在 descriptor 中记录
策略。任意非 `reflect` attribute 均被忽略且不进入 runtime descriptor。

### 9.6 生成路径与 facade

derive crate 使用 `proc-macro-crate` 查找调用者依赖中的 facade 名称。所有生成代码只引用 facade 的公共或
`#[doc(hidden)]` 重导出，不要求调用者直接依赖 `inventory` 或 `qubit-reflect-derive`。

`qubit-model-derive` 后续的角色 attribute macro 应输出由 `qubit-model-metadata` facade 隐藏重导出的
`Reflect` derive。领域属性由模型宏消费，`Reflect` 只读取 `#[reflect(...)]`，不会把其他 attribute 泄漏到 descriptor。

## 10. 静态汇聚与公开 registry

### 10.1 fragment 发现

runtime 使用 `inventory` 收集统一的 `RegistrationFragment`。宏生成代码通过
`qubit_reflect::__private::inventory::submit!` 提交，因此下游 crate 无需声明 `inventory` 依赖。

fragment 只包含静态数据和函数指针，不在链接器构造阶段执行用户代码：

```rust,ignore
struct RegistrationFragment {
    kind: FragmentKind,
    identity: FragmentIdentity,
    target_identity: fn() -> RuntimeIdentity,
    build: fn() -> FragmentPayload,
}
```

`FragmentIdentity` 包含声明 crate、`module_path!()`、行列位置、成员类别和宏根据规范化输入计算的内容指纹。`file!()`
仅保留作诊断，不作为跨机器排序的首要键。

### 10.2 初始化流水线

```text
inventory fragments
  -> 复制静态 fragment 引用
  -> 按稳定 identity 排序
  -> 构建 payload
  -> 全量身份与契约校验
  -> 合并 type / trait / impl / specialization
  -> 建立全部索引
  -> 冻结 ReflectRegistry
  -> OnceLock<Result<ReflectRegistry, RegistryError>>
```

builder 使用私有临时集合；只有全部校验成功才构造公开 registry。失败结果被 `OnceLock` 缓存，后续查询得到等价错误。
纯 `TypeDescriptor::of::<T>()`、字段和 variant shape 查询不依赖 registry 初始化成功；`impls()`、`methods()` 等聚合查询
传播 `RegistryError`。

### 10.3 registry 索引

冻结快照至少包含：

- `TypeId -> TypeDescriptor`；
- `type_name -> [TypeDescriptor]`；
- `query_name -> [TypeDescriptor]`；
- `TraitId -> TraitDescriptor`；
- target `TypeId -> [ImplDescriptor]`；
- fragment identity 和 capability ID 的冲突审计信息。

内部查找索引可使用 `HashMap`，公开迭代使用预排序的 boxed slice。任何 hash iteration 顺序都不得泄漏为公共顺序。

### 10.4 平台策略

保证平台与仓库 CI 一致：Linux、macOS 和 Windows。每个平台都运行跨依赖 crate 的 fragment 完整性、重复检测和并发
首次初始化测试。不支持 `inventory` 静态提交的平台在编译期给出明确诊断，不提供貌似成功但缺项的 registry。

## 11. 动态值安全边界

### 11.1 mode 泛型

公共包装共享一套 mode 泛型底层类型：

```rust,ignore
DynamicRef<'a, Local>
DynamicMut<'a, Local>
DynamicOwned<Local>

DynamicRef<'a, ThreadSafe>
DynamicMut<'a, ThreadSafe>
DynamicOwned<ThreadSafe>
```

公共别名分别为 `Reflected*` 和 `SendReflected*`。`Mode` 是 sealed trait，内部关联到不同的擦除边界：

- Local owned：`Box<dyn Any>`；
- ThreadSafe owned：`Box<dyn Any + Send + Sync>`；
- Local borrow：不增加 auto trait；
- ThreadSafe shared borrow：构造时要求 `T: Sync`；
- ThreadSafe mutable borrow：构造时要求 `T: Send + Sync`。

不存在第二个公共 `ReflectValue` trait。动态包装可以承载未实现 `Reflect` 但满足准确类型和生命周期要求的 opaque 值。

### 11.2 downcast 与 Any

- `is::<T>()`、`downcast_ref::<T>()`、`downcast_mut::<T>()` 使用准确 `TypeId`；
- owned `downcast::<T>(self)` 失败返回原包装；
- `as_any`/`as_any_mut`/`into_any` 保持 mode 的 auto-trait 边界；
- ThreadSafe 包装提供消费自身的 `into_local`，该降级不可失败；
- 不提供从 Local 到 ThreadSafe 的运行时升级。

### 11.3 unsized 类型

稳定安全 Rust 无法把任意 `T: ?Sized` 统一擦除到 `Any`。因此：

- `str` 使用内建专用 enum variant，提供 `new_str`、`new_str_mut`、`as_str`、`as_str_mut`；
- slice 和 dyn trait 默认只能描述；
- 只有某 kind 显式登记安全 `UnsizedValueAdapter` 后才能进入对应专用动态路径；
- 不使用裸指针伪造 unsized dynamic value。

## 12. 字段与 variant 操作

### 12.1 字段 adapter

derive 在类型定义位置生成普通安全函数，因此可以访问私有字段。`FieldDescriptor` 保存共享读取、可变读取和 set 的可选
函数指针。操作流程为：

1. 检查 target `TypeId` 等于 declaring type；
2. 检查策略：`skip`、`read_only`；
3. set 时检查 value `TypeId`；
4. 全部成功后调用 adapter；
5. 返回受 target 生命周期约束的动态借用。

同步字段 API 使用 Local mode。ThreadSafe 包装先通过 `into_local` 无损降级；字段访问本身在当前线程同步执行，不根据
runtime capability 动态增加 auto trait。

### 12.2 enum variant

variant 保存源码顺序、shape、字段和 active-test adapter。字段 adapter 首先验证根 enum 类型和当前 active variant，
错误 variant 返回结构化错误。

数值 discriminant 仅为 fieldless integer-`repr` enum 生成。宏从声明顺序、显式值和 Rust 隐式递增规则生成常量
adapter；data-carrying 或无整数 `repr` enum 不提供数值接口。

## 13. 方法、trait 与 impl

### 13.1 方法声明与实例

`MethodDescriptor` 描述声明签名，`MethodInstanceDescriptor` 描述可调用 concrete specialization。二者分离，避免把
无限泛型声明误当作运行时可调用实例。

参数保存：声明索引、可选 identifier 名称、pattern 类别、`TypeExpression`、可选 concrete descriptor 和 passing
mode。返回值使用 `ReturnDescriptor` 区分 unit、never、concrete、reference 和 opaque `impl Trait`。

descriptor 完整记录 `async`、`unsafe`、`const`、`extern ABI`、variadic 和 generic 事实。`const fn` 的 adapter 只按
普通运行时调用，不承诺常量求值；safe extern 方法在 ABI 和签名可由安全 Rust 调用时使用普通 adapter；unsafe、
variadic 或目标 ABI 无安全 adapter 的声明只描述并返回稳定不可调用原因。

### 13.2 trait 完整度

- reflected trait：完整 supertrait、方法、关联类型、关联常量和默认来源；
- external trait：由 `ExternalTraitId` 标识的 `ExternalIncomplete`，只包含当前 impl 可证明的事实；
- dyn-compatible trait 另有 `dyn Trait` 类型 descriptor；
- 非 dyn-compatible trait 仍有独立 `TraitDescriptor`，不伪造 dyn 类型。

generic trait 分为 definition 与 applied descriptor。applied identity 包含 trait marker/external ID 和全部 concrete
类型/const 实参；supertrait 与关联项在 applied view 中完成代换。

supertrait 分别保存源码顺序的 direct view 和确定、去重、防递归的 transitive closure。external trait 路径只作诊断
alias，`ExternalTraitId` 才是汇聚身份；同一 ID 的不同 `use` 路径允许合并，同一目标上的重复 impl 或不可合并事实产生
确定性 `RegistryError`。

关联类型绑定始终先保存结构化 `TypeExpression`。只有 trait 声明或显式 specialization 已提供足够
`Reflect + 'static` bound 时才生成 resolved handle；宏不得通过 specialization 探测任意 concrete 关联类型。关联
常量同样始终描述声明，只有其类型可安全进入 owned 动态边界时才生成读取 adapter，并记录值来自默认声明还是 impl
覆盖。

### 13.3 有效方法视图

registry 合并 trait 声明 hook 和 impl fragment：

1. 从 trait hook取得 concrete `Self` 的默认方法与关联项；
2. 用 impl 中显式声明的相同 item identity 覆盖；
3. 标记 `Defaulted` 或 `Overridden`；
4. 保留 inherent 与各 trait 命名空间；
5. 按限定查询解决同名歧义。

## 14. 动态调用

### 14.1 输入模型

一次调用拥有显式 receiver 和有序 `InvocationArg<'call, Mode>`：

- receiver：无 receiver、owned、shared、mutable 以及受支持的智能指针/Pin receiver；
- 参数：Owned、Ref、Mut；
- 只允许 mutable borrow 安全重借为 shared borrow；
- 不把 owned 参数隐式借出、clone 或执行 `Into`；
- 名称绑定只接受名称唯一的 identifier 参数，位置绑定是规范入口。

所有借用重借到共同 `'call` 生命周期。借用输出记录 `BorrowOrigin::Receiver` 或参数索引集合。
HRTB 只有在生成代码能把关系安全实例化到该共同 `'call` 时才有 adapter，否则完整保留声明并报告不可调用。

### 14.2 预执行验证与 recovery

adapter 调用前统一验证：方法能力、receiver 形态、数量、passing mode 和全部准确类型。验证阶段不消费底层 owned 值。
任一失败返回 `InvocationRecovery`，其中按原顺序返还 owned receiver 和参数；借用 wrapper 保持原生命周期。

只有校验全部成功后才从 wrapper 中提取 owned 值并进入用户代码。进入用户代码后的消费与 panic 行为遵循原 Rust
签名，不再承诺恢复。

### 14.3 输出

`InvocationOutput<'call, Mode>` 区分：

- Unit；
- Owned；
- Ref + borrow origins；
- Mut + borrow origin；
- Future。

never 方法若正常返回属于生成 adapter 的内部不变量错误。非 async opaque `impl Trait` 默认只描述，调用能力返回稳定
`OpaqueReturnType` 原因；类型作者若需要动态调用，应使用可命名返回类型。

### 14.4 panic、async 与线程安全

- 普通 `invoke` 原样传播用户 panic；
- 只有 `#[reflect(catch_unwind)]` 生成 catching adapter，并以编译期 bound 验证 unwind safety；
- `panic = "abort"` 时 descriptor 报告 catching 不可用；
- async adapter 返回保留 `'call` 的 boxed `ReflectedFuture`，不选择或运行 executor；
- 方法默认只有 Local adapter；
- `#[reflect(thread_safe)]` 显式生成 ThreadSafe adapter，并编译期验证 receiver、参数、输出和 future 的
  `Send`/`Sync` 约束。

### 14.5 receiver 扩展

核心支持普通 receiver、`Box<Self>`、`Rc<Self>`、`Arc<Self>`、`Pin<&Self>`、`Pin<&mut Self>` 和
`Pin<Box<Self>>`。其他 arbitrary self type 必须通过类型化 `ReceiverAdapter` capability 提供安全转换；没有 adapter
时保留完整签名并报告不可调用。`unsafe fn` 永远只描述。

## 15. 动态构造

### 15.1 两阶段构造

struct 和 variant 构造统一采用：

```text
输入收集
  -> 名称/索引、重复、缺失、策略、TypeId 全量校验
  -> ValidatedConstructionInput
  -> 宏生成的安全 Rust 字面量构造 adapter
  -> ReflectedOwned
```

宏生成 adapter 通过正常 struct/enum 字面量构造私有字段，不使用未初始化内存。验证失败时 recovery payload 返还所有
owned 输入。

### 15.2 缺省策略

- 默认每个可构造字段恰好出现一次；
- `Option<T>` 不自动补 `None`；
- `#[reflect(default)]` 编译期验证字段类型 `Default`；
- `#[reflect(default = path)]` 编译期验证零参数 provider 的准确返回类型；
- skipped/no_construct 字段缺少显式 provider 时，整个从零构造能力不可用；
- 根类型 `Default` capability 与逐字段缺省互不影响。

### 15.3 update

update 消费一个准确根类型的 owned 基值和零到多个 override。先完整验证并 downcast 全部 override，再由生成 adapter
对已 downcast 的基值执行安全字段赋值并返回同一 owned 根值；该路径也适用于实现 `Drop` 的 struct，不尝试移动其
字段。预验证失败返还基值与全部 override；开始字段赋值后，旧字段值的用户 `Drop` panic 按普通用户代码 panic 传播，
不再承诺恢复。opaque 字段只允许准确类型整体替换。

核心不为 container kind 自动生成构造器；上层可通过类型化 `ConstructFromElements` capability 扩展。

## 16. 错误模型

错误按阶段分层，全部实现 `Debug`、`Display` 和 `Error`：

```text
Macro diagnostics                    编译期
DescriptorBuild invariant panic      框架 bug，非普通错误
RegistryError                        静态 fragment 汇聚失败
FieldAccessError                     字段 target/value/策略错误
InvocationError + Recovery           进入用户代码前失败
InvocationPanic                      显式 catching 捕获的用户 panic
ConstructionError + Recovery         构造/update 验证失败
CapabilityError                      ID/contract/adapter 不匹配
```

错误分类是机器可匹配的公开 enum；显示文本不是稳定协议。成员不可执行错误保存全部阻塞原因，而不是只报告首个原因。
registry 错误保存冲突双方 fragment identity；动态类型错误尽量同时保存期望和实际 `TypeId`/诊断名。
为满足缓存结果按值返回的公共签名，`RegistryError` 使用内部 `Arc<RegistryErrorData>` 保存不可变详情并实现低成本
`Clone`；每次初始化查询返回同一错误事实的 clone，而不是重新执行汇聚。

## 17. 与模型层的集成契约

`qubit-reflect` 只提供：

- 类型、字段、variant、方法、trait 和 impl 的结构；
- 安全读写、调用、构造机制；
- 业务无关 capability 扩展点。

模型层负责：

- `TypeMetadata` 和角色；
- text/decimal/sequence 等约束；
- identifier、unique、reference；
- validation、codec、redact；
- Property getter/setter 推导；
- model-aware random 和上下文 fixture。

`FieldMetadata` 关联同一个 `FieldDescriptor`，`TypeMetadata` 关联同一个根 `TypeDescriptor`。模型角色宏通过 facade
委托 `Reflect` derive，不在 `qubit-reflect` 增加模型专用 helper、descriptor 字段或注册分支。

结构随机生成器可以只依赖本库，但只能保证 Rust shape 与类型正确。任何静态领域约束必须由上层读取
`TypeMetadata`；关系、唯一性和外部 validator 还需要业务上下文。

## 18. 测试架构

### 18.1 测试层次

1. **模块单元测试**：身份、名称、类型表达式、kind、capability、索引和错误格式。
2. **runtime 集成测试**：动态值、字段、variant、调用、构造、recovery 和 interner。
3. **compile-pass/compile-fail**：使用 `trybuild` 覆盖宏 target、helper 属性、bound、生命周期和线程安全。
4. **跨 crate 夹具**：至少两个依赖 crate 加最终应用，验证 `inventory` 汇聚、alias 路径和链接顺序无关性。
5. **并发测试**：interner 与 registry 并发首次查询、递归关系和缓存错误。
6. **平台 CI**：Linux、macOS、Windows 都运行静态登记完整性测试。
7. **模型边界测试**：只验证通用 facade 委托协议和 descriptor 身份，不把 `rs-model-*` 加为本仓库依赖；完整模型集成
   测试归属 `rs-model-*` 仓库。

### 18.2 测试夹具结构

```text
tests/
├── integration_tests.rs
├── descriptor/
├── value/
├── access/
├── invoke/
├── construct/
└── registry/

tests/ui/
├── pass/
└── fail/

test-crates/
├── registry-types/
├── registry-impl-a/
├── registry-impl-b/
└── registry-app/
```

compile-fail 测试必须断言有意义的 `.stderr`，而不只断言退出码。调用与构造测试使用析构计数器证明失败路径不丢失、不
重复析构输入；副作用计数器证明预验证失败没有进入用户代码。

### 18.3 需求追踪

实施期间建立 `doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md`，将 284 条需求逐项映射到实现模块和测试。
该矩阵是完成验收的必需产物，不替代测试代码。

## 19. 性能与内存策略

- descriptor 查询热路径是 `TypeId` hash lookup 或静态引用读取；
- registry 初始化只执行一次，允许为完整冲突诊断付出排序和临时索引成本；
- 公开迭代使用 boxed slice，避免每次查询重新排序或分配；
- generic/composite descriptor 按实际查询数量在进程生命周期有界保留；
- 动态字段读取不分配，owned 擦除和 future 允许必要的 `Box`；
- 不为了消除小额初始化分配而引入本仓库 unsafe；
- benchmark 关注 descriptor 热查询、动态字段访问、方法调用、构造和大规模 registry 初始化，不把 Debug/错误路径作为
  吞吐目标。

## 20. 发布与兼容性

- `qubit-reflect` 与 `qubit-reflect-derive` 使用相同版本并一起发布；
- 根 crate 的 `derive` feature 锁定完全相同版本的 derive crate；
- `__private` 虽不属于用户 API，也必须在同版本双 crate 间保持严格兼容；
- 公共 descriptor enum 新增 variant、错误分类和 helper 语法变化按 Cargo SemVer 规则处理；
- README 与用户手册只展示 facade 依赖，不要求普通用户理解双 crate；
- `cargo package` 必须同时验证两个可发布 workspace member；
- runtime-only、默认 feature、全 feature 三种配置都进入 CI。

## 21. 实施边界与顺序约束

实现必须先建立身份、类型表达式、动态值和 interner 等安全基础，再实现字段/variant，随后实现宏、registry、调用、构造
和高级 trait/generic 能力。任何阶段都不得通过临时裸指针或第二套 value trait 绕过尚未完成的安全设计。

推荐的依赖方向为：

```text
identity + expression + errors
        ├─> descriptor core ─> builtin/interner
        ├─> dynamic value ───> field/variant access
        └─> capability ──────> invocation/construction

derive parsing/validation ──> generated adapters/fragments
                                      │
inventory discovery ─────────────────┴─> registry freeze
                                              │
                                              └─> trait/impl effective views
```

具体任务、精确文件、TDD 步骤、调度拓扑和验证命令由独立实施计划定义。

## 22. 已否决方案

### 22.1 单一 Cargo crate

否决原因：Rust 的 `proc-macro` crate 不能同时作为普通 runtime library 导出 descriptor API。采用单仓库双 crate，在不
增加用户依赖负担的前提下满足语言限制。

### 22.2 独立第三个 codegen crate

否决原因：增加发布、版本和跨仓库依赖复杂度。模型 attribute macro 可以委托 facade 重导出的 `Reflect` derive，
不需要调用 reflect derive 的内部 Rust 函数。

### 22.3 直接依赖 `qubit-spi`

否决原因：运行时热注册和 provider selection 语义与不可变 descriptor registry 不一致，会引入无关高层概念。

### 22.4 `linkme::distributed_slice`

否决原因：虽然静态切片模型直观，但本项目更重视跨依赖 crate 的插件式提交和平台验证；选择面向类型化分布式注册的
`inventory`，再由本库完成排序和冲突检测。

### 22.5 显式中心注册表

否决原因：要求最终应用手工枚举 impl 会破坏分散声明的目标，并把漏登责任从宏系统转嫁给用户。

### 22.6 自动 opaque fallback

否决原因：结果会依赖当前可见 impl、查询顺序或 registry 状态，破坏根 descriptor 唯一性。opaque 必须由源码显式
声明。

### 22.7 反射层内建模型约束

否决原因：text/decimal/identifier/reference/validation 等不是 Rust 结构事实，会造成 `rs-reflect` 与
`rs-model-metadata` 职责重叠和反向依赖。

## 23. 设计完成条件

本设计只有在以下条件全部满足时才可进入实施计划：

- crate 与依赖方向无循环；
- 所有公开动态操作都有安全 Rust 实现路径或明确的“只描述”结论；
- descriptor 递归和 generic concrete 唯一化不存在初始化死锁路径；
- static registry 的发现、排序、冲突和平台边界明确；
- trait 默认方法可在不跨文件读取源码的情况下为 concrete impl 生成 adapter；
- opaque 不产生第二根 descriptor；
- Local/ThreadSafe 不通过运行时标志伪造 auto trait；
- 模型层只作为下游通用消费者，不进入依赖或核心语义；
- 需求追踪矩阵能够覆盖最终需求规范的全部 284 条要求。
