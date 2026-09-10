# qubit-reflect 用户指南

[English](2026-08-29-qubit-reflect-user-guide.md) · [README](../README.zh_CN.md)

本手册面向第一次使用 `qubit-reflect` 的 Rust 开发者，适用于当前仓库的 0.1.0 版本，需要 Rust 1.94 或更高版本。你将完成一个配置编辑器的核心操作：按字段名读取和修改对象、处理错误输入，再根据需要扩展到对象构造和方法调用。

## 先判断是否需要反射

普通业务代码知道对象类型和字段，可以直接访问 `user.name`。配置编辑器或通用框架收到的往往只是字段名，无法为每种对象写一套分支。`qubit-reflect` 让类型在声明处提供结构信息和访问代码，框架通过统一接口操作它们，省去手工维护第二份结构定义。

它不会把任意 Rust 类型自动变成可反射对象。自定义类型需要主动派生或实现 `Reflect`。它也不解析表单字符串、不校验业务规则、不负责序列化；这些步骤由应用完成，再把类型正确的值交给反射接口。

## 阅读路线

| 你要完成的任务 | 从哪里开始 |
| --- | --- |
| 首次接入并跑通示例 | [安装与最小配置](#安装与最小配置) → [核心工作流](#核心工作流) |
| 按名称调用业务方法 | [调用方法](#调用方法) |
| 查找已链接类型或控制注册范围 | [发现类型与扩展能力](#发现类型与扩展能力) |
| 使用外部类型、限制结构访问或跨线程操作 | [选择依赖功能与访问边界](#选择依赖功能与访问边界) |
| 定位失败原因 | [错误与诊断](#错误与诊断) → [排障](#排障) |
| 封装外观库或升级旧版集成 | [外观库集成与迁移](#外观库集成与迁移) |

## 贯穿场景与三个基本概念

宿主程序持有 `User { id: 7, name: String::from("Ada") }`。编辑器收到字段名 `"name"`，需要显示当前名称、将其改为 `"Grace"`，并拒绝把 `9_u64` 写入字符串字段。成功标准是名称正确更新，错误输入被原样返还，失败操作没有改动对象。

先理解三个概念即可开始：

- **描述符**：`TypeDescriptor::of::<User>()` 返回 `User` 的类型信息；`field("name")` 找到字段信息。它描述类型，不持有某个 `User` 实例。
- **动态值包装器**：`ReflectedRef` 借用对象用于读取，`ReflectedMut` 独占借用对象用于修改，`ReflectedOwned` 接收一个值的所有权。包装器让统一接口仍能检查精确类型和借用方式。
- **受检操作**：字段描述符的 `get` 或 `set` 把字段信息与实际对象连接起来，返回结果或结构化错误。调用方仍需知道如何处理取出的具体值，例如将名称识别为 `String`。

直接访问已知类型的字段、构造对象时，不需要先初始化注册表。方法查找和扩展能力查询才需要后面的 `ReflectRegistry`。

## 安装与最小配置

在 `rs-reflect` 的同级目录创建示例应用：

```bash
cargo new reflect-editor
cd reflect-editor
```

在生成的 `Cargo.toml` 中添加以下依赖，保留默认 feature 以使用反射宏：

```toml
[dependencies]
qubit-reflect = { version = "0.1", path = "../rs-reflect" }
```

本 crate 当前只作为 Qubit 内部依赖使用，尚未发布到 crates.io。请通过工作区内的相对路径或经过批准的内部 Git 版本接入，并确保运行时 `qubit-reflect` 与派生宏 `qubit-reflect-derive` 来自同一个仓库版本。

默认 `derive` feature 会重导出 `Reflect`、`reflect`、`reflect_impl` 三个宏。设置 `default-features = false` 后，运行时和手写注册 API 仍然存在，但这些宏不再被重导出。

### 运行示例

请先准备 Rust 1.94 或更高版本，以及同一版本的内部仓库依赖。这里的 `path` 相对于应用的 `Cargo.toml`；仓库的 Cargo 清单还引用了 `../../rust-common/rs-id` 和 `../../rust-common/rs-datatype`，本地检出时需保留相应目录布局。

在采用上述默认依赖的二进制 crate 中，将每个带 `main` 的示例分别保存为 `src/main.rs`，运行 `cargo run`。每段都是独立程序，不要把多段拼成一个文件。成功时不会打印业务输出，结果由断言验证；第二步会确认名称改为 `Grace`，错误输入 `9_u64` 被原样返还。外观库的示例标为 `rust,no_run`，应放在库的 `src/lib.rs` 中，用 `cargo check` 检查。

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

`#[derive(Reflect)]` 支持结构体和枚举，字段与枚举分支按源码顺序保留。此处先确认可以取得 `User` 的描述符；泛型定义与类型导航见后面的注册表说明。

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
    assert_eq!(user.name, "Grace");
}
```

`get` 需要共享借用，`get_mut`、`set` 需要独占可变借用。进入生成代码前，适配器会检查目标类型、操作策略和替换值的 `TypeId`。如果 `set` 在这些执行前检查中被拒绝，`FieldSetFailure` 的恢复对象会保留字段身份和未改动的替换值；失败调用结束后目标借用会释放，并不会存进 `FieldSetRecovery`。若适配器已经接收所有权，随后才报告执行错误，`FieldSetFailure::recovery()` 会返回 `None`；不能假定每次失败都能直接重试。

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

元组结构体和单元结构体分别使用 `construct_tuple`、`construct_unit`；枚举的 `VariantDescriptor` 也提供同样的三个构造方法。构造会在消费传入的自有值前检查形状、名称或位置、重复项、缺失项、策略和精确类型。失败时 `ConstructionRecovery` 会按调用方原顺序返还输入。结构体更新也遵循先完整校验、后整体移动的原则，包含实现 `Drop` 的类型。

至此，编辑器已经可以读取名称、完成合法替换、拒绝错误类型并取回输入，还能构造新对象。需要按名称调用业务方法时，继续阅读“调用服务方法并恢复错误输入”；需要控制注册范围时，阅读“构建隔离的 registry snapshot”。

### 空结构体的构造方式

| 声明 | 描述符形状 | 构造入口 |
| --- | --- | --- |
| `struct A;` | `StructKind::Unit` | `construct_unit()` |
| `struct B {}` | `StructKind::Named` | `construct_struct(NamedConstructionInput::new([]))` |
| `struct C();` | `StructKind::Tuple` | `construct_tuple(TupleConstructionInput::new([]))` |

上述区别同样适用于 const 泛型空结构体。传错形状返回构造错误，不返回错误类型的值，也不触发内部断言。

## 调用方法

当编辑器需要触发对象上的业务操作时，用 `#[reflect_impl]` 为实现生成方法信息，再通过同一个注册表查找和调用。下面用独立的计数器示例演示：从 `1` 加到 `3`；传入字符串 `"2"` 时失败，计数器保持 `3`，字符串被返还。这里仍由应用负责把界面输入转换为 `u64`。

### 调用服务方法并恢复错误输入

同一个注册表用于查找与执行。`None` 表示静态不支持；`Some(Err(...))` 表示调用失败；成功输出仍需按确切类型解码。

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

### 描述 trait 与可调用实现

- `#[reflect]` 描述 trait 声明，包括 supertrait、默认方法、关联类型和关联常量。
- `#[reflect_impl]` 描述 inherent impl 或 trait impl，并为 receiver、参数、ABI、返回值均能安全通过动态边界的方法生成调用适配器。
- `#[reflect(rename = "...")]` 仅改查询名称，`rust_name()` 保留源码身份；`skip`、`read_only`、`no_construct`、`no_invoke`、`opaque` 会保留适用的结构事实，同时禁用或限制对应动态操作。

从注册表或有效类型视图取得 `MethodInstanceDescriptor` 后，用 `invoke_local(registry, invocation)` 显式传入同一个注册表与 `Invocation`。位置参数是规范入口。运行时按接收者、参数数量、传递方式、精确类型的顺序校验；在用户代码执行前失败时，`InvocationRecovery` 会完整保留接收者与参数。

泛型实现和覆盖一类类型的通用实现（blanket impl）会注册定义级元数据。要让有限的具体泛型实例参与查找或调用，使用 `#[reflect(specialize(...))]`。方法上的 `#[reflect(thread_safe)]` 用于请求线程安全适配器，接收者、参数、自有输出和异步返回值必须满足相应 Rust 约束。线程安全值可以转为本地模式，本地值不能通过运行时标志反向升级。

### 为具体泛型实例生成调用支持

下面只为 `Service<u8>` 注册一个具体实现，并通过该类型查找和执行方法。其他类型实参不会因此自动获得调用支持。

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

## 发现类型与扩展能力

已知 `User` 类型时，可以直接获得它的描述符。需要发现程序链接了哪些类型、查找方法，或获取某种类型的扩展操作时，才使用注册表。注册表保存类型和操作的元数据，不保存业务对象，也不负责加载动态插件。

通常使用 `ReflectRegistry::initialize()` 收集程序已链接的注册片段。只有库或测试需要自行指定事实集合时，才构建隔离快照。能力是附加在类型上的扩展，例如类型安全的 `Clone` 或 `Default` 操作；它与“该类型是否属于注册表”是两个问题。

### 查询全局注册表与能力

在相关 crate 已链接后调用 `ReflectRegistry::initialize()`。它会一次性校验并汇总注册片段；冲突时返回 `RegistryError`，不会留下部分成功的注册表。初始化成功后，类型、名称、trait、实现、能力和有效方法索引均固定下来。静态内置类型已包含在注册结果中；随后按需创建的复合类型描述符不会成为新的注册表成员。

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

将快照显式传给 `impls_in`、`methods_in` 或 `methods_named_in`，可以明确指定查询所用的注册表。快照一旦生成便不可变；全局初始化失败时不会暴露部分结果。

`snapshot.definitions()` 可在没有注册任何具体实例时枚举泛型声明，并支持按 `TypeDefinitionId`、Rust 路径或查询名定位。定义级扩展通过 `definition_capability` 或 `definition_capability_by_id` 查询。泛型声明由 `TypeDefinitionDescriptor` 描述，具体实例保留解析后的类型实参。定义字段包含 `TypeExpression`，不提供值访问适配器。只有生成代码能够证明字段的具体类型时，`TypeRef` 才能解析到目标描述符；不会根据字符串猜测类型。

`Clone` 和 `Default` 通过类型安全的能力接口提供。具体类型满足 Rust 约束并注册相应能力后，可用 `clone_key()`、`default_key()` 查询。某些特殊 `self` 形式还需要在所选注册表中注册类型精确匹配的 `ReceiverAdapter`，可以使用全局注册宏或显式构建器。缺少接收者能力时，方法入口仍存在，但调用返回 `Some(Err(ReceiverAdapterUnavailable))` 并保留输入；静态不支持的签名则没有调用入口。

### 构建隔离的 registry snapshot

`ReflectRegistry::initialize()` 收集进程内通过 inventory 链接的注册信息。库或测试若需要自行决定注册内容，可改用 `RegistrySnapshotBuilder`，独立于全局初始化状态构建快照。构建器从空集合开始，添加事实时不会执行提供器，只有 `build()` 成功后才会得到不可变的 `ReflectRegistry`。

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

类型成员与能力数据相互独立。空构建器生成的快照不含注册类型；只调用 `add_type_capabilities` 时，`types()` 仍为空，但 `capability()` 和 `capability_by_id()` 可以查询目标的能力。其他注册入口包括
`add_definition`、`add_trait`、`add_impl_definition`、`add_impl` 和
`add_definition_capabilities`。

把生成的快照传给 `impls_in`、`methods_in` 或 `methods_named_in`，即可指定查询范围。`build()` 会统一校验标识、引用关系和能力冲突，失败时返回 `RegistryError`。每个片段应提供稳定的 `FragmentIdentity`，便于定位重复或内容变化的来源。冲突详情可通过 `conflicting_fragments()`、`capability_details()`、`capability_target()` 和 `capability_id()` 查看；类型自身提供的能力发生冲突时，还可通过 `intrinsic_conflict()` 与 `Error::source()` 追踪原因。

## 选择依赖功能与访问边界

### 选择依赖功能

| Feature | 提供的内容 |
| --- | --- |
| `derive`（默认） | `Reflect`、`reflect` 和 `reflect_impl` 宏。 |
| `ecosystem-types` | `BigDecimal`、`DateTime<Utc>`、`NaiveDate`、`NaiveTime` 和 `Uuid` 的反射实现。 |
| `qubit-types` | `qubit_id::Id` 和 `qubit_datatype::DataType` 的反射实现。 |

以下配置分别适用于不同需求，请选择其中一项替换 `[dependencies]` 中的 `qubit-reflect` 条目，不要同时复制三项：

```toml
# 只使用运行时描述符、动态值和手写注册。
qubit-reflect = { version = "0.1", path = "../rs-reflect", default-features = false }
```

```toml
# 使用宏，并为 BigDecimal、chrono、UUID 类型提供反射实现。
qubit-reflect = { version = "0.1", path = "../rs-reflect", features = ["ecosystem-types"] }
```

```toml
# 使用宏，并为 Qubit DataType、Id 类型提供反射实现。
qubit-reflect = { version = "0.1", path = "../rs-reflect", features = ["qubit-types"] }
```

`ecosystem-types` 与 `qubit-types` 相互独立，而且都不属于默认 feature。只使用运行时的下游不会编译这些依赖，也不会在未声明的情况下获得相应 trait 实现。
如果外观库或元数据 crate 需要为这些外部类型生成描述符，应在它自己的 `qubit-reflect` 依赖上启用对应 feature；仅重导出宏不会启用这些反射实现。

### 选择透明、opaque 与线程安全边界

应按下游真正需要的操作选择最窄边界：

| 边界 | 可见能力 | 关键约束 |
| --- | --- | --- |
| 普通反射字段 | 提供已解析的 `TypeRef`，可继续查看字段类型 | 具体字段类型必须实现 `Reflect`。 |
| `#[reflect(opaque)]` 字段 | 支持整体读取、替换、传参和外层构造 | 操作仍要求 `TypeId` 精确匹配；不能导航内部结构，也不能从该成员视图独立构造根对象。 |
| `#[reflect(opaque)]` 类型 | 提供唯一的不透明根描述符和显式登记的能力 | 不公开字段、枚举分支或成员级构造入口。 |
| 本地动态包装器 | 对普通本地值和借用执行受检操作 | 注册表元数据不能把它升级为 `Send` 或 `Sync`。 |
| `SendReflected*` 包装器 | 满足编译期约束后建立线程安全的类型擦除边界 | 可消费自身并通过 `into_local` 降级；本地包装器不能在运行时升级。 |

模型语义应留在下游。模型层可以通过自定义能力及其提供器，将 `FieldDescriptor` 与校验、持久化、编解码、业务关系或脱敏元数据关联起来。`qubit-reflect` 不定义或解释这些领域规则，模型 crate 仍单向依赖它。

下面的类型级 `#[reflect(thread_safe)]` 示例生成线程安全字段访问支持。方法调用的线程安全适配器需要在对应方法上显式请求；仅把对象装入 `SendReflected*` 包装器并不足够：

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

- 字段访问返回 `FieldAccessError`。字段替换在适配器执行前被拒绝时，`FieldSetFailure` 会保留未改动的替换值；所有权越过适配器边界后才发生的错误不携带可恢复的输入。
- 构造失败返回 `ConstructionRecovery`，同时携带错误和调用方持有的值。
- 调用前校验失败时，`InvocationRecovery` 会返还接收者和参数。
- 访问当前未激活的枚举分支字段会得到结构化错误。无字段且使用整数 `repr` 的枚举可提供规范化的表示信息和判别值；携带数据的枚举不提供这种整数映射。

处理错误时应匹配结构化分类，不要解析 `Display` 文本。重试前先检查恢复对象：构造和调用恢复对象会保持调用方输入顺序；`FieldSetFailure::recovery()` 则明确区分可重试的执行前拒绝与已经越过所有权边界的错误。

普通调用不捕获 panic。使用 `#[reflect(catch_unwind)]` 时，在支持的平台上会增加显式的捕获入口；`panic=abort` 构建会报告该能力不可用。异步适配器只返回受调用生命周期约束的 Future，不选择执行器，也不主动轮询；异步方法不能使用 `catch_unwind`。

## 排障

| 现象 | 检查方式 |
| --- | --- |
| `field("...")` 返回 `None` | 请使用查询名称；`rename` 会改查询名称，而 `rust_name()` 保留源码拼写。 |
| 字段操作失败 | 检查包装器是否正确（`ReflectedRef` 或 `ReflectedMut`）、字段策略，以及替换值的精确类型。 |
| 方法可见但不能调用 | 查看不可用原因：泛型方法需要受支持的具体特化；`unsafe` 方法、可变参数、不支持的 ABI、不透明输出及部分借用或动态大小类型不能通过动态边界。 |
| 注册表初始化失败 | 检查 `RegistryError`；初始化错误会缓存，修复冲突后需要启动新进程。 |
| 跨线程调用不可用 | 方法必须显式标记 `thread_safe`，并且只在 Rust 约束满足时构造 `SendReflected*` 值。 |
| 外部类型没有 `Reflect` 实现 | 在拥有反射边界的 crate 上启用 `ecosystem-types` 或 `qubit-types`；这些实现默认不会启用。 |
| 通过 facade 派生时找不到生成辅助项 | 检查 `#[reflect(crate = ...)]` 指向的 facade，确认它精确暴露版本匹配的 `__private::codegen_v3`，并确保 facade 与派生宏使用兼容的 `qubit-reflect` 协议版本。 |

## 限制与最佳实践

将反射属性放在相应类型或成员的声明处。不希望递归公开内部结构时，使用不透明边界。反射描述符是进程内不可变元数据；查询名、`TypeId`、描述符地址和 trait 标记不能用作序列化或跨进程标识。反射不会推导领域规则，也不能绕开 Rust 的所有权、类型和线程安全检查。

`unsafe` 函数、不支持的 ABI、可变参数、不能安全擦除的动态大小类型、未经具体特化的泛型，以及不透明的 `impl Trait` 返回值可以保留描述信息，但不能动态调用。元组支持 0 到 32 个元素，可移植函数指针支持 0 到 32 个参数；超出范围时没有 `Reflect` 实现。

描述符和按具体类型缓存的能力会长期保留在进程中，不是每次调用结束就释放的临时对象。初始化和首次解析可能分配内存，不要据此假定反射操作零分配；如果关注开销，请测量应用实际使用的路径。生成的访问代码可能包含私有字段，因此反射策略不能代替应用层权限检查。

## 外观库集成与迁移

只有维护统一依赖入口、过程宏或旧版本集成的开发者需要本节。直接在业务 crate 中使用派生宏时，无需配置生成协议。

### 通过下游 facade 或宏集成

如果外观库（facade）为调用方提供 `qubit-reflect` 派生宏，需要在约定路径下导出带版本号的生成协议。业务 API 的公开范围可以另外选择。下面是外观库的最小代码，只导出调用方使用的两个类型；它没有程序入口，放入 `src/lib.rs` 后只编译、不运行：

```rust,no_run
pub use qubit_reflect::Reflect;
pub use qubit_reflect::TypeDescriptor;

#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::codegen_v3;
}
```

业务声明随后可使用 `#[reflect(crate = my_facade)]`。生成代码只需要 `codegen_v3`，外观库无需为宏展开额外重导出 `descriptor`、`construct`、`value` 等运行时模块。不要通配重导出 `qubit_reflect` 或它的 `__private`，否则无关的内部实现会成为外观库的 API。下游过程宏应只在自己的精确私有 ABI 中逐项重导出所需协议项。`codegen_v3` 是生成代码与运行时之间的协议，不是供业务代码手写描述符的稳定 API；将来若协议不兼容，应新增版本化模块。

使用显式快照不需要更换生成协议，外观库仍导出 `__private::codegen_v3`。下游 `qubit-model-metadata` 的模型 ABI v4 是独立协议。

### 迁移 effective capability 查询

从旧版本迁移时，原有能力查询需要处理 `Result`，没有忽略错误的兼容入口。`capabilities` 返回 `Result<&TypeCapabilities, CapabilityConflict>`；按类型键或文本 ID 的单项查询返回 `Result<Option<_>, CapabilityConflict>`。先处理冲突，再判断能力是否存在。冲突保留类别、能力 ID 和双方适配器的 `TypeId`；注册阶段可通过 `RegistryError::intrinsic_conflict()` 和 `Error::source()` 读取原始原因。

便捷的 `capability` 查询会把 ID 缺失、类型键的适配器类型不匹配，以及只有事实没有适配器的描述符统一折叠为 `Ok(None)`。需要保留诊断信息时，应使用 `capability_lookup`，它区分四种状态：`Missing`、`FactOnly`、`AdapterTypeMismatch` 和 `Found`。这些状态与能力集合冲突是两回事。`capability_origin` 返回 `CapabilityOrigin::Intrinsic` 或 `CapabilityOrigin::Registered { source }`，`capability_source` 则在能力来自注册时返回贡献它的 `FragmentIdentity`；泛型定义对应使用 `definition_capability_origin` 和 `definition_capability_source`。`types_with_capability` 及定义级查询只读取冻结索引，不执行能力工厂，其返回类型不增加 `Result`。对尚未注册的具体实例查询有效能力时，可以执行该类型自身的能力工厂，但不会把实例加入快照。

类型自身的能力提供器只能依赖静态类型信息，不能依赖快照、时间或外部可变配置，也不能重入注册表初始化。泛型能力工厂在缓存锁外执行，成功或冲突按具体 `TypeId` 缓存，并发查询共享结果。提供器发生 panic 时仍向外传播，不转换为能力缺失或 `CapabilityConflict`。

生成的调用适配器在接收者能力解析失败时返回结构化错误，并恢复原接收者、参数值、名称和调用方顺序。调用过程本身不初始化注册表。

### 显式调用迁移与排障

所有 `invoke_*` 入口都必须传入注册表。方法能够找到，调用却返回 `ReceiverAdapterUnavailable` 时，应检查所选快照是否提供了类型和调用模式都匹配的接收者适配器。同一能力键在不同快照中可以绑定不同适配器，各自独立生效；全局初始化失败也不会影响有效的本地快照调用。输出和 Future 不借用注册表，但仍受输入生命周期约束。

仍导出 `codegen_v2` 的旧外观库会编译失败，需要改为精确导出 `codegen_v3`。模型 ABI v4 与 `definition_provider_v2` 是独立协议。`Debug` 只输出结构信息，不执行提供器；提供器不得重入注册表初始化。

## 术语速查

| 术语 | 本手册中的含义 |
| --- | --- |
| 描述符（descriptor） | 类型或成员的不可变元数据。 |
| 注册片段（fragment）与快照（snapshot） | 前者提供注册事实，后者是校验通过后得到的不可变注册表。 |
| 有效能力（effective capability） | 注册表为目标类型解析出的扩展能力。 |
| 提供器（provider）与适配器（adapter） | 提供器生成能力事实；适配器执行已注册的操作。 |
| 接收者（receiver） | 方法调用中的 `self` 对象或其借用。 |
| 自有值（owned value）与恢复对象（recovery） | 自有值随调用转移所有权；恢复对象在执行前失败时返还输入。 |
| 外观库（facade）与特化（specialization） | 外观库统一提供依赖入口；特化为有限的具体泛型实例生成支持。 |
| 不透明（opaque） | 保留类型身份，但限制内部结构的反射访问。 |

## 延伸阅读

- [README](../README.zh_CN.md) 与 [English README](../README.md)
- [English user guide](2026-08-29-qubit-reflect-user-guide.md)
- [Rustdoc 源码中的 API 概览](../src/lib.rs)；在仓库根目录运行
  `cargo doc --all-features --no-deps --open`，生成并打开完整参考文档
- [中文详细设计](2026-09-03-qubit-reflect-design.zh_CN.md) 与 [English design](2026-09-03-qubit-reflect-design.md)
- [中文演进历史](2026-09-07-qubit-reflect-evolution.zh_CN.md) 与 [Evolution history](2026-09-07-qubit-reflect-evolution.md)
- [中文版需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md) 与 [追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
- [English requirements](2026-09-03-qubit-reflect-requirements.md) and [traceability matrix](2026-09-03-qubit-reflect-requirements-traceability.md)
