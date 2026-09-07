# qubit-reflect 用户指南

[English](2026-08-29-qubit-reflect-user-guide.md) · [README](../README.zh_CN.md) · API 文档：`cargo doc --all-features`

本手册面向使用 Rust 1.94 及以上版本、采用 `qubit-reflect` 0.1 的框架和基础库作者。它说明怎样让模式驱动的工具了解 Rust 类型，同时不赋予工具不受限制的值访问或内存布局访问能力。反射始终需要显式选择：宏在声明位置生成普通的安全 Rust 代码；生成的不可变描述符只在当前进程中有效。

## 概念模型

`qubit-reflect` 由四部分协作完成反射：

```text
Rust 声明 --宏--> TypeDescriptor / 成员描述符
                         |
应用对象 --动态值包装器--> 受检适配器 --> 结果或恢复对象
                         |
链接得到的注册片段 --> ReflectRegistry --> 有效类型视图
```

- `TypeDescriptor` 是某个具体反射类型唯一且不可变的根描述符，负责暴露结构视图、字段、枚举分支和构造入口；effective capability 统一由 `ReflectRegistry` 解析。
- `TypeDefinitionDescriptor` 是泛型源码声明的不可执行根；具体实例链接到它并保留已解析实参。
- `ReflectedRef`、`ReflectedMut`、`ReflectedOwned` 分别携带共享借用、可变借用和所有权，使值能安全经过动态边界。
- 字段、构造和调用适配器会先校验访问策略与精确 `TypeId`，再进入用户代码。
- `ReflectRegistry` 只会聚合一次静态链接的 inventory fragment：要么发布完整的冻结注册表，要么返回结构化初始化错误。

反射元数据不能代替领域模型。它不会推导校验规则、持久化 ID、编解码器、业务关系或线协议；查询名称、`TypeId`、descriptor 地址和反射 trait marker 也不是可移植标识。

## 贯穿场景

设想一个配置编辑器。宿主程序拥有 `User` 对象，编辑器收到字段名 `"name"` 后需要显示它的当前值，并且只允许用另一个 `String` 替换。成功标准是宿主对象能看到新名称；目标不对、策略不允许或替换值类型不对时，操作必须在字段发生变化前失败。

## 安装与最小配置

要使用宏，请保持默认 feature：

```toml
[dependencies]
qubit-reflect = { path = "../rs-reflect" }
```

本 crate 当前只作为 Qubit 内部依赖使用，尚未发布到 crates.io。请通过上述 workspace path 或经过批准的内部 Git revision 接入，并确保 `qubit-reflect` 与 `qubit-reflect-derive` 来自同一个 revision。

默认 `derive` feature 会重导出 `Reflect`、`reflect`、`reflect_impl` 三个宏。设置 `default-features = false` 后，运行时和手写注册 API 仍然存在，但这些宏不再被重导出。

请按集成边界选择最窄的依赖配置：

```toml
# 只使用运行时描述符、动态值和手写注册。
qubit-reflect = { path = "../rs-reflect", default-features = false }

# 使用宏，并为 BigDecimal、chrono、UUID 类型提供反射实现。
qubit-reflect = { path = "../rs-reflect", features = ["ecosystem-types"] }

# 使用宏，并为 Qubit DataType、Id 类型提供反射实现。
qubit-reflect = { path = "../rs-reflect", features = ["qubit-types"] }
```

`ecosystem-types` 与 `qubit-types` 相互独立，而且都不属于默认 feature。只使用运行时的下游不会编译这些依赖，也不会在未声明的情况下获得相应 trait 实现。
如果 facade 或元数据 crate 会为这些外部类型生成 descriptor，该 crate 必须在自己的 `qubit-reflect` 依赖上启用对应 feature；仅重导出宏不会自动启用类型族实现。

## 核心工作流

### 1. 为类型派生结构描述符

```rust
use qubit_reflect::{Reflect, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let descriptor = TypeDescriptor::of::<User>();
    assert_eq!(descriptor.query_name(), "User");
}
```

同一个具体类型多次调用 `TypeDescriptor::of::<T>()` 会得到同一份不可变根描述符。递归关系按需解析，因此 `Node -> Vec<Node>` 这样的关系不会导致无限递归初始化。

`#[derive(Reflect)]` 支持 struct 和 enum，字段与变体按源码顺序保留；泛型定义与具体实参分开记录。只有生成的 Rust 代码已经提供静态证明时，`TypeRef` 才会解析到目标类型，运行时不会根据类型名字符串猜测。

### 2. 读取并替换字段

```rust
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let name = TypeDescriptor::of::<User>().field("name").expect("字段存在");
    let mut user = User { id: 7, name: String::from("Ada") };

    let value = name.get(ReflectedRef::new(&user)).expect("允许共享读取");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("Ada"));

    name.set(
        ReflectedMut::new(&mut user),
        ReflectedOwned::new(String::from("Grace")),
    )
    .expect("替换值与字段类型精确匹配");
    assert_eq!(user.name, "Grace");

    let failure = name
        .set(ReflectedMut::new(&mut user), ReflectedOwned::new(9_u64))
        .expect_err("u64 不能替换 String 字段");
    let recovered = failure
        .into_recovery()
        .expect("执行前拒绝会保留所有权")
        .into_value()
        .downcast::<u64>()
        .unwrap_or_else(|_| unreachable!("恢复值保留原始类型"));
    assert_eq!(recovered, 9);
}
```

`get` 需要共享借用，`get_mut`、`set` 需要独占可变借用。进入生成代码前，适配器会检查 receiver 类型、操作策略和替换值的 `TypeId`。如果 `set` 在这些执行前检查中被拒绝，`FieldSetFailure` 的恢复对象会保留字段身份和未改动的 owned 替换值；失败调用结束后目标借用会释放，并不会存进 `FieldSetRecovery`。若适配器已经接收所有权，随后才报告执行错误，`FieldSetFailure::recovery()` 会返回 `None`；不能假定每次失败都能直接重试。

### 3. 用编辑器输入构造新对象

命名结构体通过查询名称提供所有可构造字段：

```rust
use qubit_reflect::{NamedConstructionInput, Reflect, ReflectedOwned, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let value = TypeDescriptor::of::<User>()
        .construct_struct(NamedConstructionInput::new([
            ("id", ReflectedOwned::new(7_u64)),
            ("name", ReflectedOwned::new(String::from("Ada"))),
        ]))
        .expect("输入完整且类型精确")
        .downcast::<User>()
        .unwrap_or_else(|_| unreachable!("该描述符构造 User"));
    assert_eq!(value.name, "Ada");
}
```

元组结构体和单元结构体分别使用 `construct_tuple`、`construct_unit`；enum 的 `VariantDescriptor` 也提供同样的三个构造方法。构造会在消费 owned 输入前检查形状、名称或位置、重复项、缺失项、策略和精确类型。失败时 `ConstructionRecovery` 会按调用方原顺序返还输入。结构体更新也遵循先完整校验、后整体移动的原则，包含实现 `Drop` 的类型。

## 进阶用法

### 通过下游 facade 或宏集成

如果 facade 直接承载 `qubit-reflect` 的派生宏，应在派生宏约定的路径下暴露带版本的生成协议。面向业务代码的公开导出可以独立选择；下面代码属于 facade 库，没有程序入口，因此只编译、不运行；最小示例只导出调用方使用的两个类型：

```rust,no_run
pub use qubit_reflect::Reflect;
pub use qubit_reflect::TypeDescriptor;

#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::codegen_v3;
}
```

业务声明随后可使用 `#[reflect(crate = my_facade)]`。生成代码只需要 `codegen_v3`，facade 无需为宏展开额外重导出 `descriptor`、`construct`、`value` 等 runtime 模块。不要通配重导出 `qubit_reflect` 或它的 `__private`，否则无关的内部实现会被固化成 facade API。下游过程宏应只在自己的精确私有 ABI 中逐项重导出所需协议项。`codegen_v3` 是生成代码与运行时之间的协议，不是供业务代码手写描述符的稳定 API；将来若协议不兼容，应新增版本化模块。

### 描述 trait 与可调用实现

- `#[reflect]` 描述 trait 声明，包括 supertrait、默认方法、关联类型和关联常量。
- `#[reflect_impl]` 描述 inherent impl 或 trait impl，并为 receiver、参数、ABI、返回值均能安全通过动态边界的方法生成调用适配器。
- `#[reflect(rename = "...")]` 仅改查询名称，`rust_name()` 保留源码身份；`skip`、`read_only`、`no_construct`、`no_invoke`、`opaque` 会保留适用的结构事实，同时禁用或限制对应动态操作。

从 registry 或有效类型视图取得 `MethodInstanceDescriptor` 后，用 `invoke_local(registry, invocation)` 显式传入同一个 registry 与 `Invocation`。位置参数是规范入口。运行时按 receiver、参数数量、传递方式、精确类型的顺序校验；在用户代码执行前失败时，`InvocationRecovery` 会完整保留 receiver 与参数。

泛型和 blanket impl 会注册定义级元数据。若要让有限的具体泛型实例参与有效查找或调用，使用 `#[reflect(specialize(...))]`。`#[reflect(thread_safe)]` 会显式请求线程安全适配器，只有生成代码证明 receiver、输入、owned 输出和 future 的边界都满足 Rust 约束时才能通过。线程安全值可以降级到本地模式，但不能靠运行时标志反向升级。

### 调用服务方法并恢复错误输入

同一个 registry 用于查找与执行。`None` 表示静态不支持；`Some(Err(...))` 表示调用失败；成功输出仍需按确切类型解码。

```rust
use qubit_reflect::{Invocation, InvocationOutput, Reflect, ReflectedMut, ReflectedOwned,
                    ReflectRegistry, TypeDescriptor, reflect_impl};
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::{InvocationArg, InvocationErrorKind};

#[derive(Reflect)]
struct Counter { value: u64 }

#[reflect_impl]
impl Counter {
    fn add(&mut self, amount: u64) -> u64 {
        self.value += amount;
        self.value
    }
}

fn main() {
    let registry = ReflectRegistry::initialize().expect("valid declarations");
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>()
        .methods_named_in(registry, "add") else { panic!("unique method") };
    let mut counter = Counter { value: 1 };
    {
        let invocation = Invocation::borrowed_mut(ReflectedMut::new(&mut counter),
            [InvocationArg::Owned(ReflectedOwned::new(2_u64))]);
        let output = method.invoke_local(registry, invocation)
            .expect("statically supported signature")
            .expect("valid receiver and arguments");
        let InvocationOutput::Owned(value) = output else { panic!("owned output") };
        assert_eq!(value.downcast::<u64>().unwrap_or_else(|_| panic!("u64")), 3);
    }

    {
        let invalid = Invocation::borrowed_mut(ReflectedMut::new(&mut counter),
            [InvocationArg::Owned(ReflectedOwned::new(String::from("2")))]);
        let failure = method.invoke_local(registry, invalid)
            .expect("static entry still exists").err().expect("exact types required");
        assert!(matches!(failure.error().kind(), InvocationErrorKind::ArgumentTypeMismatch { .. }));
        let (receiver, arguments) = failure.into_recovery().into_parts();
        drop(receiver);
        let InvocationArg::Owned(value) = arguments.into_vec().pop().unwrap() else {
            panic!("original owned input")
        };
        assert_eq!(value.downcast::<String>().unwrap_or_else(|_| panic!("String")), "2");
    }
    assert_eq!(counter.value, 3);
}
```

### 显式泛型 specialization

下面为 `Service<u8>` 注册有限的具体 impl，通过具体类型查找并执行。

```rust
use qubit_reflect::{Invocation, InvocationOutput, Reflect, ReflectRegistry, TypeDescriptor, reflect_impl};
use qubit_reflect::descriptor::MethodLookup;

#[derive(Reflect)]
struct Service<T> { value: T }

#[reflect_impl(specialize(T = u8))]
impl<T> Service<T> {
    fn answer() -> u8 { 42 }
}

fn main() {
    let registry = ReflectRegistry::initialize().unwrap();
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Service<u8>>()
        .methods_named_in(registry, "answer") else { panic!("explicit specialization") };
    let output = method.invoke_local(registry, Invocation::associated([])).unwrap().unwrap();
    let InvocationOutput::Owned(value) = output else { panic!("owned output") };
    assert_eq!(value.downcast::<u8>().unwrap_or_else(|_| panic!("u8")), 42);
}
```

### Capability 与注册表发现

在相关 crate 已链接后调用 `ReflectRegistry::initialize()`。注册表会事务性聚合 fragment：冲突时返回 `RegistryError`，不会发布部分结果；冻结后类型、名称、trait、impl、capability 和有效方法索引均不会改变。静态内置类型会在首次查询前出现；按需生成的复合类型使用独立 interner，不会改写公开的冻结注册表。

```rust
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::registry::ReflectRegistry;

#[derive(Reflect)]
struct Service;

fn main() {
    let snapshot = ReflectRegistry::initialize().expect("所有 fragment 均通过校验");
    let descriptor = TypeDescriptor::of::<Service>();
    let _methods = descriptor.methods_in(snapshot);
    assert!(snapshot.get(descriptor.type_id()).is_some());
    let clone = snapshot.capability_by_id(descriptor, "qubit.reflect.clone")
        .expect("valid capability declarations");
    assert!(clone.is_none());
}
```

将 snapshot 显式传给 `impls_in`、`methods_in` 或 `methods_named_in`，可以避免隐藏的全局查询依赖。snapshot 一旦生成便不可变；即便全局初始化失败，也不会暴露只构建了一部分的注册表。

`snapshot.definitions()` 可在没有注册任何具体实例时枚举泛型声明，并支持按 `TypeDefinitionId`、Rust 路径或查询名定位。定义级扩展通过 `definition_capability` 或 `definition_capability_by_id` 查询。定义字段只包含 `TypeExpression`，不会伪造值访问 adapter。

`Clone` 和 `Default` 是类型安全的 capability。只有具体类型满足 Rust bound 时才注册，然后用 `clone_key()`、`default_key()` 查询。可安全生成适配器的特殊 `self` receiver 需要在所选 registry 中注册精确的 `ReceiverAdapter`。可以使用全局注册宏或显式 builder；缺少 capability 时入口仍存在，调用返回 `Some(Err(ReceiverAdapterUnavailable))` 并保留输入。静态不支持的签名才没有入口。

### 构建隔离的 registry snapshot

`ReflectRegistry::initialize()` 是进程全局 inventory 的入口。当库、fixture
或测试只拥有一组明确 fragment，且需要与链接得到的 inventory 及全局初始化状态隔离时，
使用 `RegistrySnapshotBuilder`。builder 从空集合开始；添加事实时不会执行 provider，只有
`build()` 成功后才会冻结一份不可变的 `ReflectRegistry`。

```rust
use qubit_reflect::capability::{CapabilityDescriptor, CapabilityKey};
use qubit_reflect::identity::{CapabilityId, FragmentIdentity};
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::TypeDescriptor;

fn source(kind: &str, line: u32) -> FragmentIdentity {
    FragmentIdentity::new("example", "fixture", line, 1, kind, u64::from(line))
}

fn main() -> Result<(), qubit_reflect::RegistryError> {
    let target = TypeDescriptor::of::<u32>();
    let key = CapabilityKey::<u32>::new(
        CapabilityId::new("example.limit").expect("合法的 capability ID"),
    );
    let mut builder = RegistrySnapshotBuilder::new();
    builder
        .add_type(target, source("type", 10))
        .add_type_capabilities(
            target,
            vec![CapabilityDescriptor::with_adapter(key, 7_u32)],
            source("capability", 11),
        );
    let snapshot = builder.build()?;

    assert!(snapshot.get(target.type_id()).is_some());
    assert_eq!(
        snapshot
            .capability(target, key)
            .expect("capability 声明合法"),
        Some(&7),
    );
    assert!(target.methods_in(&snapshot).is_empty());
    Ok(())
}
```

成员资格与 capability payload 相互独立。空 builder 会生成不含注册根的 snapshot。
只调用 `add_type_capabilities` 会生成 capability-only snapshot：`types()` 仍为空，但
`capability()` 和 `capability_by_id()` 仍可解析该目标。其他带类型的输入包括
`add_definition`、`add_trait`、`add_impl_definition`、`add_impl` 和
`add_definition_capabilities`。

需要让属性或方法查询只使用这组事实时，把生成的 snapshot 显式传给
`impls_in`、`methods_in` 或 `methods_named_in`。`build()` 会把所有 identity、链接关系和
capability 冲突作为一个事务统一校验；失败返回 `RegistryError`，不会发布部分结果。请给每个
fragment 提供稳定的 `FragmentIdentity`，这样重复或内容变化的来源可以被诊断。冲突时可读取
`conflicting_fragments()`、`capability_details()`、`capability_target()` 和 `capability_id()`；
intrinsic provider 失败可通过 `intrinsic_conflict()` 与 `Error::source()` 继续追踪。

该 API 不改变生成代码协议。现有 facade 继续暴露 `__private::codegen_v3`，
`qubit-model-metadata` 继续使用独立的模型 ABI v4。intrinsic capability provider 只能依赖静态
类型事实，不能依赖 snapshot，也不能重新进入 registry 初始化。provider 在 capability 缓存锁外执行；
自身 panic 不会被转成能力缺失或 `CapabilityConflict`。

### 迁移 effective capability 查询

原有查询名称直接改为 `Result`，没有兼容的吞错入口。`capabilities` 返回
`Result<&TypeCapabilities, CapabilityConflict>`；typed/textual 单项查询返回
`Result<Option<_>, CapabilityConflict>`。先处理错误，再判断能力是否存在。
错误保留冲突类别、能力 ID 和双方 adapter TypeId；注册阶段也可通过
`RegistryError::intrinsic_conflict()` 和 `Error::source()` 读取原始冲突。

`Ok(None)` 可能是 ID 不存在、typed key 的适配器类型不匹配，或只有事实没有 adapter。
这与整个集合冲突不同。`types_with_capability` 及定义级查询只读冻结索引，不执行 factory，
因此不为这些入口增加 `Result`。传入未注册具体实例时，effective 查询可以执行 intrinsic factory，
但不会将该实例加入 snapshot。

provider 必须只依赖静态类型事实，不能依赖 snapshot、时间或外部可变配置，也不能重入注册表初始化。
泛型 intrinsic factory 在缓存表锁外执行；成功和冲突按具体 `TypeId` 缓存，并发查询共享结果。
provider 自身的 panic 仍会传播，不转成能力缺失或 `CapabilityConflict`。
生成调用适配器遇到 receiver 能力解析失败时，返回结构化调用错误，并恢复原 receiver、
参数值、名称和调用方顺序。调用本身不执行 registry 初始化。

下游 `ModelRegistry::metadata_for` 也返回 `Result<Option<_>, ModelMetadataError>`，
属性查询传播 `PropertyResolutionError`；解析错误通过 `cause()` 保留原因和路径上下文。
`qubit-platform-testkit::link_all::validate_all_models` 返回 `ModelRegistryError`，只验证注册可用性。

### 空结构体的构造方式

| 声明 | 描述符形状 | 构造入口 |
| --- | --- | --- |
| `struct A;` | `StructKind::Unit` | `construct_unit()` |
| `struct B {}` | `StructKind::Named` | `construct_struct(NamedConstructionInput::new([]))` |
| `struct C();` | `StructKind::Tuple` | `construct_tuple(TupleConstructionInput::new([]))` |

上述区别同样适用于 const 泛型空结构体。传错形状返回构造错误，不返回错误类型的值，也不触发内部断言。

### 选择透明、opaque 与线程安全边界

应按下游真正需要的操作选择最窄边界：

| 边界 | 可见能力 | 关键约束 |
| --- | --- | --- |
| 普通反射字段 | 提供 resolved `TypeRef`，可继续导航字段类型 | concrete 字段类型必须实现 `Reflect`。 |
| `#[reflect(opaque)]` 字段 | 支持整体读取、替换、传参和外层构造 | 操作仍要求 `TypeId` 精确匹配；不能导航内部结构，也不能从该成员视图独立构造根对象。 |
| `#[reflect(opaque)]` 类型 | 提供唯一 opaque 根描述符和显式登记的 capability | 不公开字段、variant 或成员级构造入口。 |
| 本地动态包装器 | 对普通本地值和借用执行受检操作 | registry 元数据不能把它升级为 `Send` 或 `Sync`。 |
| `SendReflected*` 包装器 | 在编译期 bound 成立时建立线程安全擦除边界 | 可消费自身并通过 `into_local` 降级；本地包装器不能在运行时升级。 |

模型语义应留在下游。模型层或 schema 层可以把 `FieldDescriptor` 与 validation、持久化、codec、relation、redaction 等元数据关联起来，下游可以用自己拥有的自定义 capability 和 provider 承载这些元数据。`qubit-reflect` 不定义或解释领域语义，模型 crate 仍单向依赖它。

类型级 `thread_safe` 约定会统一覆盖 owned-to-borrow bridge、字段访问、构造、更新与方法适配器：

```rust
use qubit_reflect::Reflect;
use qubit_reflect::SendReflectedMut;
use qubit_reflect::SendReflectedOwned;
use qubit_reflect::SendReflectedRef;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
#[reflect(thread_safe)]
struct SharedCounter {
    value: u64,
}

fn main() {
    let field = TypeDescriptor::of::<SharedCounter>()
        .field("value")
        .expect("派生字段存在");
    let mut counter = SharedCounter { value: 1 };
    let current = field
        .get_thread_safe(SendReflectedRef::new(&counter))
        .expect("线程安全读取适配器可用");
    assert_eq!(current.downcast_ref::<u64>(), Some(&1));
    field
        .set_thread_safe(
            SendReflectedMut::new(&mut counter),
            SendReflectedOwned::new(2_u64),
        )
        .expect("线程安全替换适配器可用");
    assert_eq!(counter.value, 2);
}
```

## 错误与诊断

API 不做隐式转换：不会转换数值、解析字符串、推导 `Into`，也不会在类型擦除后凭空增加 `Send`/`Sync`。

- 字段访问返回 `FieldAccessError`。字段替换在适配器执行前被拒绝时，`FieldSetFailure` 会保留未改动的 owned 替换值；所有权越过适配器边界后才发生的错误不带 recovery payload。
- 构造失败返回 `ConstructionRecovery`，同时携带错误和调用方持有的值。
- 调用前校验失败时，`InvocationRecovery` 会返还 receiver 和参数。
- 访问未激活 enum variant 的字段会得到结构化错误。无字段的整数 `repr` enum 可公开规范化表示和 discriminant；携带数据的 enum 不会被伪造为整数映射。

处理错误时应匹配结构化分类，不要解析 `Display` 文本。重试前先检查 recovery：构造和调用恢复对象会保持调用方输入顺序；`FieldSetFailure::recovery()` 则明确区分可重试的执行前拒绝与已经越过所有权边界的错误。

普通调用不捕获 panic。使用 `#[reflect(catch_unwind)]` 时，在支持的平台上会增加显式的捕获入口；`panic=abort` 构建会报告该能力不可用。异步适配器只返回绑定于调用生命周期的 future，不选择执行器，也不主动 poll；异步方法不能使用 `catch_unwind`。

## 排障

| 现象 | 检查方式 |
| --- | --- |
| `field("...")` 返回 `None` | 请使用查询名称；`rename` 会改查询名称，而 `rust_name()` 保留源码拼写。 |
| 字段操作失败 | 检查包装器是否正确（`ReflectedRef` 或 `ReflectedMut`）、字段策略，以及替换值的精确类型。 |
| 方法可见但不能调用 | 查看不可用原因：泛型方法需要受支持的 specialization；unsafe、variadic、不支持的 ABI、opaque 输出及部分借用/unsized 形式不能穿过动态边界。 |
| 注册表初始化失败 | 检查 `RegistryError`；初始化错误会缓存，修复冲突后需要启动新进程。 |
| 跨线程调用不可用 | 方法必须显式标记 `thread_safe`，并且只在 Rust bound 满足时构造 `SendReflected*` 值。 |
| 外部类型没有 `Reflect` 实现 | 在拥有反射边界的 crate 上启用 `ecosystem-types` 或 `qubit-types`；这些实现默认不会启用。 |
| 通过 facade 派生时找不到生成辅助项 | 检查 `#[reflect(crate = ...)]` 指向的 facade，确认它精确暴露版本匹配的 `__private::codegen_v3`，并确保 facade 与派生宏使用兼容的 `qubit-reflect` 协议版本。 |

## 限制与最佳实践

将反射属性放在拥有该约定的声明附近。对于不希望递归暴露内部结构的类型，使用 opaque 边界；将 descriptor 视为进程内不可变元数据。不要借助反射推导领域规则，也不要试图绕开 Rust 的所有权、隐私、类型或线程安全检查。unsafe 函数、不支持的 ABI、variadic、无法安全擦除的 unsized 值、未 specialize 的泛型和 opaque `impl Trait` 返回值可以被描述，但不能动态调用。
tuple 与可移植函数指针 descriptor 支持 0 到 32 个元素或参数；33 及以上 arity 明确不支持，也不会获得 `Reflect` 实现。

## 延伸阅读

- [README](../README.zh_CN.md) 与 [English README](../README.md)
- [English user guide](2026-08-29-qubit-reflect-user-guide.md)
- 使用 `cargo doc --all-features` 在内部生成 API 文档
- [中文详细设计](2026-09-03-qubit-reflect-design.zh_CN.md) 与 [English design](2026-09-03-qubit-reflect-design.md)
- [中文版需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md)与[追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
- [English requirements](2026-09-03-qubit-reflect-requirements.md) and [traceability matrix](2026-09-03-qubit-reflect-requirements-traceability.md)

### 显式调用迁移与排障

所有 `invoke_*` 入口现在必须传入 registry。查得到方法但调用返回 `ReceiverAdapterUnavailable` 时，检查所选快照是否拥有匹配模式和确切类型的 receiver capability；静态入口存在并不保证能力存在。同一 key 在两个快照可以绑定不同 adapter，不会交叉污染；全局失败也不影响有效本地调用。输出与 future 不借用 registry，但仍借用输入。

旧 `codegen_v2` facade 会编译失败，请把精确导出迁移为 `codegen_v3`；模型 v4 与 `definition_provider_v2` 保持独立。Debug 只输出结构，不执行 provider；provider 自己不得重入初始化。
