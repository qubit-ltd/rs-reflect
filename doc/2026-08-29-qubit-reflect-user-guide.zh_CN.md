# qubit-reflect 用户手册

[中文 README](../README.zh_CN.md) · [English user guide](2026-08-29-qubit-reflect-user-guide.md) · [Rustdoc 源码入口](../src/lib.rs)

本文适用于当前仓库的 `qubit-reflect 0.1.0`，要求 Rust 1.94 或更高版本，面向需要在运行时按名称访问 Rust 类型的应用和框架开发者。读到[核对编辑结果](#核对编辑结果)，你就能在项目中完成字段查找、读取、替换和错误输入恢复。后面按需查阅对象构造、方法调用、注册表、能力、线程安全及外观库集成。

## 它解决什么问题

以用户资料编辑器为例：服务端持有 `User { id: 7, name: "Ada" }`，编辑界面只传来字段名 `"name"` 和新值 `"Grace"`。普通代码知道字段时可以直接写 `user.name`；但通用编辑器要处理许多类型，不能为每种类型再维护一张字段表。`qubit-reflect` 让类型在声明处生成描述符与受检访问代码，编辑器可按名称找到字段，再对实际对象执行读写。

这里有一道应用边界：HTTP 或表单输入通常是文本，应用需要先解析为字段要求的 Rust 类型，并完成业务校验、鉴权和持久化。反射接口只检查 Rust 类型、借用方式和声明的访问策略，不替应用解释文本，也不决定谁有权修改资料。自定义类型需要主动派生或实现 `Reflect`；任意 Rust 类型不会自动变成可反射对象。

本手册也会用计数器展示按名称调用方法。注册表只保存类型和操作的元数据，不保存业务对象，也不会在运行时加载插件。

## 从哪里开始

1. 按[接入资料编辑器](#接入资料编辑器)安装依赖，运行字段读写程序。
2. 在[核对编辑结果](#核对编辑结果)理解成功值、错误输入和对象状态；到这里，基础接入完成。
3. 需要创建对象时读[用输入构造新对象](#用输入构造新对象)；需要触发业务方法时读[按名称调用业务方法](#按名称调用业务方法)。
4. 编写框架或外观库时，再查[发现类型与能力](#发现类型与能力)、[构建隔离的 registry snapshot](#构建隔离的-registry-snapshot)和[外观库与迁移](#外观库与迁移)。

## 接入资料编辑器

在 `rs-reflect` 的同级目录创建二进制项目：

```bash
cargo new reflect-editor
cd reflect-editor
```

在新项目的 `Cargo.toml` 中加入依赖。这里的 `path` 相对于该文件，保留默认 `derive` feature：

```toml
[dependencies]
qubit-reflect = { version = "0.1", path = "../rs-reflect" }
```

当前检出用于准备 `0.1.0` 发布候选。在 registry 发布并完成消费者验收前，请使用上面的 workspace 路径或经过批准的 Git 修订版，并确保 runtime 与 `qubit-reflect-derive` 来自同一仓库修订版。本地检出还需保留清单引用的 `../../rust-common/rs-id` 与 `../../rust-common/rs-datatype` 路径布局。发布后可使用 `qubit-reflect = "=0.1.0"`。默认 feature 重导出 `Reflect`、`reflect`、`reflect_impl` 三个宏。

下面是独立的入门程序。把它保存为 `src/main.rs`，运行 `cargo run`。程序先读取 `name`，替换为 `"Grace"`，再故意传入错误类型 `9_u64`，检查错误输入是否能取回。

```rust
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let name = TypeDescriptor::of::<User>()
        .field("name")
        .expect("User 声明了 name 字段");
    let mut user = User { id: 7, name: String::from("Ada") };

    let current = name.get(ReflectedRef::new(&user)).expect("允许共享读取");
    assert_eq!(current.downcast_ref::<String>().map(String::as_str), Some("Ada"));

    name.set(
        ReflectedMut::new(&mut user),
        ReflectedOwned::new(String::from("Grace")),
    )
    .expect("替换值是 String");
    assert_eq!(user.name, "Grace");

    let failure = name
        .set(ReflectedMut::new(&mut user), ReflectedOwned::new(9_u64))
        .expect_err("u64 不能写入 String 字段");
    let original = failure
        .into_recovery()
        .expect("执行前拒绝会返还替换值")
        .into_value()
        .downcast::<u64>()
        .unwrap_or_else(|_| unreachable!("恢复值仍是 u64"));
    assert_eq!(original, 9);
    assert_eq!(user.name, "Grace");
}
```

`#[derive(Reflect)]` 在 `User` 声明处生成结构信息和访问适配器。`TypeDescriptor::of::<User>()` 描述类型，不持有 `user`；`field("name")` 根据查询名找到字段。`ReflectedRef` 包装共享借用供 `get` 读取，`ReflectedMut` 包装独占借用供 `get_mut` 或 `set` 修改，`ReflectedOwned` 把新值的所有权交给操作。读出的动态值仍要由应用按确切类型解包，这里使用 `downcast_ref::<String>()`。

入门程序用一次性 `main` 展示 API。放进实际应用时，可把字段操作放在资料服务里，并以应用自己的仓储接口作为保存边界。下面是可放入服务模块的集成代码；调用方负责先加载 `User`、校验权限，并把请求文本解析为 `ReflectedOwned::new(具体字段值)`。`UserRepository` 由应用连接实际存储的代码实现：

```rust,no_run
use qubit_reflect::{FieldSetFailure, Reflect, ReflectedMut, ReflectedOwned, TypeDescriptor};

#[derive(Reflect)]
pub struct User {
    id: u64,
    name: String,
}

pub trait UserRepository {
    fn save(&self, user: &User) -> Result<(), String>;
}

pub enum EditError {
    UnknownField(ReflectedOwned),
    Field(FieldSetFailure),
    Save(String),
}

pub fn replace_field(
    repository: &impl UserRepository,
    user: &mut User,
    field_name: &str,
    value: ReflectedOwned,
) -> Result<(), EditError> {
    let Some(field) = TypeDescriptor::of::<User>().field(field_name) else {
        return Err(EditError::UnknownField(value));
    };
    field.set(ReflectedMut::new(user), value).map_err(EditError::Field)?;
    repository.save(user).map_err(EditError::Save)
}
```

`replace_field` 成功返回说明字段已替换，且仓储的 `save` 报告保存成功。未知字段时，`UnknownField` 原样保留输入；字段操作失败时，调用方从 `FieldSetFailure` 检查能否恢复输入。如果 `save` 失败，内存中的 `user` 已经改变，应用必须决定回滚、重新加载或重试保存；反射库不负责数据库事务。

### 涉及的核心类型

| 类型 | 在资料编辑器中的职责 |
| --- | --- |
| `Reflect` | 让声明提供反射描述符；派生支持结构体和枚举。 |
| `TypeDescriptor` | 提供具体类型的不可变结构信息和字段查询。 |
| `FieldDescriptor` | 对指定字段执行受检读取、可变借用或替换。 |
| `ReflectedRef` / `ReflectedMut` | 将实际对象按共享或独占借用传给适配器。 |
| `ReflectedOwned` | 将新值的所有权传给替换或构造操作。 |
| `FieldSetFailure` | 报告替换失败，并在执行前拒绝时保存未消费的新值。 |

同一具体类型重复调用 `TypeDescriptor::of::<T>()` 会得到同一份不可变根描述符。派生会按源码顺序保留字段和枚举分支；递归关系按需解析。已知具体类型并只访问字段时，不必先初始化注册表。

## 核对编辑结果

入门程序的第一条断言确认读到了 `"Ada"`，第二条确认合法替换把 `user.name` 改为 `"Grace"`，最后两条确认错误的 `9_u64` 被返还且 `user` 没有再变化。程序不打印业务输出；全部断言通过后正常退出，就是这段入门练习的成功信号。

`set` 会先核对目标类型、访问策略和替换值的精确 `TypeId`，再调用生成的适配器。错误值 `9_u64` 与 `String` 不匹配，发生在执行前，因此 `FieldSetFailure::into_recovery()` 返还原值。恢复对象还保留字段身份，却不保存目标对象的借用；失败调用结束后可以再次借用 `user`。

| 阶段 | 应用会看到什么 | 怎样处理 |
| --- | --- | --- |
| `field("name")` 找不到 | `None`；可能用了源码名而非 `rename` 后的查询名。 | 检查用户输入和 `rust_name()`，不要继续调用字段操作。 |
| 执行前校验拒绝 `set` | `FieldSetFailure::recovery()` 为 `Some`，替换值尚未被消费。 | 读取结构化错误，提示用户修正输入；需要时取回原值。 |
| 适配器接收所有权后报错 | `recovery()` 为 `None`。 | 检查错误和业务状态，不能假定原值可直接重试。 |

`get` 需要共享借用，`get_mut` 与 `set` 需要独占借用。反射不会把 `"9"` 解析为整数、把 `u64` 转为 `String`，也不会推导 `Into`。如果业务保存发生在字段替换之后，保存失败还需由应用决定如何回滚或重试；字段适配器不管理数据库事务。

## 用输入构造新对象

创建资料时，编辑器可能还没有 `User` 实例，只有已经解析成 `u64` 和 `String` 的输入。命名结构体使用字段查询名提交所有可构造字段。下面是独立程序；替换 `src/main.rs` 后运行 `cargo run`，断言验证新对象的名称。

```rust
use qubit_reflect::{NamedConstructionInput, Reflect, ReflectedOwned, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let user = TypeDescriptor::of::<User>()
        .construct_struct(NamedConstructionInput::new([
            ("id", ReflectedOwned::new(7_u64)),
            ("name", ReflectedOwned::new(String::from("Ada"))),
        ]))
        .expect("字段齐全且类型精确匹配")
        .downcast::<User>()
        .unwrap_or_else(|_| unreachable!("描述符构造 User"));
    assert_eq!(user.name, "Ada");
}
```

构造入口先检查形状、字段名或位置、重复与缺失输入、访问策略以及精确类型，然后才消费自有值。失败时 `ConstructionRecovery` 连同结构化错误一起返还输入，并保持调用方顺序。元组结构体和单元结构体分别使用 `construct_tuple`、`construct_unit`；枚举分支的 `VariantDescriptor` 也提供相应入口。结构体更新遵循先完整校验、后移动字段的规则，包括实现 `Drop` 的类型。

### 空结构体的构造方式

Rust 中三种“空结构体”的形状不同。构造时按声明选择入口；传错形状得到构造错误，不会返回其他形状的值。

| 声明 | 描述符形状 | 构造入口 |
| --- | --- | --- |
| `struct A;` | `StructKind::Unit` | `construct_unit()` |
| `struct B {}` | `StructKind::Named` | `construct_struct(NamedConstructionInput::new([]))` |
| `struct C();` | `StructKind::Tuple` | `construct_tuple(TupleConstructionInput::new([]))` |

这一区别也适用于 const 泛型空结构体。

## 按名称调用业务方法

资料编辑器若要按名称触发对象操作，就需要方法元数据。下面用独立的 `Counter` 展示调用过程：在实现块上标注 `#[reflect_impl]`，从 `ReflectRegistry` 查找 `add`，再用同一个注册表执行。应用仍负责把界面上的 `"2"` 解析为 `u64`；故意把字符串直接传入时，调用应失败并返还字符串。

```rust
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::invoke::{InvocationArg, InvocationErrorKind};
use qubit_reflect::{reflect_impl, Invocation, InvocationOutput, Reflect, ReflectedMut,
                    ReflectedOwned, ReflectRegistry, TypeDescriptor};

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
    let registry = ReflectRegistry::initialize().expect("注册声明有效");
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Counter>()
        .methods_named_in(registry, "add") else { panic!("应找到唯一方法") };
    let mut counter = Counter { value: 1 };

    {
        let input = Invocation::borrowed_mut(
            ReflectedMut::new(&mut counter),
            [InvocationArg::Owned(ReflectedOwned::new(2_u64))],
        );
        let output = method.invoke_local(registry, input)
            .expect("有本地调用入口")
            .expect("接收者和参数有效");
        let InvocationOutput::Owned(value) = output else { panic!("应返回自有值") };
        assert_eq!(value.downcast::<u64>().unwrap_or_else(|_| panic!("应返回 u64")), 3);
    }

    {
        let input = Invocation::borrowed_mut(
            ReflectedMut::new(&mut counter),
            [InvocationArg::Owned(ReflectedOwned::new(String::from("2")))],
        );
        let failure = method.invoke_local(registry, input)
            .expect("调用入口仍存在")
            .err().expect("参数类型错误");
        assert!(matches!(failure.error().kind(), InvocationErrorKind::ArgumentTypeMismatch { .. }));
        let (receiver, arguments) = failure.into_recovery().into_parts();
        drop(receiver);
        let InvocationArg::Owned(value) = arguments.into_vec().pop().unwrap() else {
            panic!("应返还自有参数")
        };
        assert_eq!(value.downcast::<String>().unwrap_or_else(|_| panic!("应为 String")), "2");
    }
    assert_eq!(counter.value, 3);
}
```

正常调用返回 `InvocationOutput::Owned`，解包后得到 `3_u64`；失败分支返回 `InvocationFailure`，错误分类为 `ArgumentTypeMismatch`，原字符串 `"2"` 仍可取回，计数器保持 `3`。`methods_named_in` 的结果可能是缺失、唯一或歧义，应用应分别处理。`invoke_local` 返回 `None` 表示静态上没有本地调用入口；`Some(Err(...))` 表示入口存在，但本次调用没通过校验。调用输出仍需按确切类型解包。

位置参数是规范入口。运行时依次校验接收者、参数数量、传递方式和精确类型；用户代码执行前的失败通过 `InvocationRecovery` 返还接收者、参数和调用方顺序。这里的 `counter` 是业务对象，注册表只参与查找与选择适配器。方法体中的业务规则及其副作用由应用负责。

`#[reflect]` 描述 trait，包括 supertrait、默认方法、关联类型和关联常量。`#[reflect_impl]` 描述 inherent impl 或 trait impl，并只为可以安全跨越动态边界的签名生成适配器。`#[reflect(rename = "...")]` 只改查询名，`rust_name()` 保留源码名；`skip`、`read_only`、`no_construct`、`no_invoke` 和 `opaque` 则限制相应动态操作，同时保留适用的结构信息。

条件编译会先于反射校验。可在声明或成员上使用普通 `#[cfg(...)]` 和 `#[cfg_attr(...)]`；未启用的成员不会进入反射元数据，也不会进入编译后的 Rust 项目。例如，平台专属方法可以引用仅在该平台存在的类型，只要方法也受相同条件约束。`#[reflect(no_invoke)]` 用途不同：该方法仍保留在描述符中，但反射不会为其生成动态调用适配器。

### 为具体泛型实例生成调用入口

泛型和 blanket impl 会登记定义级信息，但不能据此假定所有具体类型都有可调用入口。如果应用只需要动态调用 `Service<u8>`，在实现块上指定 `specialize(T = u8)`。下面的独立程序查找这个具体类型的方法，成功输出 `42_u8`：

```rust
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::{reflect_impl, Invocation, InvocationOutput, Reflect,
                    ReflectRegistry, TypeDescriptor};

#[derive(Reflect)]
struct Service<T> { value: T }

#[reflect_impl(specialize(T = u8))]
impl<T> Service<T> {
    fn answer() -> u8 { 42 }
}

fn main() {
    let registry = ReflectRegistry::initialize().expect("注册声明有效");
    let MethodLookup::Unique(method) = TypeDescriptor::of::<Service<u8>>()
        .methods_named_in(registry, "answer") else { panic!("应找到特化方法") };
    let output = method.invoke_local(registry, Invocation::associated([]))
        .expect("已生成本地调用入口")
        .expect("无参数调用成功");
    let InvocationOutput::Owned(value) = output else { panic!("应返回自有值") };
    assert_eq!(value.downcast::<u8>().unwrap_or_else(|_| panic!("应为 u8")), 42);
}
```

这个特化只给 `Service<u8>` 提供相应支持；对其他类型实参应分别确认是否有注册和调用入口。方法上的 `#[reflect(thread_safe)]` 可以请求线程安全适配器，但生成代码必须证明接收者、参数、自有输出及 Future 满足 Rust 约束。

普通调用会传播用户代码中的 panic。只有明确请求 `#[reflect(catch_unwind)]`，且构建目标支持时，才有捕获入口；`panic=abort` 构建不会提供它。异步适配器返回受输入生命周期约束的 Future，应用自行选择执行器并轮询；异步方法不能使用 `catch_unwind`。

## 发现类型与能力

已知 `User` 时，直接调用 `TypeDescriptor::of::<User>()` 即可。框架若要列出已链接类型、按名称查找方法或获取某种扩展操作，就初始化 `ReflectRegistry`。调用 `initialize()` 后，注册片段通过一次校验汇总为不可变快照；冲突返回 `RegistryError`，不会留下部分结果。下面的独立程序确认 `Service` 已进入类型索引，并检查它没有声明 `qubit.reflect.clone` 能力：

```rust
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::{Reflect, TypeDescriptor};

#[derive(Reflect)]
struct Service;

fn main() {
    let snapshot = ReflectRegistry::initialize().expect("所有注册片段均有效");
    let descriptor = TypeDescriptor::of::<Service>();
    let _methods = descriptor.methods_in(snapshot);
    assert!(snapshot.get(descriptor.type_id()).is_some());
    let clone = snapshot.capability_by_id(descriptor, "qubit.reflect.clone")
        .expect("能力声明有效");
    assert!(clone.is_none());
}
```

快照固定类型、名称、trait、实现、能力和有效方法索引；静态内置类型也在结果中。之后按需生成的复合类型描述符不会成为新的快照成员。把同一快照显式传给 `impls_in`、`methods_in` 或 `methods_named_in`，才能明确这些查询使用哪一组注册事实。

`snapshot.definitions()` 用于枚举泛型定义，即使没有任何具体实例注册；可按 `TypeDefinitionId`、Rust 路径或查询名定位。`TypeDefinitionDescriptor` 描述声明，具体实例保留解析后的类型实参。定义字段中的 `TypeExpression` 只表达类型关系，不提供值访问适配器；`TypeRef` 仅在生成代码能证明具体类型时解析，不会根据字符串猜测。

### 怎样判断能力能否使用

能力是在类型上登记的扩展事实或操作，例如类型安全的 `Clone`、`Default` 适配器。具体类型满足 Rust 约束并注册后，可用 `clone_key()`、`default_key()` 或自定义 `CapabilityKey<A>` 查询。**拥有能力事实**和**成为快照中的类型成员**是两回事；按文本 ID 查询描述符也不等于取得类型安全的适配器。

| `capability_lookup` 状态 | 含义 | 应用操作 |
| --- | --- | --- |
| `Missing` | 没有匹配的能力事实。 | 走应用定义的替代路径。 |
| `FactOnly` | 有事实，但没有执行适配器。 | 可读取元数据，不要尝试当作可调用操作。 |
| `AdapterTypeMismatch` | ID 相同，适配器 Rust 类型与查询键不同。 | 检查能力注册与查询键的契约。 |
| `Found` | 找到类型匹配的适配器。 | 按该能力契约调用。 |

便捷的 `capability` 把 `Found` 映射为 `Ok(Some(&A))`、`Missing` 映射为 `Ok(None)`；`FactOnly` 和 `AdapterTypeMismatch` 是错误，不应被当作缺失。`capability_by_id` 可按文本 ID 读取描述符。要查事实来自哪里，用 `capability_origin` 或 `capability_source`，区分类型自身和注册片段。

某些特殊 `self` 形式还需要所选注册表提供接收者类型、调用模式均匹配的 `ReceiverAdapter`。缺少它时，方法入口仍可能存在，调用返回 `Some(Err(ReceiverAdapterUnavailable))` 并返还输入；静态上不支持的签名则没有入口。

### 构建隔离的 registry snapshot

测试或宿主应用若只想暴露选定类型，可用 `RegistrySnapshotBuilder` 从空集合构建快照。它不读取全局链接目录；添加事实时不执行能力提供器，`build()` 通过校验后才交付不可变的 `ReflectRegistry`。下面的独立程序把 `u32` 加入类型成员，同时只给 `u64` 登记能力：

```rust
use qubit_reflect::capability::{CapabilityDescriptor, CapabilityKey};
use qubit_reflect::identity::{CapabilityId, FragmentIdentity};
use qubit_reflect::registry::RegistrySnapshotBuilder;
use qubit_reflect::TypeDescriptor;
use std::any::TypeId;

fn source(kind: &str, line: u32) -> FragmentIdentity {
    FragmentIdentity::new("example", "fixture", line, 1, kind, u64::from(line))
}

fn main() -> Result<(), qubit_reflect::RegistryError> {
    let target = TypeDescriptor::of::<u32>();
    let key = CapabilityKey::<u32>::new(
        CapabilityId::new("example.limit").expect("能力 ID 有效"),
    );
    let mut builder = RegistrySnapshotBuilder::new();
    builder.add_type_with_capabilities(
        target,
        vec![CapabilityDescriptor::with_adapter(key, 7_u32)],
        source("type", 10),
        source("capability", 11),
    );
    builder.add_type_capabilities(
        TypeDescriptor::of::<u64>(),
        vec![CapabilityDescriptor::with_adapter(key, 8_u32)],
        source("capability", 12),
    );
    let snapshot = builder.build()?;

    assert_eq!(snapshot.capability(target, key).expect("能力有效"), Some(&7));
    assert_eq!(snapshot.types().len(), 1);
    assert_eq!(
        snapshot.capability_only_type_targets("example.limit")[0].0,
        TypeId::of::<u64>(),
    );
    Ok(())
}
```

`u32` 可从 `types()` 中找到；`u64` 虽可查询 `example.limit`，却不是类型成员。`add_type_with_capabilities` 同时添加成员和能力，`add_type_capabilities` 只添加能力。空构建器的 `types()` 为空；仅添加能力也不会让目标出现在 `types()` 中。`capability_only_type_targets(id)` 按稳定能力 ID 枚举这些目标，即使适配器类型不一致也会返回，并按来源片段排序；审计模型元数据时可查 `"qubit.model.metadata.v1"`。

反射本身不要求执行此审计，但模型投影有更严格的约束：若模型元数据能力指向的类型不是 snapshot 成员，`ModelRegistry::from_reflect_registry` 会返回 `UnregisteredModelTarget`。启用泛型模型元数据时，定义目标也遵循相同规则。应检查对应的 `capability_only_*_targets` 结果及其来源片段，再将预期的模型类型或定义加入 snapshot，或从该视图中移除相应元数据注册。能力可查询并不代表它已成为模型成员。

其他入口包括 `add_type`、`add_definition`、`add_trait`、`add_impl_definition`、`add_impl`、`add_definition_capabilities`。模型层的 `ModelRegistry::from_reflect_registry` 只投影快照的类型成员；若只想纳入 `MyModel`，在拥有该类型和 `qubit-model-metadata` 依赖的外观库中按以下集成步骤构建快照：

```text
let mut builder = RegistrySnapshotBuilder::new();
builder.add_type(
    TypeDescriptor::of::<MyModel>(),
    FragmentIdentity::new("example", "models", 1, 1, "type", 1),
);
let snapshot = builder.build()?;
let models = qubit_model_metadata::registry::ModelRegistry::from_reflect_registry(&snapshot)?;
```

这段是集成片段，`MyModel` 和错误返回上下文由应用提供；显式快照不会自动包含其他已链接注册。`build()` 会一起校验标识、引用关系和能力冲突。为片段设置稳定的 `FragmentIdentity`；失败时查看 `RegistryError::conflicting_fragments()`、`capability_details()`、`capability_target()`、`capability_id()`，类型自身的冲突还可从 `intrinsic_conflict()` 与 `Error::source()` 追踪。

### 泛型定义的三种能力查询状态

`definition_capabilities(id)` 查询泛型定义的能力事实，不能单靠它判断定义是否已成为快照成员：

| 返回值 | 说明 | 应用下一步 |
| --- | --- | --- |
| `None` | 既没有该定义成员，也没有相关能力事实。 | 核对定义 ID 和注册来源。 |
| `Some(empty)` | 定义是成员，但没有能力事实。 | 如需扩展操作，添加定义级能力。 |
| `Some(nonempty)` | 有能力事实，定义可能是成员，也可能只是能力目标。 | 用 `definition(id).is_some()` 判断成员资格。 |

例如只调用 `add_definition_capabilities` 而不调用 `add_definition`，就可能得到 `Some(nonempty)`，但 `definition(id)` 仍为 `None`。类型化与文本 ID 便捷查询也能读取能力专用目标；没有匹配事实时才表示能力缺失。

## 选择依赖功能与访问边界

### 选择依赖功能

| Feature | 适用情形 |
| --- | --- |
| `derive`（默认） | 需要 `Reflect`、`reflect`、`reflect_impl` 宏。 |
| `ecosystem-types` | 需要 `BigDecimal`、`DateTime<Utc>`、`NaiveDate`、`NaiveTime`、`Uuid` 的反射实现。 |
| `qubit-types` | 需要 `qubit_id::Id`、`qubit_datatype::DataType` 的反射实现；同时禁用 `qubit-id` 默认的生成器 feature。 |

以下是互相替代的依赖配置。只把符合需求的一项放进 `[dependencies]`；`path` 仍相对于你的 `Cargo.toml`：

```toml
# 只使用运行时描述符、动态值和手写注册 API。
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

两个外部类型 feature 相互独立，均不在默认配置中。需要生成外部类型描述符的外观库或元数据 crate，须在自己的依赖上开启对应 feature；仅重导出宏不会启用这些实现。`qubit-types` 只为 `qubit_id::Id` 提供反射。如果应用还使用 ID 生成器，请直接依赖 `qubit-id` 并启用所需 feature，不要依赖本 crate 间接启用的 feature。

### 什么时候使用不透明边界

普通字段可通过已解析的 `TypeRef` 继续导航内部类型，要求字段类型实现 `Reflect`。如果下游只需整体读取、替换、传参或用外层对象构造，而不应遍历内部结构，可把字段标为 `#[reflect(opaque)]`；仍会核对精确 `TypeId`。将整个类型标为不透明时，只暴露根描述符与显式登记的能力，不公开字段、分支或成员级构造入口。

业务模型的校验、持久化、编解码、关系和脱敏规则由下游负责。下游可用自定义能力与提供器将这些元数据关联到 `FieldDescriptor`，而 `qubit-reflect` 不解释领域含义；模型 crate 保持单向依赖它。

### 什么时候需要线程安全包装器

本地动态值使用 `Reflected*` 包装器。跨线程使用时，只有满足编译期 `Send + Sync` 约束的值才能建立 `SendReflected*` 包装器，类型还要用 `#[reflect(thread_safe)]` 请求字段或构造支持；方法的线程安全适配器须在该方法上单独请求。线程安全包装器可消费自身并转成本地模式，本地包装器不能靠注册表信息或运行时标志升级。

下面的独立程序检查线程安全字段适配器的读取与替换；它不创建线程，只演示该 API 的输入和结果：

```rust
use qubit_reflect::{Reflect, SendReflectedMut, SendReflectedOwned,
                    SendReflectedRef, TypeDescriptor};

#[derive(Reflect)]
#[reflect(thread_safe)]
struct SharedCounter { value: u64 }

fn main() {
    let field = TypeDescriptor::of::<SharedCounter>()
        .field("value")
        .expect("字段存在");
    let mut counter = SharedCounter { value: 1 };
    let current = field.get_thread_safe(SendReflectedRef::new(&counter))
        .expect("线程安全读取入口可用");
    assert_eq!(current.downcast_ref::<u64>(), Some(&1));
    field.set_thread_safe(
        SendReflectedMut::new(&mut counter),
        SendReflectedOwned::new(2_u64),
    )
    .expect("线程安全替换入口可用");
    assert_eq!(counter.value, 2);
}
```

## 错误、诊断与排障

按结构化错误分类处理，不要解析 `Display` 文本。字段读取返回 `FieldAccessError`；字段替换的 `FieldSetFailure` 要先检查恢复对象；构造失败会携带 `ConstructionRecovery`；用户代码执行前的调用失败会携带 `InvocationRecovery`。构造和调用的恢复对象保留调用方输入顺序。当前未激活的枚举分支字段也会返回结构化访问错误。

| 现象 | 检查顺序 |
| --- | --- |
| `field("...")` 返回 `None` | 检查 `rename` 后的查询名；`rust_name()` 保留源码拼写。 |
| 字段读取或替换失败 | 检查目标是否是声明类型、借用包装器、访问策略及替换值的精确类型；再看 `FieldSetFailure::recovery()`。 |
| 构造失败 | 检查形状、重复或缺失的字段、名称或位置，以及每个值的类型；从 `ConstructionRecovery` 取回输入。 |
| 方法可见但无法调用 | 区分没有入口的 `None` 与执行前失败的 `Some(Err(...))`；检查参数和所选快照中的接收者能力。 |
| 注册表初始化或快照构建失败 | 查看 `RegistryError` 的冲突来源；全局初始化错误在当前进程缓存，修复声明后启动新进程。 |
| 外部类型没有 `Reflect` 实现 | 在使用该反射边界的 crate 上启用 `ecosystem-types` 或 `qubit-types`。 |
| 跨线程入口不可用 | 检查类型或方法的 `thread_safe` 请求、Rust 约束及 `SendReflected*` 包装器。 |
| 外观库派生找不到辅助项 | 核对 `#[reflect(crate = ...)]` 路径、`__private::codegen_v3` 的精确导出及宏和运行时版本。 |

普通调用会传播方法体的 panic。`catch_unwind` 入口仅在显式请求且平台支持时可用；`panic=abort` 无法提供该入口。异步调用返回 Future，不替应用选择执行器。恢复输入不代表业务操作一定可以重试；应用还要判断用户代码或外围持久化步骤是否产生了副作用。

## 边界与实践清单

- 将反射属性写在拥有该契约的类型或成员声明处。生成代码可能访问私有字段，反射策略不能替代应用鉴权。
- `TypeId`、描述符地址、查询名和 trait 标记只适合作为进程内身份，不能作为序列化或跨进程模型 ID。
- `unsafe` 方法、不支持的 ABI、可变参数、不透明的 `impl Trait` 输出、未具体特化的泛型以及不能安全擦除的动态大小类型，可以保留结构信息，但不保证有动态调用入口。
- 元组和可移植函数指针分别支持 0 到 32 个元素或参数；超过范围没有 `Reflect` 实现。携带数据的枚举不提供整数判别值映射。
- 描述符和按具体类型缓存的能力会在进程中长期保留；初始化与首次解析可能分配内存。性能敏感路径应按应用负载测量。

## 外观库与迁移

本节面向维护统一依赖入口、过程宏或旧版集成的开发者。业务 crate 直接使用派生宏时，不需要配置生成协议。

### 通过外观库导出派生宏

如果业务声明使用 `#[reflect(crate = my_facade)]`，外观库须在约定路径精确导出匹配版本的生成协议。下面是放在外观库 `src/lib.rs` 的最小代码，使用 `cargo check` 检查；它没有程序入口：

```rust,no_run
pub use qubit_reflect::Reflect;
pub use qubit_reflect::TypeDescriptor;

#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::codegen_v3;
}
```

生成代码只需要 `codegen_v3`；外观库无须为宏展开重导出 `descriptor`、`construct`、`value`，也不应通配重导出整个 `qubit_reflect` 或 `__private`。`codegen_v3` 是宏展开代码与运行时之间的版本化协议，不是供业务手写描述符的 API。显式快照不会更换此协议；旧外观库的 `codegen_v2` 导出应升级。下游 `qubit-model-metadata` 的模型元数据 ABI `v7` 与此协议独立。

### 迁移 effective capability 查询

旧集成若把能力查询当作简单 `Option`，需要改为先处理 `Result`：`capabilities` 返回 `Result<&TypeCapabilities, CapabilityConflict>`，类型键查询 `capability` 返回 `Result<Option<&A>, CapabilityAccessError>`，文本 ID 查询 `capability_by_id` 返回 `Result<Option<&CapabilityDescriptor>, CapabilityConflict>`。没有忽略错误的兼容入口。`FactOnly` 和 `AdapterTypeMismatch` 是错误状态，不等于能力缺失；需要完整四态时使用 `capability_lookup`。

类型自身能力声明冲突会包装为 `CapabilityAccessError::IntrinsicConflict`。冲突保留类别、能力 ID 和双方适配器的 `TypeId`；注册阶段可用 `RegistryError::intrinsic_conflict()` 和 `Error::source()` 追踪原始原因。`capability_origin` 区分 `Intrinsic { type_id }` 与 `Registered { source }`；泛型定义对应使用 `definition_capability_origin` 和 `definition_capability_source`。`type_capability_members` 与 `definition_capability_members` 只遍历冻结快照中的成员，不执行 provider；每项携带 lookup 状态、origin 和来源，fact-only 与适配器类型不匹配也会保留。孤儿目标应单独通过 `capability_only_type_targets` 和 `capability_only_definition_targets` 审计。内建能力有类型成员时以类型声明片段为来源，否则使用最早触发检查的片段；未注册具体实例的有效能力查询可能执行其自身工厂，但不会将该实例加入快照。

所有 `invoke_*` 入口必须显式传入选定注册表。方法可见但返回 `ReceiverAdapterUnavailable` 时，检查该快照是否提供类型和模式都匹配的 `ReceiverAdapter`，调用不会自动回退到全局注册表。同一能力键可在不同快照中选择不同适配器；全局初始化失败也不影响有效的本地快照。输出和 Future 不借用注册表，但仍受输入生命周期约束。

能力提供器只能依赖静态类型事实，不应依赖快照、时间或外部可变配置，也不能重入注册表初始化。泛型类型自身的能力工厂在缓存锁外执行，成功或冲突按具体 `TypeId` 缓存并供并发查询共享；提供器的 panic 会传播。`Debug` 只输出结构信息，不执行提供器。

## 延伸阅读

- [中文 README](../README.zh_CN.md) · [English user guide](2026-08-29-qubit-reflect-user-guide.md)
- [Rustdoc 源码入口](../src/lib.rs)：在 `rs-reflect` 目录运行 `cargo doc --all-features --no-deps --open`，查看当前 API 签名。
- [中文详细设计](2026-09-03-qubit-reflect-design.zh_CN.md) · [需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md) · [需求追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
