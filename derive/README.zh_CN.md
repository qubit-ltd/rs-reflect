# `qubit-reflect-derive`

`qubit-reflect-derive` 提供 `#[Reflect]`、`#[ReflectImpl]` 和 `bind_method!` 过程宏，用于生成元数据与适配器。运行时描述符、context 和受检操作由 `qubit-reflect` 提供。

## 安装

大多数应用直接依赖默认启用 `derive` feature 的 `qubit-reflect` 即可：

```toml
[dependencies]
qubit-reflect = "0.2"
```

如果关闭 runtime 的默认 feature，则显式添加匹配版本的 derive crate：

```toml
[dependencies]
qubit-reflect = { version = "0.2", default-features = false }
qubit-reflect-derive = "0.2"
```

runtime 与 derive 版本应保持一致。生成代码会调用带版本号的内部 `__private::codegen_v4` 协议；它是实现边界，不是应用 API。

## 快速开始

```rust
use qubit_reflect::TypeDescriptor;
use qubit_reflect_derive::Reflect;

#[Reflect]
struct User {
    name: String,
}

fn read_name(user: &User) -> Result<&String, Box<dyn std::error::Error>> {
    let field = TypeDescriptor::of::<User>().field("name")?;
    Ok(field.get(user)?)
}
```

使用 `#[ReflectImpl]` 描述 inherent impl 或 trait impl。`#[reflect(skip)]` 会把字段、variant 或方法排除在反射元数据之外。vNext 宏不再接受其他 helper attribute。

泛型方法声明会先作为元数据保留；使用 `bind_method!` 在编译期绑定具体特化后，才能动态调用。运行时无法根据动态类型 ID 生成 Rust 单态化代码。不支持执行的方法签名仍可检查，并附带不可用原因。

## 示例与参考

仓库示例需要启用 `derive` feature，可从 workspace 根目录运行：

```bash
cargo run --example field_patch
cargo run --example customer_patch
cargo run --example support_action
```

- [中文用户手册](../doc/user_guide.zh_CN.md) · [English user guide](../doc/user_guide.md)
- [运行时概览](../README.zh_CN.md) · [English README](../README.md)
- [宏 API 文档](../src/lib.rs)
