# Qubit Reflect（`rs-reflect`）

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-reflect` 为 Rust 提供按需启用、基于宏的反射能力。在结构体、枚举或 trait 上使用 `#[Reflect]`，在 inherent 或 trait 实现块上使用 `#[ReflectImpl]`，宏就会生成结构元数据及经过类型检查的运行时操作入口。应用与框架可以据此发现类型和成员、访问字段、构造值，并调用受支持的方法，无须为每个类型另写一套容易过时的描述代码。该机制可在稳定版 Rust 上使用，动态操作仍遵守 Rust 的所有权和类型约束。

## 为什么需要反射

普通 Rust 类型信息无法在运行时枚举应用结构体的字段，也不能按名称调用方法。需要这些能力的框架常另写字段名、类型和访问函数表；类型定义一变，这张表就可能遗漏更新。`qubit-reflect` 把反射约定放回声明处：宏在编译期生成描述符和操作适配器，运行时通过 `TypeDescriptor` 和注册表查询。

例如，通用记录编辑器可以按名称找到 `Customer.email`，检查类型并替换其值；另一种记录只要实现 `Reflect`，就能沿用同一套处理逻辑。输入解析、鉴权、业务校验和持久化仍由应用负责。[用户手册](doc/user_guide.zh_CN.md)以加载、修改、保存客户记录为例，展示完整接入过程。

## 安装

```toml
[dependencies]
qubit-reflect = "0.2"
```

反射宏默认启用。每个派生反射的 crate 都要直接声明 `qubit-reflect` 依赖。仅使用运行时 API，或为第三方类型提供反射实现时，参阅用户手册的[依赖功能选择](doc/user_guide.zh_CN.md#选择依赖功能)。

## 快速开始

下面在类型上使用 `#[Reflect]`，在方法实现上使用 `#[ReflectImpl]`。同一个
示例先通过类型描述符定位字段，读取并替换精确类型的值，再从注册表按名称
找到并调用 `raise_credit_limit`。安全入口拒绝私有 `secret` 字段；示例明确
给出 safety 理由后使用文档规定的 `_unsafe` 入口。`internal_note` 保留在
Rust 结构体中，但被 skip 后不会进入反射元数据。

<!-- reflect-source: examples/field_patch.rs -->
```rust
use std::error::Error;
use std::io;

use qubit_reflect::Invocation;
use qubit_reflect::InvocationOutput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectRegistry;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::descriptor::MethodLookup;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::Local;

#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub credit_limit_cents: u64,
    secret: String,
    #[reflect(skip)]
    internal_note: String,
}

#[ReflectImpl]
impl Customer {
    /// Raises the credit limit, returning false when the requested limit is not
    /// greater than the current one.
    pub fn raise_credit_limit(&mut self, credit_limit_cents: u64) -> bool {
        if credit_limit_cents <= self.credit_limit_cents {
            return false;
        }
        self.credit_limit_cents = credit_limit_cents;
        true
    }
}

/// Runs the example, returning reflection setup and operation failures to the
/// caller.
fn main() -> Result<(), Box<dyn Error>> {
    let registry = ReflectRegistry::initialize()?;
    let descriptor = TypeDescriptor::of::<Customer>();
    let mut customer = Customer {
        id: 1001,
        email: String::from("ada@example.com"),
        credit_limit_cents: 50_000,
        secret: String::from("private"),
        internal_note: String::from("not reflected"),
    };

    let email_field = descriptor
        .field("email")
        .ok_or(io::Error::other("missing reflected email field"))?;
    email_field.set(&mut customer, String::from("ada@corp.example"))?;
    assert_eq!(customer.email, "ada@corp.example");

    let current: &String = email_field.get(&customer)?;
    assert_eq!(current, "ada@corp.example");

    let secret_field = descriptor
        .field("secret")
        .ok_or(io::Error::other("missing reflected secret field"))?;
    assert!(matches!(
        secret_field.get::<_, String>(&customer),
        Err(FieldAccessError::NotPublic { .. })
    ));
    // SAFETY: this example owns Customer and intentionally demonstrates the
    // visibility bypass.
    assert_eq!(unsafe { secret_field.get_unsafe::<_, String>(&customer) }?, "private");
    // SAFETY: this example owns Customer and maintains the String field's type
    // invariant.
    unsafe { secret_field.set_unsafe(&mut customer, String::from("updated")) }?;
    assert_eq!(customer.secret, "updated");
    assert!(descriptor.field("internal_note").is_none());
    assert_eq!(customer.internal_note, "not reflected");

    let MethodLookup::Unique(raise_credit_limit) = descriptor.methods_named_in(registry, "raise_credit_limit") else {
        return Err(io::Error::other("expected exactly one method named raise_credit_limit").into());
    };
    {
        let invocation = Invocation::receiver_mut(&mut customer).arg(75_000_u64);
        let output = raise_credit_limit
            .invoke(registry, invocation)
            .map_err(|failure| io::Error::other(failure.to_string()))?;
        let InvocationOutput::Owned(changed) = output else {
            return Err(io::Error::other("raise_credit_limit did not return an owned value").into());
        };
        let changed = DynamicOwned::<Local>::downcast::<bool>(changed)
            .map_err(|_| io::Error::other("raise_credit_limit did not return bool"))?;
        assert!(changed);
    }
    assert_eq!(customer.credit_limit_cents, 75_000);
    Ok(())
}
```

这段代码按顺序完成了一个反射操作闭环：取得 `Customer` 的类型描述符，按名称
找到字段，写入并读回邮箱，确认安全入口会拒绝私有字段、被 skip 的字段不会出现在
元数据中，最后按名称找到并调用方法。所有断言通过且程序正常退出，就是示例的成功
结果。

反射在这里负责发现成员，并在动态操作前检查可见性和精确 Rust 类型；请求解析、
调用者鉴权、业务校验和持久化仍由应用负责。需要把字段修改接入加载—修改—保存流程
时，可继续阅读可运行的 [`customer_patch` 示例](examples/customer_patch.rs)及
[客服控制台接入指南](doc/user_guide.zh_CN.md#接入客服控制台)。

## 核心能力与使用边界

`qubit-reflect` 提供以下核心能力：

- `#[Reflect]` 为结构体、枚举和 trait 生成结构描述；`#[ReflectImpl]` 为 inherent impl 与 trait impl 生成实现和方法描述。
- `TypeDescriptor` 与 `FieldDescriptor` 支持经过类型检查的字段读取、可变借用和替换；构造 API 可以从普通 Rust 值创建或更新对象。
- `ReflectRegistry` 汇总已链接的类型、实现和方法，使应用可以按运行时信息查找成员、调用受支持的方法，并查询 `Clone`、`Default` 等类型化能力。
- 内置基础类型、文本、元组、数组、`Option`、序列、集合、映射、智能指针和函数指针的描述符；`ecosystem-types` 与 `qubit-types` feature 可补充常用生态类型和 Qubit 类型。

使用时需要明确这些边界：

- 动态操作只接受精确类型；库不会解析字符串、转换数值或推导 `Into`。
- 反射不替代鉴权、业务校验、数据库事务或线程同步，这些策略仍由应用控制。
- `#[reflect(skip)]` 只把成员排除在反射元数据之外，不是保密机制；安全 API 会拒绝非公开成员，只有满足文档中的安全前提才能使用 `_unsafe` API 绕过可见性检查。
- 字段设置、构造或调用若在执行前被拒绝，会通过结构化错误保留尚未消费的输入；适配器开始执行后，库不承诺自动回滚业务状态。
- 运行时不能凭 `TypeId` 生成新的泛型单态化代码。动态调用泛型方法前，必须用 `bind_method!` 在编译期绑定具体实例；`unsafe` 函数、不支持的 ABI、可变参数和不透明的 `impl Trait` 返回值只能描述，不能动态调用。
- `TypeId`、描述符地址和 trait 标记只在当前进程内有意义，不能当作序列化格式或跨进程模型标识；描述符及按类型缓存的能力会在进程生命周期内保留。

构造、方法调用、`Option` 投影、错误恢复和注册表的完整说明见
[用户手册](doc/user_guide.zh_CN.md)。

## 延伸阅读

- [架构说明](doc/architecture.zh_CN.md) · [English architecture](doc/architecture.md)
- [用户手册](doc/user_guide.zh_CN.md)
- [API 文档](https://docs.rs/qubit-reflect)
- [English README](README.md) · [English user guide](doc/user_guide.md)

## 测试

以下命令在源码检出根目录运行：

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
