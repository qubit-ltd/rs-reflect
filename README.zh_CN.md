# Qubit Reflect（`rs-reflect`）

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

<!-- reflect-contract: runtime.direct=qubit_reflect -->
<!-- reflect-contract: examples.native=cargo-example -->

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


完整的加载–补丁–保存流程见 [`examples/customer_patch.rs`](examples/customer_patch.rs)；按名称调用业务方法见 [`examples/support_action.rs`](examples/support_action.rs)。这些是运行时包的原生 example。宏会直接解析 `qubit-reflect` 运行时，同包 example 也无须额外指定 crate 路径。

可选的 `qubit-datatype` 与 `qubit-id` 依赖从 crates.io 解析，因此源码检出无需准备相邻仓库。在仓库根目录确认依赖解析：

```bash
cargo metadata --locked --format-version 1
```

`cargo metadata` 成功即表示锁定的依赖图已解析。然后从源码检出根目录运行示例：

```bash
cargo run --example field_patch
cargo run --example customer_patch
cargo run --example support_action
```

三个目标均声明 `required-features = ["derive"]`，仅运行时构建会跳过它们。运行时 crate 的分发包包含这些源码，启用 `derive` 后也可在解包目录运行相同命令。上面的版本依赖用于选择已发布版本；源码检出的运行结果不能证明当前检出已发布或可从 registry 安装。

完整的加载–补丁–保存服务流程放在[客服控制台用户指南](doc/user_guide.zh_CN.md#接入客服控制台)中，其中说明 `CustomerRepository` 如何连接应用存储，以及如何处理被拒绝的变更。[`customer_patch` example](examples/customer_patch.rs) 提供可运行示例。

把请求解码为字段的 Rust 类型、判断调用者是否有权编辑记录，属于 API 层职责；`apply_patch` 只核对字段名、访问策略与精确类型。某条变更被拒绝时，前面的变更可能已写入内存中的结构体，但尚未调用 `save`，调用方丢弃已加载对象即可。完整错误模型见[用户手册](doc/user_guide.zh_CN.md#错误诊断与排障)。

### 反射方法与实现块

上面的快速开始展示了 inherent impl。描述 trait 声明时使用 `#[Reflect]`，
其实现仍由 `#[ReflectImpl]` 标注。[`examples/support_action.rs`](examples/support_action.rs)
提供另一份可运行的方法调用场景；[方法调用指南](doc/user_guide.zh_CN.md#按名称调用业务方法)
进一步解释查找结果、调用错误和输入恢复。

## 能力与边界

- `#[Reflect]` 与 `#[ReflectImpl]` 在声明处为结构体、枚举、trait 和 impl 生成不可变描述符，可在稳定版 Rust 上使用。
- `TypeDescriptor` 与 `FieldDescriptor` 通过普通 `&T`、`&mut T` 和拥有所有权的值提供受检字段读取、可变借用和替换；构造 builder 同样接收普通值。
- `#[reflect(skip)]` 会将字段、分支或方法完全排除在反射元数据之外。它不会删除 Rust 成员，也不能用来保密。其他成员属性不改变可见性；安全入口拒绝非公开成员，满足安全前提时可使用文档说明的 `_unsafe` 入口绕过可见性。
- 生成代码执行前失败会返回结构化错误，并退回调用方尚未被消费的输入：`FieldSetFailure`、`ConstructionRecovery`、`InvocationRecovery`。named 与 tuple 构造没有可用适配器时也会通过 `ConstructionRecovery` 退回输入，错误为 `TargetUnavailable`。
- `ReflectRegistry::initialize()` 把已链接的声明汇总成一份确定性的不可变注册表；`RegistrySnapshotBuilder` 让库和测试用显式事实构建隔离快照。两者都能解析方法以及 `Clone`、`Default` 等类型化能力，并区分能力处于 `Missing`、`FactOnly`、`AdapterTypeMismatch` 还是 `Found` 状态。
- 泛型声明保留为定义。若要动态调用泛型方法，须在编译期已知的调用点使用 `bind_method!` 绑定具体实例；运行时 `TypeId` 不能生成新的单态化代码。未绑定的泛型方法没有可调用适配器。
- 需要在线程间共享反射对象时，由调用方自行加锁。跨线程传值仍受 Rust 的 `Send` 和 `Sync` 规则约束。
- 内置基础类型、文本、元组、数组、`Option`、序列、集合、映射、智能指针和函数指针的描述符；可选的 `ecosystem-types` 与 `qubit-types` feature 分别补充 `BigDecimal`、`chrono`、`Uuid` 以及 Qubit `Id`/`DataType` 的实现。

`Option<T>` 描述符既能返回元素类型，也能对内置 `Option<T>` 值执行经过类型检查的借用投影。`Some` 返回内部借用，`None` 返回空值，不相关的值返回类型不匹配错误。由结构事实构造的描述符保留相同的类型形状，但因没有运行时适配器而报告投影不可用。详见[可选值](doc/user_guide.zh_CN.md#可选值)。

反射能力有明确边界：不转换数值、不解析字符串，也不推导 `Into`。字段替换失败时，`FieldSetFailure<V>` 保留未消费的原始输入；构造与调用在执行前失败时也会保留输入。动态输入中的 `Box<dyn Any>` 按盒内实际类型检查；若字段本身的类型就是 `Box<dyn Any>`，则仍按该字段的精确类型处理。任意 Rust 类型不会自动可反射，必须实现 `Reflect`。`TypeId`、描述符地址和 trait 标记只是进程内身份，不能用作序列化或跨进程的模型标识。反射不能替代应用鉴权。`unsafe` 函数、不支持的 ABI、可变参数、未绑定的泛型和不透明的 `impl Trait` 返回值可以被描述，但不能动态调用。元组和函数指针描述符支持 0 到 32 个元素或参数。描述符和按类型缓存的能力会在整个进程生命周期内保留，初始化可能分配内存；关心开销时请按应用实际路径测量。详情见[用户手册](doc/user_guide.zh_CN.md#边界与实践清单)。

## 延伸阅读

- [架构说明](doc/architecture.zh_CN.md) · [English architecture](doc/architecture.md)
- [用户手册](doc/user_guide.zh_CN.md)
- [API 文档](https://docs.rs/qubit-reflect)
- [English README](README.md) · [English user guide](doc/user_guide.md)

## 测试

以下命令在源码检出根目录运行。Cargo 会从 crates.io 解析可选的 `qubit-datatype` 与 `qubit-id` 依赖，无需准备相邻仓库；如需先确认依赖解析，请运行上文的 metadata 命令。

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
