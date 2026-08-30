# qubit-reflect 用户指南

`qubit-reflect` 将 Rust 声明转换为安全、不可变的运行时 descriptor。它不读取私有布局，也不使用 unsafe；私有字段的访问由 derive 在声明位置生成的普通 Rust 函数完成。

## 快速开始

```rust
use qubit_reflect::{Reflect, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

let descriptor = TypeDescriptor::of::<User>();
assert_eq!(descriptor.field("id").unwrap().index(), 0);
```

`#[reflect(opaque)]` 可隐藏整个根类型；字段上的同名属性仅隐藏该字段的递归 descriptor。`rename` 提供查询名称，`read_only` 和 `skip` 分别限制动态写入与全部动态访问。

## 枚举与注册表

对 enum 使用相同的 derive。variant descriptor 可检查一个值当前是否匹配，并只允许访问激活 variant 的字段。fieldless 整数 `repr` enum 还记录准确 discriminant，并可按数值反查 variant。

具体 derive 根会提交静态 fragment；调用 `ReflectRegistry::initialize()` 后可按 TypeId、Rust 类型名或查询名发现它们。注册失败会返回结构化错误，而不会部分发布 registry。

## 方法、动态值与边界

使用 `#[reflect]` 标记 trait，使用 `#[reflect_impl]` 标记实现。可调用方法先验证 receiver、参数个数、所有权模式和精确 TypeId，随后才调用用户代码；验证失败会返还原始输入。unsafe、非 Rust ABI、可导致借用逃逸或未具体化泛型的方法仍会被描述，但保持不可调用。

`ReflectedRef`、`ReflectedMut` 和 `ReflectedOwned` 保留 Local/ThreadSafe 模式与借用生命周期。不要把 descriptor 当作序列化协议：TypeId 与运行时地址只在当前进程内有效。

普通 receiver 以及 `Box<Self>`、`Rc<Self>`、`Arc<Self>` 和受支持的 `Pin` 形式可直接调用。其他合法显式 receiver 必须在目标类型上用 `register_type_capabilities!` 登记 `invoke::receiver_adapter_key::<Receiver, Mode>()`，其 `ReceiverAdapter` 只能在安全转换成功时消费 receiver；转换失败必须返还原 receiver。缺少该 capability 的方法仍可查询完整签名，但不会暴露动态调用 adapter。

## 验证

开发时运行 `./align-ci.sh`、`./ci-check.sh`，并在修改宏诊断后运行 `cargo test --test ui_tests`。更完整的需求到实现/测试对应关系见追踪矩阵。
