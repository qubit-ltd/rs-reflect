# Qubit Reflect 用户手册

<!-- reflect-contract: runtime.direct=qubit_reflect -->
<!-- reflect-contract: provider.qualified=custom-provider -->
<!-- reflect-contract: panic.async_poll=not-caught -->
<!-- reflect-contract: dispatch.input_recovery=original-input -->
<!-- reflect-contract: examples.native=cargo-example -->
<!-- reflect-contract: typeref.constructor=owned-resolved -->
<!-- reflect-contract: lazy.retry=panic-retry -->
<!-- reflect-contract: field.lookup=linear -->

[中文 README](../README.zh_CN.md) · [English user guide](user_guide.md) · [API 文档](https://docs.rs/qubit-reflect)

本文适用于 `qubit-reflect` 0.2.0，要求 Rust 1.94 或更高版本，面向希望通过宏为自有类型添加反射能力的 Rust 应用与框架开发者。先了解宏如何生成描述符和运行时操作入口，再运行字段示例；客服控制台是这一机制的应用案例。后续章节按需讲解构造、方法调用、注册表、能力和访问边界。已有集成可按需查阅[迁移说明](#迁移说明)。

## 目录

- [它解决什么问题](#它解决什么问题)
- [宏如何生成反射能力](#宏如何生成反射能力)
- [从哪里开始](#从哪里开始)
- [使用宏生成的字段反射](#使用宏生成的字段反射)
- [接入客服控制台](#接入客服控制台)
- [核对 PATCH 结果](#核对-patch-结果)
  - [成功时能看到什么](#成功时能看到什么)
- [用输入构造新对象](#用输入构造新对象)
- [按名称调用业务方法](#按名称调用业务方法)
- [发现类型与能力](#发现类型与能力)
  - [怎样判断能力能否使用](#怎样判断能力能否使用)
  - [构建隔离的 registry snapshot](#构建隔离的-registry-snapshot)
  - [泛型定义的三种能力查询状态](#泛型定义的三种能力查询状态)
- [选择依赖功能与访问边界](#选择依赖功能与访问边界)
  - [选择依赖功能](#选择依赖功能)
  - [什么时候使用不透明边界](#什么时候使用不透明边界)
  - [什么时候需要同步](#什么时候需要同步)
- [错误、诊断与排障](#错误诊断与排障)
- [边界与实践清单](#边界与实践清单)
- [迁移说明](#迁移说明)
  - [迁移 effective capability 查询](#迁移-effective-capability-查询)
- [延伸阅读](#延伸阅读)

## 它解决什么问题

Rust 不提供通用的运行时机制来枚举应用结构体的字段、枚举分支、trait 或实现。框架若需要按名称发现成员或执行运行时选定的操作，通常得手写元数据和分发表，重复类型声明中已有的信息。这些副本容易随着类型修改而过时。

`qubit-reflect` 从标注过的 Rust 声明生成结构描述和受支持操作的适配器。使用方通过不可变描述符和经过类型检查的动态值检查或操作类型。自定义类型和 trait 使用 `#[Reflect]`，实现块使用 `#[ReflectImpl]`；也可手动实现运行时 `Reflect` trait。注册表汇总已链接的类型与实现信息，不保存业务对象，也不在运行时加载插件。

## 宏如何生成反射能力

| 声明处 | 运行时得到什么 |
| --- | --- |
| 结构体、枚举或 trait 上的 `#[Reflect]` | 结构类型、字段、分支和 trait 元数据，以及受支持的适配器。 |
| inherent 或 trait `impl` 上的 `#[ReflectImpl]` | 实现和方法元数据，以及受支持的方法调用适配器。 |
| 字段、分支或方法上的 `#[reflect(skip)]` | 将该成员完全排除在反射约定之外。 |

宏在编译期运行。`TypeDescriptor::of::<T>()` 返回具体类型的描述符。字段操作接受普通引用和值；方法调用使用 `Invocation` builder 和选定的 `ReflectRegistry`。运行时会检查精确 Rust 类型，并保留在操作开始前被拒绝的输入。

`skip` 不会删除 Rust 成员，也不是保密机制。非公开成员可以被描述，但安全操作会返回可见性错误。文档明确标为 `_unsafe` 的操作只绕过可见性；调用方仍须满足类型和其他不变量。带 skip 字段的结构体或枚举分支仍可被描述，但不会生成动态构造适配器，因为宏不能替被隐藏的字段臆造有效初值，也不应增加隐式 trait 约束。

泛型方法声明保留为定义。若需按名称动态调用，须在编译期已知的调用点使用 `bind_method!` 并提供显式函数签名绑定具体实例。运行时 `TypeId` 不能生成新的单态化代码；未绑定的泛型方法没有可调用适配器。

动态构造与更新中的 `Box<dyn Any>` 按盒内具体类型检查。如果字段本身声明为 `Box<dyn Any>`，仍按该字段的精确类型处理。调用方自行加锁并负责同步；反射 API 没有独立的线程安全动态包装模式。

手写 `Reflect` 实现必须为其类型报告相同的 Rust `TypeId`。`TypeRef::of::<T>()` 指向 `TypeDescriptor::of::<T>()`，但不做全局驻留。描述符初始化可能分配；`TypeDescriptor::field` 按名称扫描字段，热路径应按实际记录形状测量查找开销。

## 从哪里开始

1. 先看[宏如何生成反射能力](#宏如何生成反射能力)，再运行[使用宏生成的字段反射](#使用宏生成的字段反射)中的示例，了解派生宏、描述符查询和受检字段访问如何衔接。
2. 要开发按字段驱动的业务流程，再继续阅读通用修改逻辑和[核对 PATCH 结果](#核对-patch-结果)。
3. 需要创建对象时读[用输入构造新对象](#用输入构造新对象)；需要操作按钮触发业务方法时读[按名称调用业务方法](#按名称调用业务方法)。
4. 编写框架时，再查[发现类型与能力](#发现类型与能力)和[构建隔离的 registry snapshot](#构建隔离的-registry-snapshot)；升级已有集成时参阅[迁移说明](#迁移说明)。

## 使用宏生成的字段反射

下面的可运行代码只展示反射边界。`Customer` 仍是调用方持有的普通 Rust 值；`#[Reflect]` 为它生成描述符和受检字段入口。业务流程放在示例之后。

<!-- reflect-source: examples/field_patch.rs -->
```rust
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub credit_limit_cents: u64,
    secret: String,
    #[reflect(skip)]
    internal_note: String,
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
        secret: String::from("private"),
        internal_note: String::from("not reflected"),
    };

    let email = descriptor.field("email").expect("derived field");
    email.set(&mut customer, String::from("ada@corp.example")).expect("exactly typed replacement");
    assert_eq!(customer.email, "ada@corp.example");

    let current: &String = email.get(&customer).expect("shared read");
    assert_eq!(current, "ada@corp.example");

    let secret = descriptor.field("secret").expect("private field is described");
    assert!(matches!(secret.get::<_, String>(&customer), Err(FieldAccessError::NotPublic { .. })));
    // SAFETY: this example owns Customer and intentionally demonstrates the visibility bypass.
    assert_eq!(unsafe { secret.get_unsafe::<_, String>(&customer) }.expect("exact field type"), "private");
    // SAFETY: this example owns Customer and maintains the String field's type invariant.
    unsafe { secret.set_unsafe(&mut customer, String::from("updated")) }.expect("exact replacement type");
    assert_eq!(customer.secret, "updated");
    assert!(descriptor.field("internal_note").is_none());
    assert_eq!(customer.internal_note, "not reflected");
}
```


在依赖中加入：

```toml
[dependencies]
qubit-reflect = "0.2"
```

宏会直接解析 `qubit-reflect` 运行时，同包 example 无须指定 crate 路径。
`qubit-datatype` 与 `qubit-id` 依赖从 crates.io 解析，因此源码检出无需准备相邻仓库。先确认锁定的依赖图可以解析：

```bash
cargo metadata --locked --format-version 1
```

metadata 命令成功表示锁文件中的依赖图可以解析。随后在源码检出根目录运行原生示例：

```bash
cargo run --example field_patch
cargo run --example customer_patch
cargo run --example support_action
```

三个目标均声明 `required-features = ["derive"]`，仅运行时构建会跳过它们。运行时 crate 的分发包包含这些源码，启用 `derive` 后也可在解包目录运行相同命令。上面的版本依赖用于选择已发布版本；源码检出的运行结果不能证明当前检出已发布或可从 registry 安装。

## 接入客服控制台

可运行的 [`customer_patch` 示例](../examples/customer_patch.rs)展示了加载、修改和保存流程。API 层负责解析请求文本并鉴权；补丁层按字段名查找并调用 `field.set(&mut target, value)`。校验失败时，`FieldSetFailure<V>` 会保留原值供检查或重试。若前面的修改已写入内存对象，只有整个补丁成功后才应持久化。

私有字段仍可被描述，但安全 getter/setter 会返回可见性错误。只有在调用方能满足文档中 `# Safety` 前提时，才使用 `_unsafe` 操作绕过可见性。skip 字段不会出现在查询中，因此不能用来实现访问控制。

## 核对 PATCH 结果

最小示例在修改邮箱后读回 `"ada@corp.example"`，并确认被 skip 的 `internal_note` 不在描述符中。程序不打印业务输出；断言全部通过且进程正常退出，就是这段练习的成功信号。

### 成功时能看到什么

`set` 会先核对目标类型、可见性和替换值的精确 `TypeId`，再调用生成的适配器。非公开字段或类型不匹配发生在执行前，`FieldSetFailure::into_parts()` 可返还尚未消费的新值。恢复对象还保留字段身份，却不保存目标对象的借用；失败调用结束后可以再次借用 `customer`。

| 阶段 | 应用会看到什么 | 怎样处理 |
| --- | --- | --- |
| `field("email")` 找不到 | `None`；成员可能被 skip，或者名称与 Rust 声明不符。 | 检查声明以及是否标记了 `#[reflect(skip)]`；skip 成员不可查询。 |
| 执行前校验拒绝 `set` | `FieldSetFailure::recovery()` 为 `Some`，替换值尚未被消费。 | 读取结构化错误，提示用户修正输入；需要时取回原值。 |
| 适配器接收所有权后报错 | `recovery()` 为 `None`。 | 检查错误和业务状态，不能假定原值可直接重试。 |

`get` 需要共享借用，`get_mut` 与 `set` 需要独占借用。反射不会把 `"9"` 解析为整数、把 `u64` 转为 `String`，也不会推导 `Into`。如果业务保存发生在字段替换之后，保存失败还需由应用决定如何回滚或重试；字段适配器不管理数据库事务。

## 用输入构造新对象

构造 builder 接收普通自有值。类型化构造返回具体 Rust 值；动态构造返回 `Box<dyn Any>`。校验会先检查形状、成员名称或索引、重复输入、可见性、缺失值和精确类型，再消费输入。

```rust
use qubit_reflect::{NamedConstructionInput, Reflect, TypeDescriptor};

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
}

fn main() {
    let customer: Customer = TypeDescriptor::of::<Customer>()
        .construct_struct(
            NamedConstructionInput::new()
                .field("id", 1001_u64)
                .field("email", String::from("ada@example.com")),
        )
        .expect("公开字段都有对应输入");
    assert_eq!(customer.email, "ada@example.com");
}
```

校验失败时，`ConstructionRecovery` 按调用方顺序保留输入。结构体更新也接受基础值和普通替换值。动态 `Box<dyn Any>` 输入根据盒内类型检查；使用 `field_any`/`value_any` builder 方法显式传入动态盒。若字段带有 `#[reflect(skip)]`，反射仍保留类型描述，但该结构体或枚举分支没有动态构造适配器：生成代码不能安全初始化被省略的 Rust 字段。

## 按名称调用业务方法

用 `#[ReflectImpl]` 标注方法实现。注册表提供方法视图；`Invocation` builder 接收普通 receiver 借用、普通自有参数或借用参数。调用返回单层 `Result`；在执行前校验失败时，`InvocationFailure` 会保留接收者和未消费输入。

<!-- reflect-source: examples/support_action.rs -->
```rust
use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub suspended: bool,
    pub suspension_reason: Option<String>,
}

#[ReflectImpl]
impl Customer {
    /// Suspends this customer with `reason`, returning false if already
    /// suspended.
    fn suspend(&mut self, reason: String) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        self.suspension_reason = Some(reason);
        true
    }
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let registry = ReflectRegistry::initialize().expect("valid reflection declarations");

    let MethodLookup::Unique(suspend) = TypeDescriptor::of::<Customer>().methods_named_in(&registry, "suspend") else {
        panic!("exactly one method named suspend")
    };

    let mut customer = Customer {
        id: 1001,
        suspended: false,
        suspension_reason: None,
    };
    {
        let invocation = Invocation::receiver_mut(&mut customer).arg(String::from("chargeback dispute"));
        let output = suspend
            .invoke(registry, invocation)
            .expect("valid receiver and arguments");
        let InvocationOutput::Owned(changed) = output else {
            panic!("owned output")
        };
        assert!(DynamicOwned::<Local>::downcast::<bool>(changed).unwrap_or_else(|_| panic!("bool")));
    }
    assert!(customer.suspended);
    assert_eq!(customer.suspension_reason.as_deref(), Some("chargeback dispute"));
}
```

方法查找和调用要使用同一个注册表。安全调用会拒绝非公开方法。`unsafe` 调用入口仅绕过可见性，仍会检查接收者类型、参数类型、借用、Pin，以及适用的枚举分支；调用前应阅读其 `# Safety` 文档。自有值、借用值、Pin 和 Future 输出仍受 Rust 函数签名生命周期约束。

若要动态调用泛型方法，须先为编译期已知的具体实例创建绑定。`bind_method!(方法路径 as fn(签名))` 在绑定点由 Rust 检查方法签名，并记录具体目标类型和泛型实参。未绑定的泛型定义仍可查询，但没有可调用适配器；不能根据运行时 `TypeId` 生成代码。

方法查找可能返回无匹配、唯一或多个候选，应分别处理。普通方法 panic 按 Rust 的 panic 行为传播。异步调用返回 Future，由调用方选择执行器并轮询。

## 发现类型与能力

已知 `Customer` 时，直接调用 `TypeDescriptor::of::<Customer>()` 即可。框架若要列出已链接类型、按名称查找方法或获取某种扩展操作，就初始化 `ReflectRegistry`。调用 `initialize()` 后，注册片段通过一次校验汇总为不可变快照；冲突返回 `RegistryError`，不会留下部分结果。下面的独立程序确认 `Service` 已进入类型索引，并检查它没有声明 `qubit.reflect.clone` 能力：

### 可选值

可选类型描述符将结构信息与运行时访问能力分开。`element_type()` 描述 `T`；`has_ref_projection()` 表示描述符是否带有检查借用 `Option<T>` 的适配器。内置 `Option<T>` 描述符提供此适配器：

```rust
use qubit_reflect::{OptionalProjectionError, TypeDescriptor};

fn main() -> Result<(), OptionalProjectionError> {
let value = Some(12_u32);
let optional = TypeDescriptor::of::<Option<u32>>()
    .as_optional()
    .expect("Option 描述符");
let inner = optional.project_ref(&value)?;
assert_eq!(inner, Some(&12));

let absent: Option<u32> = None;
assert!(optional.project_ref(&absent)?.is_none());
assert!(matches!(
    optional.project_ref(&Some(12_u8)),
    Err(OptionalProjectionError::TypeMismatch(_)),
));
Ok(())
}
```

由结构事实构造的描述符也能描述可选元素，但不持有具体 `Option<T>` 适配器。调用其 `project_ref` 会返回 `OptionalProjectionError::Unavailable`；当适配器可用性决定操作能否绑定时，应先检查 `has_ref_projection()`。投影接受普通共享引用；调用方负责多线程访问所需的同步。

```rust
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::{Reflect, TypeDescriptor};

#[Reflect]
struct Service;

fn main() {
    let snapshot = ReflectRegistry::initialize().expect("所有注册片段均有效");
    let descriptor = TypeDescriptor::of::<Service>();
    let _methods = descriptor.methods_in(&snapshot);
    assert!(snapshot.get(descriptor.type_id()).is_some());
    let clone = snapshot.capability_by_id(descriptor, "qubit.reflect.clone")
        .expect("能力声明有效");
    assert!(clone.is_none());
}
```

快照固定类型、名称、trait、实现、能力和有效方法索引；静态内置类型也在结果中。之后按需生成的复合类型描述符不会成为新的快照成员。`impls_global()`、`methods_global()` 和 `methods_named_global(name)` 会初始化并查询进程全局注册表；对应的 `impls_in(&snapshot)`、`methods_in(&snapshot)` 和 `methods_named_in(&snapshot, name)` 查询显式传入的快照。方法查找与调用必须使用同一注册表：全局查询应初始化一次并将该注册表用于调用，显式快照则在两处都传入同一快照。

`snapshot.definitions()` 用于枚举泛型定义，即使没有任何具体实例注册；可按 `TypeDefinitionId`、Rust 路径或查询名定位。`TypeDefinitionDescriptor` 描述声明，具体实例保留解析后的类型实参。定义字段中的 `TypeExpression` 只表达类型关系，不提供值访问适配器；`TypeRef` 仅在生成代码能证明具体类型时解析，不会根据字符串猜测。

### 怎样判断能力能否使用

`#[reflect(capabilities(...))]` 中，裸写 `Clone`、`Default`、`Send`、`Sync` 选择内建能力；限定路径选择自定义 provider，即使末段与内建名称相同也不例外。`#[reflect(capabilities(my_crate::Clone))]` 会调用 `my_crate::Clone::<Self>()`。

能力是在类型上登记的扩展事实或操作，例如类型安全的 `Clone`、`Default` 适配器。具体类型满足 Rust 约束并注册后，可用 `clone_key()`、`default_key()` 或自定义 `CapabilityKey<A>` 查询。**拥有能力事实**和**成为快照中的类型成员**是两回事；按文本 ID 查询描述符也不等于取得类型安全的适配器。

| `capability_lookup` 状态 | 含义 | 应用操作 |
| --- | --- | --- |
| `Missing` | 没有匹配的能力事实。 | 走应用定义的替代路径。 |
| `FactOnly` | 有事实，但没有执行适配器。 | 可读取元数据，不要尝试当作可调用操作。 |
| `AdapterTypeMismatch` | ID 相同，适配器 Rust 类型与查询键不同。 | 检查能力注册与查询键的契约。 |
| `Found` | 找到类型匹配的适配器。 | 按该能力契约调用。 |

便捷的 `capability` 把 `Found` 映射为 `Ok(Some(&A))`、`Missing` 映射为 `Ok(None)`；`FactOnly` 和 `AdapterTypeMismatch` 是错误，不应被当作缺失。`capability_by_id` 可按文本 ID 读取描述符。要查事实来自哪里，用 `capability_origin` 或 `capability_source`，区分类型自身和注册片段。

某些特殊 `self` 形式还需要所选注册表提供接收者类型、调用模式均匹配的 `ReceiverAdapter`。缺少它时，方法入口仍可能存在，调用返回 `Ok(Err(ReceiverAdapterUnavailable))` 并返还输入；静态上不支持的签名则没有入口。

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

需要能力查询同时要求快照成员资格时，使用 `member_capabilities(type_id)`：上例中的 `u32` 返回 `Some(...)`，仅登记能力的 `u64` 返回 `None`。没有有效能力事实的成员会返回 `Some(empty)`。该入口仅读取快照中已冻结的能力集，不会执行 provider；原有 `capabilities(descriptor)` 和 `capability(...)` 仍可查询 capability-only 目标。

反射本身不要求执行此审计，但模型投影有更严格的约束：若模型元数据能力指向的类型不是 snapshot 成员，`ModelRegistry::from_reflect_registry` 会返回 `UnregisteredModelTarget`。启用泛型模型元数据时，定义目标也遵循相同规则。应检查对应的 `capability_only_*_targets` 结果及其来源片段，再将预期的模型类型或定义加入 snapshot，或从该视图中移除相应元数据注册。能力可查询并不代表它已成为模型成员。

其他入口包括 `add_type`、`add_definition`、`add_trait`、`add_impl_definition`、`add_impl`、`add_definition_capabilities`。模型层的 `ModelRegistry::from_reflect_registry` 只投影快照的类型成员；若只想纳入 `MyModel`，在直接依赖 `qubit-reflect` 和 `qubit-model-metadata` 的 crate 中按以下集成步骤构建快照：

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

例如只调用 `add_definition_capabilities` 而不调用 `add_definition`，就可能得到 `Some(nonempty)`，但 `definition(id)` 仍为 `None`。类型化的 `definition_capability` 与文本 ID 查询 `definition_capability_by_id` 也能从能力专用目标返回匹配的适配器或描述符；目标没有对应事实或能力时才返回 `None`。需要判断快照成员资格时，另行查询 `definition(id)`。存在能力事实但没有可执行适配器时，类型化查询返回 `CapabilityAccessError::FactOnly`；适配器类型与类型键不匹配时返回 `AdapterTypeMismatch`。

`member_definition_capabilities(id)` 将定义成员资格与能力集查询合并：仅作为能力目标的定义返回 `None`，无能力的定义成员返回 `Some(empty)`，有能力的定义成员返回其能力集。该入口只读取快照中冻结的数据，不会执行 provider。

## 选择依赖功能与访问边界

### 选择依赖功能

| Feature | 适用情形 |
| --- | --- |
| `derive`（默认） | 需要 `Reflect`、`ReflectImpl`、`bind_method!` 宏。 |
| `ecosystem-types` | 需要 `BigDecimal`、`DateTime<Utc>`、`NaiveDate`、`NaiveTime`、`Uuid` 的反射实现。 |
| `qubit-types` | 需要 `qubit_id::Id`、`qubit_datatype::DataType` 的反射实现；同时禁用 `qubit-id` 默认的生成器 feature。 |

以下是互相替代的依赖配置。只把符合需求的一项放进 `[dependencies]`；`path` 仍相对于你的 `Cargo.toml`：

```toml
# 只使用运行时描述符、动态值和手写注册 API。
qubit-reflect = { version = "0.2", default-features = false }
```

```toml
# 使用宏，并为 BigDecimal、chrono、UUID 类型提供反射实现。
qubit-reflect = { version = "0.2", features = ["ecosystem-types"] }
```

```toml
# 使用宏，并为 Qubit DataType、Id 类型提供反射实现。
qubit-reflect = { version = "0.2", features = ["qubit-types"] }
```

两个外部类型 feature 相互独立，均不在默认配置中。生成这些外部类型描述符的 crate，须在自己直接声明的 `qubit-reflect` 依赖上开启对应 feature。`qubit-types` 只为 `qubit_id::Id` 提供反射。如果应用还使用 ID 生成器，请直接依赖 `qubit-id` 并启用所需 feature，不要依赖本 crate 间接启用的 feature。

### 什么时候使用不透明边界

普通字段可通过已解析的 `TypeRef` 继续导航内部类型，要求字段类型实现 `Reflect`。如果下游只需整体读取、替换、传参或用外层对象构造，而不应遍历内部结构，可把字段标为 `#[reflect(opaque)]`；仍会核对精确 `TypeId`。将整个类型标为不透明时，只暴露根描述符与显式登记的能力，不公开字段、分支或成员级构造入口。

业务模型的校验、持久化、编解码、关系和脱敏规则由下游负责。下游可用自定义能力与提供器将这些元数据关联到 `FieldDescriptor`，而 `qubit-reflect` 不解释领域含义；模型 crate 保持单向依赖它。

### 什么时候需要同步

反射 API 不提供单独的跨线程动态值模式。若多线程共享反射对象，由调用方持有 mutex 等锁，并在持锁期间调用 `get`、`set` 或 `invoke`。值在线程间移动或共享仍受 Rust 的 `Send` 与 `Sync` 静态约束。

## 错误、诊断与排障

按结构化错误分类处理，不要解析 `Display` 文本。字段读取返回 `FieldAccessError`；字段替换的 `FieldSetFailure` 要先检查恢复对象；构造失败会携带 `ConstructionRecovery`；用户代码执行前的调用失败会携带 `InvocationRecovery`。构造和调用的恢复对象保留调用方输入顺序。当前未激活的枚举分支字段也会返回结构化访问错误。

| 现象 | 检查顺序 |
| --- | --- |
| Cargo 无法解析 `qubit-datatype` 或 `qubit-id` | 检查 registry 网络访问，并运行 `cargo metadata --locked --format-version 1` 查看锁定的依赖图；不需要检出相邻仓库。 |
| `field("...")` 返回 `None` | 检查 Rust 字段名以及该字段是否标记为 `#[reflect(skip)]`；skip 成员完全不进入反射。 |
| 字段读取或替换失败 | 检查目标类型、公开可见性及值的精确类型；从 `FieldSetFailure` 取回原始替换值。 |
| 构造失败 | 检查形状、重复或缺失的字段、名称或位置，以及每个值的类型；从 `ConstructionRecovery` 取回输入。 |
| 方法可见但无法调用 | 检查是否有受支持的适配器；泛型方法还须有编译期 `bind_method!` 具体实例绑定。查看调用失败中的恢复输入。 |
| 注册表初始化或快照构建失败 | 查看 `RegistryError` 的冲突来源；全局初始化错误在当前进程缓存，修复声明后启动新进程。 |
| 外部类型没有 `Reflect` 实现 | 在使用该反射边界的 crate 上启用 `ecosystem-types` 或 `qubit-types`。 |
| 跨线程操作不适用 | 在调用方使用同步机制，并满足值跨线程所需的 Rust `Send`/`Sync` 约束。 |
| 派生宏找不到运行时 | 在当前 crate 直接声明 `qubit-reflect` 依赖，并启用默认的 `derive` feature。 |

不要假设反射会捕获方法 panic；调用遵循方法本身的 Rust panic 行为。异步调用返回 Future，不替应用选择执行器。恢复输入不代表业务操作一定可以重试；应用还要判断用户代码或外围持久化步骤是否产生了副作用。

## 边界与实践清单

- 将反射属性写在拥有该契约的类型或成员声明处。生成代码可能访问私有字段，反射策略不能替代应用鉴权；`skip` 也不是访问控制。
- `TypeId`、描述符地址、查询名和 trait 标记只适合作为进程内身份，不能作为序列化或跨进程模型 ID。
- `unsafe` 方法、不支持的 ABI、可变参数、不透明的 `impl Trait` 输出、未绑定具体实例的泛型方法以及不能安全擦除的动态大小类型，可以保留结构信息，但不保证有动态调用入口。
- 元组和可移植函数指针分别支持 0 到 32 个元素或参数；超过范围没有 `Reflect` 实现。携带数据的枚举不提供整数判别值映射。
- 描述符和按具体类型缓存的能力会在进程中长期保留；初始化与首次解析可能分配内存。性能敏感路径应按应用负载测量。

## 迁移说明

每个派生反射的下游 crate 都必须直接依赖 `qubit-reflect`；生成代码会自动解析该依赖。

### 迁移 effective capability 查询

旧集成若把能力查询当作简单 `Option`，需要改为先处理 `Result`：`capabilities` 返回 `Result<&TypeCapabilities, CapabilityConflict>`，类型键查询 `capability` 返回 `Result<Option<&A>, CapabilityAccessError>`，文本 ID 查询 `capability_by_id` 返回 `Result<Option<&CapabilityDescriptor>, CapabilityConflict>`。没有忽略错误的兼容入口。`FactOnly` 和 `AdapterTypeMismatch` 是错误状态，不等于能力缺失；需要完整四态时使用 `capability_lookup`。

类型自身能力声明冲突会包装为 `CapabilityAccessError::IntrinsicConflict`。冲突保留类别、能力 ID 和双方适配器的 `TypeId`；注册阶段可用 `RegistryError::intrinsic_conflict()` 和 `Error::source()` 追踪原始原因。`capability_origin` 区分 `Intrinsic { type_id }` 与 `Registered { source }`；泛型定义对应使用 `definition_capability_origin` 和 `definition_capability_source`。`type_capability_members` 与 `definition_capability_members` 只遍历冻结快照中的成员，不执行 provider；每项携带 lookup 状态、origin 和来源，fact-only 与适配器类型不匹配也会保留。孤儿目标应单独通过 `capability_only_type_targets` 和 `capability_only_definition_targets` 审计。内建能力有类型成员时以类型声明片段为来源，否则使用最早触发检查的片段；未注册具体实例的有效能力查询可能执行其自身工厂，但不会将该实例加入快照。

`descriptor::InvocationAdapter` 与 `MethodInstanceDescriptor` 的十二个消费调用方法将外层 `Option` 改成了 `Result`：旧 `None` 分支改为处理外层 `Err` 并明确取回输入，旧 `Some(result)` 改为 `Ok(result)`。这是破坏性变更，内层校验、catching 契约及函数指针 ABI 不变。外层 `Err(InvocationUnavailable<I>)` 保留方法执行前的完整原始输入；通过 `reason()` 查看原因，再用 `into_invocation()` 或 `into_parts()` 取回输入。进入用户代码后，已消费的输入和副作用无法恢复。

所有 `invoke_*` 入口必须显式传入选定注册表。方法可见但返回 `ReceiverAdapterUnavailable` 时，检查该快照是否提供类型和模式都匹配的 `ReceiverAdapter`，调用不会自动回退到全局注册表。同一能力键可在不同快照中选择不同适配器；全局初始化失败也不影响有效的本地快照。输出和 Future 不借用注册表，但仍受输入生命周期约束。

能力提供器只能依赖静态类型事实，不应依赖快照、时间或外部可变配置，也不能重入注册表初始化。泛型类型自身的能力工厂在缓存锁外执行，成功或冲突按具体 `TypeId` 缓存并供并发查询共享；提供器的 panic 会传播。`Debug` 只输出结构信息，不执行提供器。

## 延伸阅读

- [README](../README.zh_CN.md) · [API 文档](https://docs.rs/qubit-reflect)
- [English user guide](user_guide.md) · [English README](../README.md)
