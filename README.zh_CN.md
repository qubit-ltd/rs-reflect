# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-reflect` 在稳定 Rust 上提供宏生成的结构化与可执行反射，为上层框架提供不可变 descriptor、安全字段访问、动态调用、动态构造、capability 查询和确定性的进程内注册表。

## 面向读者

本项目面向需要在稳定 Rust 上检查并访问类型结构的框架和基础库作者，避免扫描源码、读取私有布局或依赖 rustc 私有 API。

## 安装

```toml
[dependencies]
qubit-reflect = "0.1"
```

## 当前状态

文档约定的反射运行时已经实现，公共入口包括：

- 用于 struct 与 enum 的 `#[derive(Reflect)]`；
- 用于 trait 声明的 `#[reflect]`；
- 用于 inherent impl 与 trait impl 的 `#[reflect_impl]`；
- `TypeDescriptor` 及相关不可变 descriptor 视图；
- 受检动态值、字段访问、方法调用、构造、capability 与 registry 发现；
- 默认本地模式和显式请求的线程安全模式。

crate 使用 `#![forbid(unsafe_code)]`。运行时执行精确校验：不会进行数值转换、字符串解析、`Into` 推导，也不会在类型擦除后凭运行时标志增加 `Send`/`Sync`。

## 快速开始

```rust
use qubit_reflect::{Reflect, ReflectedMut, ReflectedOwned, ReflectedRef, TypeDescriptor};

#[derive(Reflect)]
struct User {
    id: u64,
    name: String,
}

let descriptor = TypeDescriptor::of::<User>();
let name = descriptor.field("name").expect("反射字段存在");
let mut user = User { id: 7, name: String::from("Ada") };

assert_eq!(
    name.get(ReflectedRef::new(&user))
        .expect("受检读取成功")
        .downcast_ref::<String>()
        .map(String::as_str),
    Some("Ada"),
);

name.set(
    ReflectedMut::new(&mut user),
    ReflectedOwned::new(String::from("Grace")),
).expect("受检替换成功");
assert_eq!(user.name, "Grace");
```

反射始终需要显式选择。`rename` 只改变查询名称，不改变 Rust 身份；`skip`、`read_only`、`no_construct` 与 `no_invoke` 会保留结构事实，只禁用对应操作。

## 延伸阅读

- [中文版需求规范](doc/2026-08-28-qubit-reflect-requirements.zh_CN.md)
- [中文用户指南](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- [英文用户指南](doc/2026-08-29-qubit-reflect-user-guide.md)
- [需求追踪矩阵](doc/2026-08-29-qubit-reflect-requirements-traceability.zh_CN.md)
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
