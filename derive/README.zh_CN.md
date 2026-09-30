# qubit-reflect-derive

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect-derive.svg?color=blue)](https://crates.io/crates/qubit-reflect-derive)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](https://github.com/qubit-ltd/rs-reflect/blob/main/LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

<!-- reflect-contract: facade.explicit=qubit_reflect -->
<!-- reflect-contract: provider.qualified=custom-provider -->

`qubit-reflect-derive` 提供 `Reflect`、`reflect` 和 `reflect_impl` 过程宏，用于生成反射元数据及受检操作适配器。大多数应用应依赖默认启用 `derive` 功能的 `qubit-reflect`，这样可以同时使用这些宏和匹配版本的运行时 API。只有在封装 facade 或编写宏集成时，通常才需要直接依赖这个过程宏 crate。

## 安装

直接使用过程宏时还需要配套的运行时 crate，因为宏展开会调用其版本化的 `__private::codegen_v3` 协议。请在 `rs-reflect` 检出目录的同级位置创建示例应用，并让两个依赖都指向同一份检出：

```toml
[dependencies]
qubit-reflect = { version = "0.1", path = "../rs-reflect", default-features = false }
qubit-reflect-derive = { version = "0.1", path = "../rs-reflect/derive" }
```

此处只说明源码检出的接入方式，不代表已验证当前检出的 registry 发布状态。请保留本地检出布局，因为 `qubit-reflect` 还依赖同级的 `rust-common/rs-id` 与 `rust-common/rs-datatype`。它们都要求 Rust 1.94 或更高版本。普通应用依赖默认启用 `derive` feature 的 `qubit-reflect` 即可，无需单独添加 derive crate。

使用 registry 依赖时，请选择同一发布线中已经发布的版本：

```toml
[dependencies]
qubit-reflect = { version = "0.1", default-features = false }
qubit-reflect-derive = "0.1"
```

## 快速开始

配置编辑器可以按运行时收到的字段名读取对象，同时由应用继续持有对象。派生宏生成类型元数据，运行时描述符按名称查找字段并执行类型检查。

```rust
use qubit_reflect::TypeDescriptor;
use qubit_reflect_derive::Reflect;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    name: String,
}

fn main() {
    let field = TypeDescriptor::of::<User>()
        .field("name")
        .expect("派生描述符包含该字段");
    assert_eq!(field.query_name(), Some("name"));
}
```

`Reflect` 支持结构体和枚举；`reflect` 属性用于描述 trait，`reflect_impl` 用于描述 inherent impl 或 trait impl。无法执行的操作会保留结构化元数据，不会绕过 Rust 的类型和所有权检查。过程宏本身不提供运行时存储或反射操作，这些功能由 `qubit-reflect` 提供。

运行时包的同包 example 中，自动发现的 `crate` 可能指向示例可执行程序。派生声明使用 `#[reflect(crate = qubit_reflect)]`，实现块使用 `#[reflect_impl(crate = qubit_reflect)]`，即可明确选择运行时外观库。三个原生示例均要求 `derive` feature，源码也随运行时包分发；在检出根目录运行 `cargo run --example field_patch`、`cargo run --example customer_patch` 或 `cargo run --example support_action`。

`#[reflect(capabilities(...))]` 中，裸写 `Clone`、`Default`、`Send`、`Sync` 会选择内建能力；限定路径始终选择自定义 provider。例如 `my_crate::Clone` 会调用 `my_crate::Clone::<Self>()`，不会因末段同名而变成内建 `Clone`。

## 延伸阅读

- [中文用户指南](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.zh_CN.md)
- [English user guide](https://github.com/qubit-ltd/rs-reflect/blob/main/doc/user_guide.md)
- [运行时 crate 概览](https://github.com/qubit-ltd/rs-reflect/blob/main/README.zh_CN.md) · [English README](https://github.com/qubit-ltd/rs-reflect/blob/main/README.md)
- [过程宏 API 文档](https://github.com/qubit-ltd/rs-reflect/blob/main/derive/src/lib.rs)

## 测试

以下命令均从 `rs-reflect` 仓库根目录运行：

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./.infra/bin/ci-check.sh

# 检查代码覆盖率
./.infra/bin/coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](https://github.com/qubit-ltd/rs-reflect/blob/main/LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并从
`rs-reflect` 仓库根目录运行 `./.infra/bin/align-ci.sh` 格式化代码、运行 `./.infra/bin/ci-check.sh`
通过 CI 检查，再提交 Pull Request。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
