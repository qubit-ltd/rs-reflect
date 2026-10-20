# 用户手册

本手册面向使用 vNext API 的 Rust 应用和框架开发者。我们以客服控制台修改客户资料为例：加载客户、更新字段、调用受支持的操作，再由应用保存结果。可运行起点是 [`examples/field_patch.rs`](../examples/field_patch.rs)；README 展示了同一流程的短代码片段。

## 在类型上启用反射

添加默认启用 `derive` feature 的 `qubit-reflect`，然后标注需要参与反射的声明。`#[reflect(skip)]` 会把成员从反射元数据中移除，但不会改变 Rust 声明本身。

```rust
use qubit_reflect::{Reflect, ReflectImpl};

#[Reflect]
struct Customer {
    pub email: String,
    pub credit_limit_cents: u64,
    #[reflect(skip)]
    internal_note: String,
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
```

宏在编译期生成源码元数据和操作适配器。反射采用显式启用：没有生成进元数据的成员无法通过描述符查到。`skip` 只决定元数据是否包含该成员，不承担访问控制职责。

## 反射 trait 实现

用 `#[Reflect]` 标记本地 trait。需要关联其反射定义并生成默认方法 adapter 的 impl，须在属性中显式给出 trait 定义路径：

```rust
use qubit_reflect::{Reflect, ReflectImpl};

#[Reflect]
trait HealthCheck {
    fn check(&self) -> bool;

    fn label(&self) -> &'static str {
        "ready"
    }
}

#[Reflect]
struct Worker;

use crate::HealthCheck as Check;

#[ReflectImpl(reflected_trait = crate::HealthCheck)]
impl Check for Worker {
    fn check(&self) -> bool {
        true
    }
}
```

`reflected_trait` 指向已反射的 trait 定义。`impl` 后的 trait 路径可以使用导入别名，仍由 Rust 按常规规则解析。任何裸 `#[ReflectImpl]` trait impl（包括实现本地 `#[Reflect]` trait 的 impl）都会保留源码路径，并将未关联的 application 记为 `ExternalIncomplete`；它不会关联反射定义或提供默认方法。只有 `reflected_trait = ...` 才会链接本地反射 trait provider。

`reflected_trait` 和 `external_trait` 是调用者对 impl trait 身份的断言。稳定过程宏不能解析导入别名来证明路径等价；请确保参数指向 impl 实际实现的 trait，否则反射信息会关联到错误的 trait。

## 读取、更新和调用

`TypeDescriptor::of` 会把描述符绑定到进程级全局 `ReflectionContext`。操作需要聚合信息时，context 会惰性初始化。字段单数 accessor 要求找到唯一字段，否则返回结构化错误。typed 访问保留所有者和字段的 Rust 类型信息。

```rust
use qubit_reflect::{Invocation, TypeDescriptor};

fn update(customer: &mut Customer) -> Result<(), Box<dyn std::error::Error>> {
    let ty = TypeDescriptor::of::<Customer>();
    let email = ty.field("email")?;
    if let Err(failure) = email.set(customer, String::from("ada@corp.example")) {
        let (error, _rejected_email) = failure.into_parts();
        return Err(format!("邮箱更新失败：{error:?}").into());
    }
    assert_eq!(email.get::<_, String>(customer)?, "ada@corp.example");

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

操作成功的信号是方法返回 `true`，且内存中的对象字段已更新。反射不会替应用保存对象。服务 handler 应先通过 repository 按客户 ID 加载对象，再执行反射操作和应用校验；只有校验成功后，才在应用自己的事务中保存。更新被拒绝时应保留或报告返回的输入，并由 handler 决定后续处理。若用户代码已经执行后才失败，应使用 repository 事务或显式补偿策略。

可运行示例有意停在存储之前。应用可以按以下顺序装配：按客户 ID 加载；调用 `update`；校验客户对象；成功后在事务中保存；失败时返回错误且不保存。Repository trait、数据库事务和鉴权规则都由应用定义。

## 处理缺失和歧义

业务要求必须唯一找到成员时，使用严格单数 accessor：

```rust
let method = ty.method("raise_credit_limit")?;
```

字段可能不存在时，用集合的可选查询 `get`；同名方法可能有多个候选项时，用 `lookup_method` 获取完整状态：

```rust
let email = ty.fields().get("email");
let matches = ty.lookup_method("reset")?;
match matches {
    qubit_reflect::Lookup::Missing => { /* 提示不支持该操作 */ }
    qubit_reflect::Lookup::Unique(method) => { /* 使用该方法 */ }
    qubit_reflect::Lookup::Ambiguous(methods) => { /* 要求调用方明确选择 */ }
}
```

`method()` 会通过 `LookupError` 报告缺失或歧义；`lookup_method()` 返回完整的 `Lookup` 状态，并把 context 初始化错误保留在外层结果中。按名称发现类型时，`ReflectionContext::type_by_name` 要求唯一，`lookup_type_by_name` 返回完整状态。诊断用的 Rust 类型名和 `TypeId` 都不应作为持久化标识。

## 恢复被拒绝的更新和调用

typed 字段替换失败时返回 `FieldSetFailure<V>`。调用 `into_parts()` 可以同时取回诊断信息和原替换值，便于 handler 保留或报告输入。方法开始执行前，调用校验可能因为 receiver 或参数的类型、传递模式、参数绑定或方法可用性而失败。此时 `InvocationFailure` 会保留执行前被拒绝的输入。

用户代码一旦开始执行，反射无法撤销它产生的副作用。如果方法先修改对象再返回业务错误，调用方应使用自身的事务或补偿策略。普通 `invoke()` 也不会捕获 panic；只有应用已经确定 panic 处理策略时，才选择显式的捕获入口。

## 在框架边界使用动态值

泛型应用代码仍知道 Rust 类型时，优先使用 typed 字段访问。框架收到类型擦除的对象或新值时，可使用 `DynamicOwned<Local>`、`DynamicRef<Local>` 或 `DynamicMut<Local>`。动态操作仍会在访问前检查目标和字段值的精确类型。downcast 失败会归还原值，字段替换失败也会保留替换输入。

当值和适配器满足相应 `Send`、`Sync` 约束时，可以使用 `ThreadSafe` 模式。类型擦除后不能把 `Local` 值升级为 `ThreadSafe`。应在已知所有权和并发要求的边界选择模式。

derive 为 `async fn` 生成的调用适配器目前仅支持 `Local`。稳定过程宏无法推断 opaque future 的 `ThreadSafe` adapter，因此即使某个实现的 future 满足 `Send`，也不会自动生成该 adapter。若要使用 `ThreadSafe` adapter，可用 `bind_method!(... as thread_safe async fn(...))` 显式绑定具体方法；编译器会检查生成的 future 是否满足 `Send`，无需命名 opaque future 类型。

## 绑定泛型方法

Rust 会在编译期生成泛型方法的具体代码。构建显式 context 时，使用 `bind_method!` 注册各个受支持的特化；运行时反射不会依据 `TypeId` 合成新的单态化代码。

```rust
use qubit_reflect::{bind_method, ReflectionContext};

let context = ReflectionContext::builder()
    .include_linked()
    .bind(bind_method!(Converter::convert::<u8> as fn(&Converter, String) -> u8))
    .build()?;
```

trait 方法绑定时使用显式身份提示。对本地反射 trait，`reflected_trait` 指向 trait 定义；binder 会从 impl 的 trait 路径调用隐藏 hook，因此 rustc 能解析导入 alias 并补齐默认类型和 const 实参：

```rust
.bind(bind_method!(
    <Service as ImportedService>::refresh as fn(&Service) -> String,
    reflected_trait = api::ServiceContract,
))
```

对外部 trait，先用 `reflect_external_trait!` 创建 identity handle，再以 `external_trait = Handle` 绑定显式方法。handle 不提供外部方法声明或默认参数信息；application identity 仅包含 impl 路径中明确写出的类型和 const 实参。此绑定是调用者登记的方法事实。

方法描述符仍表示源码声明。调用前使用 `MethodArguments` 选择具体特化。未绑定或不受支持的方法仍会以元数据和可用性原因呈现，不会被静默地绕过安全检查。

## 依赖功能

默认 feature 集包含 `derive`。仅需运行时 API 的 crate 可以关闭默认功能。`ecosystem-types` 和 `qubit-types` 为可选外部类型补充元数据；应用只需启用实际使用的集成。

```toml
[dependencies]
qubit-reflect = { version = "0.2", default-features = false }
qubit-reflect-derive = "0.2"
```

直接使用 derive crate 时，要让它和匹配版本的 runtime 配套。大多数应用直接使用默认启用 `derive` 的 `qubit-reflect` 即可。

## 延伸阅读

- [架构说明](architecture.zh_CN.md)介绍 context 构建和生成元数据的边界。
- [英文核心 API 契约摘要](core_api_contract.md)概述 context、查找、错误恢复和 capability 保证。
- [API 文档](https://docs.rs/qubit-reflect)列出公开类型和方法。
- [English user guide](user_guide.md)按相同的客服工作流说明操作步骤。
