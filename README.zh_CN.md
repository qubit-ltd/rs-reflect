# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-reflect` 让 Rust 程序在运行时按名称查询类型结构、访问字段和调用方法。它适合编写配置编辑器、通用框架和基础库的开发者：在类型声明处添加反射宏后，通用代码就能使用这些信息，无需再维护一份字段表或方法映射。反射代码由宏生成，可在稳定 Rust 上使用。

例如，配置编辑器收到字段名 `"name"` 时，可以读取 `User` 的当前名称，再用一个 `String` 修改它；类型不匹配时会返回错误，并保留尚未被消费的输入。业务程序仍然持有原来的 `User` 对象。

如果业务代码已经知道要访问哪个字段，直接使用 `user.name` 即可。只有操作目标需要在运行时确定，或框架需要统一检查多种类型时，反射才有必要。本库不负责将表单字符串转换成 Rust 值，也不提供序列化或业务校验规则。

## 安装

```toml
[dependencies]
qubit-reflect = { version = "0.1", path = "../rs-reflect" }
```

需要 Rust 1.94 或更高版本。请根据应用的 `Cargo.toml` 所在目录调整 `path`。

当前仅允许从 Qubit 内部 workspace 或经过批准的内部 Git 版本引用本 crate，尚未发布到 crates.io。运行时 crate 与 derive crate 必须来自同一个仓库版本。

默认已启用反射宏，下面的示例无需额外 feature。只使用运行时 API，或需要第三方类型的反射实现时，参阅手册的[依赖配置](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md#选择依赖功能)。

## 快速开始

假设正在编写一个根据类型结构生成表单的编辑器：它需要按字段名显示和修改对象，而业务代码仍然保有对象。只需在声明处派生描述符，按名称取得字段，再传入正确的借用包装器。真正执行前，适配器会检查目标类型、访问策略和替换值的精确 Rust 类型。

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

在采用上述依赖配置的二进制 crate 中，将示例保存为 `src/main.rs`，再运行 `cargo run`。程序会通过断言确认名称从 `Ada` 变为 `Grace`，随后正常退出。

## 为什么需要它

Rust 有意不提供不受限制的运行时反射。需要类型图、属性编辑器、插件发现或动态分发的框架，往往只能解析源码、另外维护一份类型结构定义，或在类型擦除时丢失所有权和线程安全边界。`qubit-reflect` 将这些约定留在 Rust 声明中：生成代码只暴露 Rust 能够证明安全的操作；即使某个操作不可用，描述符仍会保留结构事实。

## 核心能力与边界

- 分别描述具体运行时类型与泛型源码定义，并描述 trait、impl 及支持的内置类型族。
- 受检字段读取、可变借用、字段替换、枚举分支判断和动态构造；执行前校验失败时，恢复对象会保留调用方传入并转移所有权的值。
- 为受支持的方法生成调用适配器，区分本地模式与显式请求的线程安全模式。
- 链接收集的注册片段与显式提供的注册片段使用同一套事务性校验，生成确定性的不可变注册表；它是解析具体类型与泛型定义有效能力的唯一公开入口，并提供类型安全的 `Clone`、`Default` 适配器。调用方也可以持有显式创建的不可变注册表快照。
- 严格的类型化能力查询保留四种状态：`Missing`、`FactOnly`、`AdapterTypeMismatch` 和 `Found`。需要追溯来源时，注册表还会报告有效能力来自 `Intrinsic` 还是 `Registered`，并可返回贡献该能力的 `FragmentIdentity`。
- 动态值明确区分 `Local` 与选择性启用的 `ThreadSafe` 边界；只有生成代码证明类型满足所需 `Send + Sync` 约束时，才会提供线程安全字段访问和构造。

反射能力有明确边界：不会转换数值、解析字符串、推导 `Into`，也不会把本地动态值升级为线程安全模式。`TypeId`、描述符地址和 trait 标记 仅表示进程内身份，不能作为序列化或跨进程模型 ID。被禁用或暂不支持的操作仍可通过描述符发现，并给出结构化的不可用原因。
元组支持 0 到 32 个元素，可移植函数指针支持 0 到 32 个参数；超出该数量范围时不受支持，也不会获得 `Reflect` 实现。

当库或测试需要自行指定注册内容时，可以使用 `RegistrySnapshotBuilder`。
[隔离快照](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md#构建隔离的-registry-snapshot)、
[能力冲突处理](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md#迁移-effective-capability-查询)和
[空结构体构造](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md#空结构体的构造方式)的操作步骤见用户指南。

## 延伸阅读

- [中文用户指南](doc/2026-08-29-qubit-reflect-user-guide.zh_CN.md)
- [English user guide](doc/2026-08-29-qubit-reflect-user-guide.md)
- [Rustdoc 源码中的 API 概览](src/lib.rs)；在仓库根目录运行
  `cargo doc --all-features --no-deps --open`，生成并打开完整参考文档
- [中文详细设计](doc/2026-09-03-qubit-reflect-design.zh_CN.md)
- [English design](doc/2026-09-03-qubit-reflect-design.md)
- [中文演进历史](doc/2026-09-07-qubit-reflect-evolution.zh_CN.md) · [Evolution history](doc/2026-09-07-qubit-reflect-evolution.md)
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
