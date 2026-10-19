# Qubit Reflect（`rs-reflect`）

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-reflect` 为 Rust 声明提供按需启用的反射能力。在结构体、枚举或 trait 上使用 `#[Reflect]`，在实现块上使用 `#[ReflectImpl]`，宏便会生成元数据和经过类型检查的操作适配器。应用可以据此检查字段与 variant、构造值，并按名称调用受支持的方法。

## 什么时候需要反射

通用编辑器、序列化器或客服工具可以用同一套逻辑处理多种反射类型，不必另维护一张容易与 Rust 声明脱节的字段分发表。输入解析、鉴权、业务校验和持久化仍由应用负责。描述符会保留所属的 `ReflectionContext`，因此方法查找和调用始终使用同一个视图。

## 安装

```toml
[dependencies]
qubit-reflect = "0.2"
```

`derive` feature 默认启用。仅需运行时功能或要为第三方类型接入反射时，请参阅[依赖功能](doc/user_guide.zh_CN.md#依赖功能)。

## 快速开始

下面修改客户邮箱，并调用一个反射方法。可运行版本见 [`field_patch`](examples/field_patch.rs)。

```rust
use qubit_reflect::{Invocation, Reflect, ReflectImpl, TypeDescriptor};

#[Reflect]
struct Customer {
    pub email: String,
    pub credit_limit_cents: u64,
}

#[ReflectImpl]
impl Customer {
    pub fn raise_credit_limit(&mut self, amount: u64) -> bool {
        if amount <= self.credit_limit_cents {
            return false;
        }
        self.credit_limit_cents = amount;
        true
    }
}

fn update(customer: &mut Customer) -> Result<(), Box<dyn std::error::Error>> {
    let ty = TypeDescriptor::of::<Customer>();
    let email = ty.field("email")?;
    if let Err(failure) = email.set(customer, String::from("ada@corp.example")) {
        let (error, _rejected_value) = failure.into_parts();
        return Err(format!("邮箱更新失败：{error:?}").into());
    }
    let output = match ty
        .method("raise_credit_limit")?
        .invoke(Invocation::on_mut(customer).arg(75_000_u64))
    {
        Ok(output) => output,
        Err(failure) => {
            let (error, _recovery) = failure.into_parts();
            return Err(format!("方法调用失败：{error:?}").into());
        }
    };
    let raised = output.into_owned::<bool>()?;
    assert!(raised);
    Ok(())
}
```

`field()` 和 `method()` 要求恰好找到一个成员；查找失败时会返回结构化错误。动态操作会检查精确 Rust 类型。泛型方法需要通过 `bind_method!` 在编译期绑定具体实例，运行时的 `TypeId` 不能生成新的 Rust 单态化代码。

## 使用边界

- `ReflectionContext::global()` 提供进程级视图；需要隔离或自定义注册时，可以使用显式 context。
- 描述符携带所属 context，访问成员或调用方法时不必再次传 registry。
- `#[reflect(skip)]` 会从反射元数据中移除声明，但不影响普通 Rust 代码访问它。
- 反射不负责鉴权、解析外部输入、持久化对象，也不会回滚应用副作用。
- 动态值默认使用 `Local` 模式；当值满足相应 Rust 约束时，也可以使用 `ThreadSafe`。

## 延伸阅读

- [中文用户手册](doc/user_guide.zh_CN.md) · [English user guide](doc/user_guide.md)
- [中文架构](doc/architecture.zh_CN.md) · [Architecture](doc/architecture.md)
- [API 文档](https://docs.rs/qubit-reflect)
- [English README](README.md)

## 开发检查

从仓库根目录运行 `./.infra/bin/align-ci.sh` 对齐格式和 CI 配置，再运行 `./.infra/bin/ci-check.sh` 执行项目检查。

## 许可证

本项目基于 Apache License 2.0 授权，详见 [LICENSE](LICENSE)。
