# qubit-reflect 用户指南

[English](2026-08-29-qubit-reflect-user-guide.md) · [README](../README.zh_CN.md) · [API 文档](https://docs.rs/qubit-reflect)

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

- `TypeDescriptor` 是某个具体反射类型唯一且不可变的根描述符，负责暴露结构视图、字段、枚举分支、构造入口和 capability。
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
qubit-reflect = "0.1"
```

默认 `derive` feature 会重导出 `Reflect`、`reflect`、`reflect_impl` 三个宏。设置 `default-features = false` 后，运行时和手写注册 API 仍然存在，但这些宏不再被重导出。

## 核心工作流

### 1. 为类型派生结构描述符

```rust
# #![allow(proc_macro_derive_resolution_fallback)]
# #[cfg(feature = "derive")]
# fn main() {
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    id: u64,
    name: String,
}

let descriptor = TypeDescriptor::of::<User>();
assert_eq!(descriptor.query_name(), "User");
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
```

同一个具体类型多次调用 `TypeDescriptor::of::<T>()` 会得到同一份不可变根描述符。递归关系按需解析，因此 `Node -> Vec<Node>` 这样的关系不会导致无限递归初始化。

`#[derive(Reflect)]` 支持 struct 和 enum，字段与变体按源码顺序保留；泛型定义与具体实参分开记录。只有生成的 Rust 代码已经提供静态证明时，`TypeRef` 才会解析到目标类型，运行时不会根据类型名字符串猜测。

### 2. 读取并替换字段

```rust
# #![allow(proc_macro_derive_resolution_fallback)]
# #[cfg(feature = "derive")]
# fn main() {
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    id: u64,
    name: String,
}

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
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
```

`get` 需要共享借用，`get_mut`、`set` 需要独占可变借用。进入生成代码前，适配器会检查 receiver 类型、操作策略和替换值的 `TypeId`。`set` 失败会得到 `FieldSetFailure`，其中的恢复对象同时保留原目标借用和待写入的 owned 值，不会静默丢失输入。

### 3. 用编辑器输入构造新对象

命名结构体通过查询名称提供所有可构造字段：

```rust
# #![allow(proc_macro_derive_resolution_fallback)]
# #[cfg(feature = "derive")]
# fn main() {
use qubit_reflect::{NamedConstructionInput, Reflect, ReflectedOwned, TypeDescriptor};

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    id: u64,
    name: String,
}

let value = TypeDescriptor::of::<User>()
    .construct_struct(NamedConstructionInput::new([
        ("id", ReflectedOwned::new(7_u64)),
        ("name", ReflectedOwned::new(String::from("Ada"))),
    ]))
    .expect("输入完整且类型精确")
    .downcast::<User>()
    .unwrap_or_else(|_| unreachable!("该描述符构造 User"));
assert_eq!(value.name, "Ada");
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
```

元组结构体和单元结构体分别使用 `construct_tuple`、`construct_unit`；enum 的 `VariantDescriptor` 也提供同样的三个构造方法。构造会在消费 owned 输入前检查形状、名称或位置、重复项、缺失项、策略和精确类型。失败时 `ConstructionRecovery` 会按调用方原顺序返还输入。结构体更新也遵循先完整校验、后整体移动的原则，包含实现 `Drop` 的类型。

## 进阶用法

### 描述 trait 与可调用实现

- `#[reflect]` 描述 trait 声明，包括 supertrait、默认方法、关联类型和关联常量。
- `#[reflect_impl]` 描述 inherent impl 或 trait impl，并为 receiver、参数、ABI、返回值均能安全通过动态边界的方法生成调用适配器。
- `#[reflect(rename = "...")]` 仅改查询名称，`rust_name()` 保留源码身份；`skip`、`read_only`、`no_construct`、`no_invoke`、`opaque` 会保留适用的结构事实，同时禁用或限制对应动态操作。

从 registry 或有效类型视图取得 `MethodInstanceDescriptor` 后，用 `Invocation` 调用 `invoke_local`。位置参数是规范入口。运行时按 receiver、参数数量、传递方式、精确类型的顺序校验；在用户代码执行前失败时，`InvocationRecovery` 会完整保留 receiver 与参数。

泛型和 blanket impl 会注册定义级元数据。若要让有限的具体泛型实例参与有效查找或调用，使用 `#[reflect(specialize(...))]`。`#[reflect(thread_safe)]` 会显式请求线程安全适配器，只有生成代码证明 receiver、输入、owned 输出和 future 的边界都满足 Rust 约束时才能通过。线程安全值可以降级到本地模式，但不能靠运行时标志反向升级。

### Capability 与注册表发现

在相关 crate 已链接后调用 `ReflectRegistry::initialize()`。注册表会事务性聚合 fragment：冲突时返回 `RegistryError`，不会发布部分结果；冻结后类型、名称、trait、impl、capability 和有效方法索引均不会改变。静态内置类型会在首次查询前出现；按需生成的复合类型使用独立 interner，不会改写公开的冻结注册表。

`Clone` 和 `Default` 是类型安全的 capability。只有具体类型满足 Rust bound 时才注册，然后用 `clone_key()`、`default_key()` 查询。其他任意 `self` receiver 需要由 `register_type_capabilities!` 注册精确的 `ReceiverAdapter`；否则方法仍可发现，但会给出稳定的不可用原因。

## 错误与诊断

API 不做隐式转换：不会转换数值、解析字符串、推导 `Into`，也不会在类型擦除后凭空增加 `Send`/`Sync`。

- 字段访问返回 `FieldAccessError`；字段替换失败则通过 `FieldSetFailure` 保留输入。
- 构造失败返回 `ConstructionRecovery`，同时携带错误和调用方持有的值。
- 调用前校验失败时，`InvocationRecovery` 会返还 receiver 和参数。
- 访问未激活 enum variant 的字段会得到结构化错误。无字段的整数 `repr` enum 可公开规范化表示和 discriminant；携带数据的 enum 不会被伪造为整数映射。

普通调用不捕获 panic。使用 `#[reflect(catch_unwind)]` 时，在支持的平台上会增加显式的捕获入口；`panic=abort` 构建会报告该能力不可用。异步适配器只返回绑定于调用生命周期的 future，不选择执行器，也不主动 poll；异步方法不能使用 `catch_unwind`。

## 排障

| 现象 | 检查方式 |
| --- | --- |
| `field("...")` 返回 `None` | 请使用查询名称；`rename` 会改查询名称，而 `rust_name()` 保留源码拼写。 |
| 字段操作失败 | 检查包装器是否正确（`ReflectedRef` 或 `ReflectedMut`）、字段策略，以及替换值的精确类型。 |
| 方法可见但不能调用 | 查看不可用原因：泛型方法需要受支持的 specialization；unsafe、variadic、不支持的 ABI、opaque 输出及部分借用/unsized 形式不能穿过动态边界。 |
| 注册表初始化失败 | 检查 `RegistryError`；初始化错误会缓存，修复冲突后需要启动新进程。 |
| 跨线程调用不可用 | 方法必须显式标记 `thread_safe`，并且只在 Rust bound 满足时构造 `SendReflected*` 值。 |

## 限制与最佳实践

将反射属性放在拥有该约定的声明附近。对于不希望递归暴露内部结构的类型，使用 opaque 边界；将 descriptor 视为进程内不可变元数据。不要借助反射推导领域规则，也不要试图绕开 Rust 的所有权、隐私、类型或线程安全检查。unsafe 函数、不支持的 ABI、variadic、无法安全擦除的 unsized 值、未 specialize 的泛型和 opaque `impl Trait` 返回值可以被描述，但不能动态调用。

## 延伸阅读

- [README](../README.zh_CN.md) 与 [English README](../README.md)
- [English user guide](2026-08-29-qubit-reflect-user-guide.md)
- [API 文档](https://docs.rs/qubit-reflect)
- [需求追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
