# Qubit Reflect（`rs-reflect`）

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

<!-- reflect-contract: facade.explicit=qubit_reflect -->
<!-- reflect-contract: examples.native=cargo-example -->

`qubit-reflect` 帮助 Rust 后台服务用一套通用逻辑处理控制台发来的字段级 `PATCH`：请求只携带字段名（如 `email`、`credit_limit_cents`）和已由 API 层解码好的值，同一套界面还要能编辑客户、订单以及以后新增的记录类型。若每种类型各写一段 `match field_name { ... }`，分支很快就会和结构体定义脱节。本库让每种记录在声明处派生描述符，通用处理函数即可按名称查找字段、核对精确 Rust 类型与访问策略，再对应用仍持有的结构体读取或替换。反射代码由宏在稳定版 Rust 上生成。本库不负责把请求文本解析成 Rust 值，不提供序列化，也不替代业务校验与鉴权。

## 客服控制台实战场景

客户服务从仓储加载 `Customer { id, email, display_name, credit_limit_cents }`。API 层已把请求解码为带类型的变更，例如 `("email", String)`。通用 `apply_patch` 在 `TypeDescriptor::of::<Customer>()` 上按名称定位字段并调用 `set`。`id` 标注了 `#[reflect(read_only)]`，改主键会在触碰结构体前被拒绝；类型不匹配时同样拒绝，未消费的值会退回调用方。全部变更应用成功后，服务才保存客户。以后要让同一控制台编辑 `Order`，只需为 `Order` 派生 `Reflect`，`apply_patch` 与 HTTP 处理代码无须改动。

## 安装

```toml
[dependencies]
qubit-reflect = "0.1"
```

反射宏默认启用。仅使用运行时 API，或为第三方类型提供反射实现时，参阅用户手册的[依赖功能选择](doc/user_guide.zh_CN.md#选择依赖功能)。

## 快速开始

<!-- reflect-source: examples/field_patch.rs -->
```rust
use qubit_reflect::FieldAccessError;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedMut;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::ReflectedRef;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct Customer {
    #[reflect(read_only)]
    id: u64,
    email: String,
    credit_limit_cents: u64,
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
    };

    let email = descriptor.field("email").expect("derived field");
    email
        .set(
            ReflectedMut::new(&mut customer),
            ReflectedOwned::new(String::from("ada@corp.example")),
        )
        .expect("exactly typed replacement");
    assert_eq!(customer.email, "ada@corp.example");

    let current = email.get(ReflectedRef::new(&customer)).expect("shared read");
    assert_eq!(
        current.downcast_ref::<String>().map(String::as_str),
        Some("ada@corp.example")
    );

    let id = descriptor.field("id").expect("derived field");
    let failure = id
        .set(ReflectedMut::new(&mut customer), ReflectedOwned::new(2002_u64))
        .expect_err("read-only field rejects replacement");
    assert!(matches!(failure.error(), FieldAccessError::ReadOnly { .. }));
    assert_eq!(customer.id, 1001);
}
```


完整的加载–补丁–保存流程见 [`examples/customer_patch.rs`](examples/customer_patch.rs)；按名称调用业务方法见 [`examples/support_action.rs`](examples/support_action.rs)。这些是运行时包的原生 example。同包 example 中，自动发现的 `crate` 会指向示例可执行程序，因此声明用 `#[reflect(crate = qubit_reflect)]` 和 `#[reflect_impl(crate = qubit_reflect)]` 显式指定运行时外观库。

源码检出中，即使 `qubit-types` 被禁用，Cargo 仍会解析可选的 `qubit-datatype` 与 `qubit-id` path manifest。先准备这两个相邻检出，再确认 Cargo 能读取两个 manifest：

```bash
./.infra/bin/prepare-local-path-dependencies.sh
cargo metadata --locked --format-version 1
```

`cargo metadata` 成功即表示路径依赖已解析。使用已发布 crate 的用户不需要运行本仓库脚本。然后从源码检出根目录运行示例：

```bash
cargo run --example field_patch
cargo run --example customer_patch
cargo run --example support_action
```

三个目标均声明 `required-features = ["derive"]`，仅运行时构建会跳过它们。运行时 crate 的分发包包含这些源码，启用 `derive` 后也可在解包目录运行相同命令。上面的版本依赖用于选择已发布版本；源码检出的运行结果不能证明当前检出已发布或可从 registry 安装。

完整的加载–补丁–保存服务流程放在[客服控制台用户指南](doc/user_guide.zh_CN.md#接入客服控制台)中，其中说明 `CustomerRepository` 如何连接应用存储，以及如何处理被拒绝的变更。[`customer_patch` example](examples/customer_patch.rs) 提供可运行示例。

把请求解码为字段的 Rust 类型、判断调用者是否有权编辑记录，属于 API 层职责；`apply_patch` 只核对字段名、访问策略与精确类型。某条变更被拒绝时，前面的变更可能已写入内存中的结构体，但尚未调用 `save`，调用方丢弃已加载对象即可。完整错误模型见[用户手册](doc/user_guide.zh_CN.md#错误诊断与排障)。

### 启动时登记可反射的业务操作

同一控制台还提供「冻结客户」等操作按钮：配置表存方法名，后端在已加载的记录上按名称调用。给 `impl` 块加上 `#[reflect_impl]`，在应用启动时调用 `ReflectRegistry::initialize()` 一次，后续通过 `TypeDescriptor::methods_named_in` 与 `invoke_local` 解析并执行操作。完整可运行代码见 [`examples/support_action.rs`](examples/support_action.rs)。`methods_named_in` 可能返回 `Missing`、`Unique` 或 `Ambiguous`，应用需分别处理。`invoke_local` 的外层 `Err(InvocationUnavailable)` 表示入口不可用，并保留完整原始输入；`Ok(Err(InvocationFailure))` 表示接收者或参数在方法体执行前未通过校验。普通调用会传播方法体内的 panic；捕获既要求方法标注 `#[reflect(catch_unwind)]`，也要求调用方选择可用的 catching 入口。可运行示例见 [`examples/support_action.rs`](examples/support_action.rs)；注册表初始化与方法查找见[用户手册](doc/user_guide.zh_CN.md#按名称调用业务方法)。

## 能力与边界

- `#[derive(Reflect)]`、`#[reflect]`、`#[reflect_impl]` 在声明处为结构体、枚举、trait 和 impl 生成不可变描述符，可在稳定版 Rust 上使用。
- `TypeDescriptor` 与 `FieldDescriptor` 通过 `ReflectedRef`、`ReflectedMut`、`ReflectedOwned` 提供受检的字段读取、可变借用和替换；`construct_struct`、`construct_tuple`、`construct_unit` 用命名或位置输入构造新值。
- 字段属性 `rename`、`read_only`、`skip`、`no_construct`、`opaque` 和方法属性 `no_invoke` 只限制对应的动态操作，结构信息仍然可见；普通 `#[cfg]` 则会让成员整体消失。
- 生成代码执行前的校验失败会返回结构化错误，并把调用方尚未被消费的输入一并退回：`FieldSetFailure`、`ConstructionRecovery`、`InvocationRecovery`。
- `ReflectRegistry::initialize()` 把已链接的声明汇总成一份确定性的不可变注册表；`RegistrySnapshotBuilder` 让库和测试用显式事实构建隔离快照。两者都能解析方法以及 `Clone`、`Default` 等类型化能力，并区分能力处于 `Missing`、`FactOnly`、`AdapterTypeMismatch` 还是 `Found` 状态。
- 泛型声明以定义的形式被描述；`#[reflect(specialize(...))]` 可为有限的具体实例生成调用入口。
- 动态值默认是 `Local` 模式；`SendReflected*` 包装器和 `#[reflect(thread_safe)]` 只在生成代码能证明所需 `Send + Sync` 约束时提供线程安全边界。
- 内置基础类型、文本、元组、数组、`Option`、序列、集合、映射、智能指针和函数指针的描述符；可选的 `ecosystem-types` 与 `qubit-types` feature 分别补充 `BigDecimal`、`chrono`、`Uuid` 以及 Qubit `Id`/`DataType` 的实现。

`Option<T>` 描述符既能返回元素类型，也能对内置 `Option<T>` 值执行经过类型检查的借用投影。`Some` 返回内部借用，`None` 返回空值，不相关的值返回类型不匹配错误。由结构事实构造的描述符保留相同的类型形状，但因没有运行时适配器而报告投影不可用。详见[可选值](doc/user_guide.zh_CN.md#可选值)。

反射能力有明确边界：不转换数值、不解析字符串、不推导 `Into`，也不会把本地动态值升级为线程安全模式。任意 Rust 类型不会自动可反射，必须派生或实现 `Reflect`。`TypeId`、描述符地址和 trait 标记只是进程内身份，不能用作序列化或跨进程的模型标识。生成的访问代码可以触及私有字段，因此反射策略不能替代应用鉴权。`unsafe` 函数、不支持的 ABI、可变参数、未特化的泛型和不透明的 `impl Trait` 返回值可以被描述，但不能动态调用。元组和函数指针描述符支持 0 到 32 个元素或参数。描述符和按类型缓存的能力会在整个进程生命周期内保留，初始化可能分配内存；关心开销时请按应用实际路径测量。详情见[用户手册](doc/user_guide.zh_CN.md#边界与实践清单)。

## 延伸阅读

- [用户手册](doc/user_guide.zh_CN.md)
- [架构与详细设计](doc/2026-09-03-qubit-reflect-design.zh_CN.md) · [derive 契约矩阵](doc/derive-contract-matrix.zh_CN.md)
- [API 文档](https://docs.rs/qubit-reflect)
- [English README](README.md) · [English user guide](doc/user_guide.md)

## 测试

以下命令在源码检出根目录运行。加载 Cargo workspace 时，即使未启用相关 feature，也会解析可选的
`qubit-datatype` 与 `qubit-id` 路径清单；请先按上文准备本地路径依赖。通过 registry 使用已发布 crate
的用户无需运行该脚本。

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
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./.infra/bin/align-ci.sh` 格式化代码，运行 `./.infra/bin/ci-check.sh` 对齐 CI 要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
