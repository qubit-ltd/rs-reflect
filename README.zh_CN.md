# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-reflect` 为框架和基础库作者提供稳定 Rust 上的显式反射能力。它在类型声明处通过宏生成代码，把类型转换为不可变描述符（descriptor），并提供受检的字段访问、动态构造、方法调用、能力查询和进程内注册表发现；整个过程不扫描源码、不读取私有内存布局、不依赖 rustc 私有 API，也不使用 `unsafe`。

## 安装

```toml
[dependencies]
qubit-reflect = { version = "0.1", path = "../rs-reflect" }
```

当前仅允许从 Qubit 内部 workspace 或经过批准的内部 Git revision 引用本 crate，尚未发布到 crates.io。运行时 crate 与 derive crate 必须来自同一个仓库 revision。

默认启用的 `derive` feature 会导出 `#[derive(Reflect)]`、`#[reflect]` 和
`#[reflect_impl]`。关闭默认 feature 后，运行时和手写注册 API 仍然可用，但这三个宏不会被重导出。

外部类型的 `Reflect` 实现均需显式选择：

| Feature | 提供的 `Reflect` 实现 |
| --- | --- |
| `derive`（默认） | 三个反射宏；不会引入外部类型依赖 |
| `ecosystem-types` | `BigDecimal`、`DateTime<Utc>`、`NaiveDate`、`NaiveTime`、`Uuid` |
| `qubit-types` | `qubit_id::Id`、`qubit_datatype::DataType` |

只使用运行时 API 时，可配置
`qubit-reflect = { version = "0.1", path = "../rs-reflect", default-features = false }`。只有确实会穿过反射边界的外部类型族才应启用对应 feature。

## 快速开始

假设正在编写一个由模式驱动的编辑器：它需要按字段名显示和修改对象，而业务代码仍然保有对象。只需在声明处派生描述符，按名称取得字段，再传入正确的借用包装器。真正执行前，适配器会检查目标类型、访问策略和替换值的精确 Rust 类型。

```rust
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

fn main() {
    let descriptor = TypeDescriptor::of::<User>();
    let name = descriptor.field("name").expect("派生字段存在");
    let mut user = User { id: 7, name: String::from("Ada") };

    let current = name.get(ReflectedRef::new(&user)).expect("受检读取成功");
    assert_eq!(current.downcast_ref::<String>().map(String::as_str), Some("Ada"));

    name.set(
        ReflectedMut::new(&mut user),
        ReflectedOwned::new(String::from("Grace")),
    )
    .expect("替换值类型精确匹配");
    assert_eq!(user.name, "Grace");
}
```

## 为什么需要它

Rust 有意不提供不受限制的运行时反射。需要类型图、属性编辑器、插件发现或动态分发的框架，往往只能解析源码、维护一份重复的模式，或在类型擦除时丢失所有权和线程安全边界。`qubit-reflect` 将这些约定留在 Rust 声明中：生成代码只暴露 Rust 能够证明安全的操作；即使某个操作不可用，描述符仍会保留结构事实。

## 核心能力与边界

- 分别描述具体运行时类型与泛型源码定义，并描述 trait、impl 及支持的内置类型族。
- 受检字段读取、可变借用、字段替换、枚举分支判断和动态构造；执行前校验失败时，恢复对象会保留调用方传入的 owned 值。
- 为受支持的方法生成调用适配器，区分本地模式与显式请求的线程安全模式。
- 所有注册入口都汇入同一条经过校验的 inventory fragment 数据流，生成确定性的不可变注册表；它是解析具体类型与泛型定义 effective capability 的唯一公开入口，并提供类型安全的 `Clone`、`Default` adapter。
- derive 与 facade 通过版本化的 `__private::codegen_v2` 协议集成；下游模型生成代码独立使用 ABI v4。
- 动态值明确区分 `Local` 与选择性启用的 `ThreadSafe` 边界；只有生成代码证明类型满足所需 `Send + Sync` 约束时，才会提供线程安全字段访问和构造。

反射能力有明确边界：不会转换数值、解析字符串、推导 `Into`，也不会把本地动态值升级为线程安全模式。`TypeId`、descriptor 地址和 trait marker 仅表示进程内身份，不能作为序列化或跨进程模型 ID。被禁用或暂不支持的操作仍可通过描述符发现，并给出结构化的不可用原因。
tuple 和可移植函数指针 descriptor 支持 0 到 32 个元素或参数；33 及以上 arity 明确不支持，也不会获得 `Reflect` 实现。

## 延伸阅读

- [中文用户指南](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- [English user guide](doc/2026-08-29-qubit-reflect-user-guide.md)
- 使用 `cargo doc --all-features` 在内部生成 API 文档
- [中文详细设计](doc/2026-09-03-qubit-reflect-design.zh_CN.md)
- [English design](doc/2026-09-03-qubit-reflect-design.md)
- [中文版需求规范](doc/2026-08-28-qubit-reflect-requirements.zh_CN.md)
- [需求追踪矩阵](doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
- [English requirements](doc/2026-09-03-qubit-reflect-requirements.md)
- [English traceability matrix](doc/2026-09-03-qubit-reflect-requirements-traceability.md)
- [English README](README.md)

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
