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

使用 `#[ReflectImpl]` 描述 inherent impl 或 trait impl。反射 trait impl 需要显式提供定义路径，这样即使 impl 使用导入别名，宏也能关联到准确的 trait：

```rust
#[ReflectImpl(reflected_trait = service_api::Service)]
impl ServiceAlias for MyService {}
```

任何裸 `#[ReflectImpl]` trait impl（包括实现本地 `#[Reflect]` trait 的 impl）都只保留源码路径，并作为未关联的 `ExternalIncomplete` application 登记；它不会关联反射 trait 定义，也不会提供默认方法。只有使用 `reflected_trait = ...` 才会链接本地反射 trait 定义；宏不会猜测路径是否指向该定义。`#[reflect(skip)]` 会把字段、variant 或方法排除在反射元数据之外。vNext 宏不再接受其他 helper attribute。

外部 trait 可通过 `reflect_external_trait!` 创建类型化身份 handle，再用 impl 参数显式关联：

这两个参数是调用者的 trait 身份断言。稳定过程宏不能解析 trait alias 并证明它与 impl 路径等价；请确保 handle/provider 对应实际实现的 trait。

```rust
reflect_external_trait! { pub DisplayHandle = std::fmt::Display; }

#[ReflectImpl(external_trait = DisplayHandle)]
impl std::fmt::Display for LocalDisplay {
    // ...
}
```

这会保留 impl 中写出的路径供 descriptor 展示，同时把 application 链接到类型化外部身份。未提供 handle 时，即使路径文本与另一条 registration 相同，该 application 仍保持 `ExternalIncomplete`。

泛型方法声明会先作为元数据保留；使用 `bind_method!` 在编译期绑定具体特化后，才能动态调用。运行时无法根据动态类型 ID 生成 Rust 单态化代码。不支持执行的方法签名仍可检查，并附带不可用原因。

绑定 trait method 时可在函数指针签名后追加 `, reflected_trait = DefinitionPath`，由本地反射 trait provider 和 compiler-resolved application hook 提供定义及默认泛型实参 identity；外部 trait 使用 `, external_trait = Handle`，只提供 exact identity，并按 impl 源路径中明确写出的 type/const 实参绑定显式方法。`register_method_binding!` 接受相同参数。

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
