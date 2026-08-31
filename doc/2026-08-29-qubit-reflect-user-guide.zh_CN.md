# qubit-reflect 用户指南

`qubit-reflect` 面向需要在稳定 Rust 上发现并操作用户类型的框架作者。它通过宏在声明位置生成安全 Rust 代码，再把不可变 descriptor 登记到进程内 registry；它不读取私有布局、不依赖 rustc 私有 API，也不使用 `unsafe`。

## 安装与 feature

```toml
[dependencies]
qubit-reflect = "0.1"
```

默认 feature 包含三个宏。关闭默认 feature 时仍可使用 runtime 与手写登记，但不会重导出 derive/attribute 宏。

## 从一个业务对象开始

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
let name = descriptor.field("name").expect("字段存在");
let mut user = User { id: 7, name: String::from("Ada") };

let current = name
    .get(ReflectedRef::new(&user))
    .expect("类型和策略校验通过");
assert_eq!(current.downcast_ref::<String>().map(String::as_str), Some("Ada"));

name.set(
    ReflectedMut::new(&mut user),
    ReflectedOwned::new(String::from("Grace")),
).expect("精确类型的替换值");
assert_eq!(user.name, "Grace");
# }
# #[cfg(not(feature = "derive"))]
# fn main() {}
```

descriptor 是进程内不可变对象。同一个 concrete Rust 类型重复查询会得到同一 descriptor；递归类型关系按需解析，因此 `Node -> Vec<Node>` 不会在初始化时递归死锁。

## 三种宏

- `#[derive(Reflect)]` 描述 struct、enum、字段、variant、泛型实参和构造入口。
- `#[reflect]` 描述 trait 声明、supertrait、默认方法、关联类型与关联常量。
- `#[reflect_impl]` 描述 inherent impl 或 trait impl，并为受支持的方法生成调用 adapter。

常用辅助属性：

- `rename = "..."` 只改变查询名称；`rust_name()` 始终保留源码名称。
- `skip` 保留成员及源码索引，但禁用其动态操作。
- `read_only` 禁止字段可变借用和替换。
- `no_construct` 与 `no_invoke` 分别禁用构造和调用。
- `opaque` 隐藏根类型或单个字段的递归结构。
- `thread_safe` 显式请求线程安全调用 adapter。
- `catch_unwind` 显式请求 panic-catching adapter；异步方法不接受该属性，因为 panic 发生在 future poll 阶段。
- `specialize(...)` 为泛型 impl 或方法登记有限 concrete specialization。
- `dyn_compatible(...)` 用于 attribute 宏无法跨声明证明 supertrait dyn compatibility 的场景；生成的真实 `dyn Trait` 代码仍由 rustc 最终验证。

## Descriptor 导航

`TypeDescriptor` 可导航到结构、枚举、primitive、文本、序列、map、set、pointer、tuple、函数和 trait-object 等 typed view。字段与 variant 保持源码顺序；泛型定义和 concrete 实参分离，类型实参、const 实参和定义参数之间可双向按索引导航。

`TypeRef` 可能已经解析，也可能保留符号表达式或 opaque 边界。只有存在静态证明时，框架才把关联类型或 concrete 泛型实参导航到 `TypeDescriptor`；不会根据字符串类型名猜测身份。

## 字段与 enum variant

字段读取需要 shared borrow；`get_mut`/`set` 需要 exclusive mutable borrow。所有 receiver、策略与精确 `TypeId` 都在进入生成 adapter 前校验。

对 enum，variant descriptor 可以检查当前激活分支。访问未激活 variant 的字段会返回结构化错误。fieldless integer `repr` enum 还公开规范化 repr 与 discriminant；普通 data-carrying enum 不伪造整数映射。

`set` 失败时返回 `FieldSetFailure`，其中的 recovery 保留原 target borrow 和 owned replacement value。框架不会把输入值静默丢弃。

## 动态调用

先从 registry 或 effective type view 找到 `MethodInstanceDescriptor`，再用 `Invocation` 提交 receiver 与参数：

```rust
use qubit_reflect::invoke::{Invocation, InvocationArg, InvocationOutput};
use qubit_reflect::descriptor::MethodInstanceDescriptor;
use qubit_reflect::{ReflectedOwned, ReflectedRef};

# fn invoke(method: &MethodInstanceDescriptor, service: &String) {
let output = method
    .invoke_local(Invocation::borrowed(
        ReflectedRef::new(service),
        [InvocationArg::Owned(ReflectedOwned::new(String::from("Ada")))],
    ))
    .expect("本地 adapter 已登记")
    .expect("调用前校验成功");

let InvocationOutput::Owned(value) = output else { unreachable!() };
let greeting = value.downcast::<String>().unwrap_or_else(|_| unreachable!());
# let _ = greeting;
# }
```

位置参数是规范入口。名称绑定只适用于名称唯一的简单 identifier；wildcard、destructure 和 `@` pattern 仍可按位置调用。校验顺序固定为 receiver、数量、passing mode、精确类型；进入用户代码前的失败返回完整 `InvocationRecovery`。

输出区分 unit、owned、shared borrow、mutable borrow 和 future。借用输出记录 `BorrowOrigin::Receiver` 或参数索引。`&str`/`&mut str` 使用专用安全动态 variant；slice 和任意 `dyn Trait` 不能通过通用动态值入口伪造。

普通调用不捕获 panic。只有 `catch_unwind` 方法提供 catching 入口；`panic=abort` 构建会明确报告 catching 不可用。async adapter 只返回保留 `'call` 生命周期的 future，不内置 executor，也不会主动 poll。

## 动态构造与更新

derive 会为可构造的 struct 与 enum variant 生成 adapter：

```rust
use qubit_reflect::{ConstructionRecovery, NamedConstructionInput, ReflectedOwned, TypeDescriptor};
use qubit_reflect::value::Local;

# struct User { id: u64, name: String }
# fn construct(descriptor: &TypeDescriptor) -> Result<(), ConstructionRecovery<Local>> {

let value = descriptor
    .construct_struct(NamedConstructionInput::new([
        ("id", ReflectedOwned::new(7_u64)),
        ("name", ReflectedOwned::new(String::from("Ada"))),
    ]))?
    .downcast::<User>()
    .unwrap_or_else(|_| unreachable!());
# let _ = value;
# Ok(())
# }
```

构造在消费 owned 值前检查 shape、名称或索引、重复、缺失、策略和类型。失败通过 `ConstructionRecovery` 按调用方顺序返还输入。struct update 同样先验证全部 replacement，再移动 base 与字段值，适用于含 `Drop` 的类型。

## Registry、trait 与 impl

```rust
use std::any::TypeId;
use qubit_reflect::{ReflectRegistry, RegistryError};

# struct User;
# fn inspect() -> Result<(), RegistryError> {
let registry = ReflectRegistry::initialize()?;
let root = registry.get(TypeId::of::<User>()).expect("已静态登记");
let effective = registry.effective_view(root.type_id());
# let _ = effective;
# Ok(())
# }
```

registry 事务性聚合 inventory fragment：任何冲突都会使初始化返回结构化错误，不会发布部分结果。冻结后，类型、名称、trait、impl、capability 与 effective method view 都不会再改变。

静态 builtin 会在首次查询前出现在 registry；`Option<Vec<T>>` 等按需 concrete composite 只进入独立 interner，不会在冻结后修改公开 registry。

trait 身份来自生成的 marker `TypeId`；未反射 external trait 使用调用者提供的稳定 `ExternalTraitId`。泛型/blanket impl 只登记定义级 descriptor；只有显式 concrete specialization 进入目标类型的有效 impl 视图并参与调用。

关联类型和关联常量始终保留结构化声明。默认关联常量只有在 trait 声明环境已证明其类型满足 owned 边界时提供 reader；显式 impl override 或 concrete specialization 可以在自己的具体环境重新证明。框架不会在运行时探测任意 concrete 关联类型来升级能力。

## Local、ThreadSafe 与 capability

动态值底层由 mode 参数区分：

- `ReflectedRef`、`ReflectedMut`、`ReflectedOwned` 是默认 Local 包装，不承诺 `Send`/`Sync`。
- `SendReflectedRef`、`SendReflectedMut`、`SendReflectedOwned` 在构造时通过 Rust bound 验证线程安全。

ThreadSafe 可以安全降级为 Local；框架不提供只靠运行时标志的反向升级。`#[reflect(thread_safe)]` 会在方法位置验证 receiver、参数、owned 输出和 future 的 `Send`/`Sync` 条件。

内置 `Clone` 和 `Default` capability 提供带类型的操作适配器。只在具体 trait bound 成立时登记，然后通过 `clone_key()` 或 `default_key()` 查询；key 携带的适配器类型会阻止无类型或类型不匹配的调用：

```rust
use qubit_reflect::capability::{clone_key, default_key};
use qubit_reflect::{ReflectedOwned, TypeDescriptor};

# fn use_capabilities(descriptor: &TypeDescriptor, value: &ReflectedOwned) {
if let Some(cloner) = descriptor.get_capability(clone_key()) {
    let copy = cloner.clone_owned(value).expect("精确的已登记类型");
    let _ = copy;
}
if let Some(default) = descriptor.get_capability(default_key()) {
    let initial = default.create();
    let _ = initial;
}
# }
```

其他 arbitrary self receiver 需要类型作者通过 `register_type_capabilities!` 登记准确的 `ReceiverAdapter`。缺少 capability 时方法仍完整可查询，但调用能力返回稳定的不可用原因。

## 模型层边界

下游模型 runtime 可以充当 facade：重导出 `qubit-reflect` descriptor 与隐藏的生成代码契约，并让 derive facade 通过显式 `#[reflect(crate = model_runtime)]` 路径委托。这样模型包可复用同一份进程内 descriptor graph，同时 `qubit-reflect` 不反向依赖模型层。

反射层只描述 Rust 结构、经检查的动态操作、capability 与 registry 连接。领域校验规则、relation 语义、codec 和 wire format、持久化身份及跨进程模型 ID 仍属于模型层；不得从 `TypeId`、descriptor 地址、查询名称或反射元数据中推断这些语义。

## 错误、恢复与边界

- 所有动态操作都使用精确 Rust 类型身份，不做隐式转换。
- owned downcast、字段写入、构造与调用的预执行失败均提供 recovery。
- TypeId、descriptor 地址和 reflected trait marker 只在当前进程内有效，不是序列化协议或跨进程模型 ID。
- `opaque` 表示有意停止递归导航；它不表示类型未知，也不会绕过动态值校验。
- unsafe fn、unsupported ABI、variadic、无法安全擦除的 unsized 值、未 specialization 泛型与 opaque `impl Trait` 只描述、不调用，并保留有序的结构化原因。
- registry 错误会被缓存；修复登记冲突后需要启动新进程重新初始化。

## 开发与验证

```bash
./align-ci.sh
./ci-check.sh
cargo test --workspace --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps
cargo bench --workspace --all-features --no-run
```

宏诊断变更还应运行 `cargo test --test ui_tests`。完整的需求、实现文件和测试文件对应关系见[需求追踪矩阵](2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)。
