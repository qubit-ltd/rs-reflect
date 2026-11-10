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

反射的 struct 或 enum 含有不满足 `Send + Sync` 的字段时，可使用
`#[Reflect(local_only)]` 保留本地字段访问与构造能力。裸 `#[Reflect]` 的
行为保持不变；该选项不会让值变为线程安全，也不改变 `#[ReflectImpl]` 注册。
只有需要从反射中完全排除成员时才使用 `#[reflect(skip)]`。

显式 context 会选择自己的目标类型和绑定目标。若代码此前依赖无关 linked
registration 意外出现在 context 中，现在应显式选择目标。context 构建也会报告
非法 linked capability，因此应处理返回的 `ContextError`。Rc 和 Arc 描述符
现在包含 pointee 类型实参；这项元数据增量不要求下游生产代码修改。

调用带 pinned receiver 的泛型方法时，先使用
`method.specialization(&MethodArguments)` 选择精确特化，再通过
`MethodSpecializationDescriptor::invoke_pinned` 调用。调用被拒绝时会返回
pinned receiver 和参数以供恢复。若选中的绑定提供了 `T` 对应的具体参数类型，
pinned 调用会在执行 adapter 前按该类型校验运行时参数。特化必须与绑定匹配；
校验拒绝的参数仍可取回并恢复。

宏在编译期生成源码元数据和操作适配器。反射采用显式启用：没有生成进元数据的成员无法通过描述符查到。`skip` 只决定元数据是否包含该成员，不承担访问控制职责。

## 集成下游 derive

下游 derive crate 可使用带版本的 `qubit_reflect::__private::codegen_v4` 集成协议，但必须协调 runtime 与生成器版本。受支持的表达式符号清单见[架构文档中的协议章节](architecture.zh_CN.md#生成代码集成协议)；该清单依据内置 derive 生成及已核查的下游生成器，包括 `rs-model-metadata` 和 `rs-model-metadata-mm-implementation`。兼容性检查使用 `PROTOCOL_VERSION` 与 `protocol::assert_protocol_version`。这不是应用 API，也不表示所有 `__private` 项目都受支持。runtime 与生成器版本应保持一致。不兼容的协议变更须使用新命名空间，并协同升级 runtime、derive 和受影响的下游生成器；当前 v4 命名空间继续保留。

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

复合类型描述符会展示完整诊断名称。为兼容已有行为，context 短名称查找仍使用末段别名；完整展示字符串不是查找键。Rust 诊断名称并不稳定，需要持久化时应使用应用自定义标识。

## 读取、更新和调用

`TypeDescriptor::of` 会把描述符绑定到进程级全局 `ReflectionContext`。操作需要聚合信息时，context 会惰性初始化。字段单数 accessor 要求找到唯一字段，否则返回结构化错误。typed 访问保留所有者和字段的 Rust 类型信息。

```rust
use qubit_reflect::{Invocation, TypeDescriptor};
use qubit_reflect::value::Local;

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
        .invoke(Invocation::<Local>::on_mut(customer).arg(75_000_u64))
    {
        Ok(output) => output,
        Err(failure) => {
            let (error, _recovery) = failure.into_parts();
            return Err(format!("方法调用失败：{error:?}").into());
        }
    };
    let raised = output
        .into_owned::<bool>()
        .map_err(|_| std::io::Error::other("方法返回了非布尔值"))?;
    assert!(raised);
    Ok(())
}
```

操作成功的信号是方法返回 `true`，且内存中的对象字段已更新。反射不会替应用保存对象。服务 handler 应先通过 repository 按客户 ID 加载对象，再执行反射操作和应用校验；只有校验成功后，才在应用自己的事务中保存。更新被拒绝时应保留或报告返回的输入，并由 handler 决定后续处理。若用户代码已经执行后才失败，应使用 repository 事务或显式补偿策略。

可运行示例有意停在存储之前。应用可以按以下顺序装配：按客户 ID 加载；调用 `update`；校验客户对象；成功后在事务中保存；失败时返回错误且不保存。Repository trait、数据库事务和鉴权规则都由应用定义。

## 处理缺失和歧义

业务要求必须唯一找到成员时，使用严格单数 accessor。字段可能不存在时，用集合的可选查询 `get`；同名方法可能有多个候选项时，用 `lookup_method` 获取完整状态：

```rust
fn lookup_demo(ty: qubit_reflect::TypeDescriptor<'_>) {
    let email = ty.fields().get("email");
    assert!(email.is_some());
    match ty.lookup_method("reset").expect("查询操作") {
        qubit_reflect::lookup::Lookup::Missing => {}
        qubit_reflect::lookup::Lookup::Unique(_) => panic!("操作不应存在"),
        qubit_reflect::lookup::Lookup::Ambiguous(_) => panic!("操作不应歧义"),
    }
    match ty.lookup_method("raise_credit_limit").expect("查询方法") {
        qubit_reflect::lookup::Lookup::Missing => panic!("方法已注册"),
        qubit_reflect::lookup::Lookup::Unique(method) => {
            let mut customer = Customer { email: String::from("ada@corp.example"), credit_limit_cents: 10_000, internal_note: String::new() };
            let result = method.invoke(qubit_reflect::Invocation::<qubit_reflect::value::Local>::on_mut(&mut customer).arg(25_000_u64))
                .unwrap_or_else(|_| panic!("调用方法"));
            assert!(result.into_owned::<bool>().unwrap_or_else(|_| panic!("布尔结果")));
            assert_eq!(customer.credit_limit_cents, 25_000);
        }
        qubit_reflect::lookup::Lookup::Ambiguous(_) => panic!("方法只有一个候选项"),
    }
}
```

`method()` 会通过 `LookupError` 报告缺失或歧义；`lookup_method()` 返回完整的 `Lookup` 状态，并把 context 初始化错误保留在外层结果中。按名称发现类型时，`ReflectionContext::type_by_name` 要求唯一，`lookup_type_by_name` 返回完整状态。诊断用的 Rust 类型名和 `TypeId` 都不应作为持久化标识。

## 构造客户

如果调用方可以构造某个类型，应为它使用字段公开且参与反射的独立类型。安全构造要求每个字段都有值，并拒绝 Rust 类型不匹配的值；`into_input()` 会归还被拒绝的命名输入，供修正或报告。示例不包含 skipped 或私有字段，也不使用 unchecked 构造。

```rust
#[derive(Debug, PartialEq)]
#[qubit_reflect::Reflect]
struct NewCustomer {
    pub email: String,
    pub credit_limit_cents: u64,
}

fn construct_demo() {
    use qubit_reflect::construct::{NamedFields, RecoveredConstructionInput};
    use qubit_reflect::value::Local;

    let ty = qubit_reflect::TypeDescriptor::of::<NewCustomer>();
    let customer = ty.construct::<NewCustomer, _>(
        NamedFields::<Local>::new()
            .field("email", String::from("ada@corp.example"))
            .field("credit_limit_cents", 10_000_u64),
    ).expect("构造客户");
    assert_eq!(customer.credit_limit_cents, 10_000);

    let missing = ty.construct::<NewCustomer, _>(
        NamedFields::<Local>::new().field("email", String::from("ada@corp.example")),
    ).expect_err("拒绝缺失字段");
    assert!(matches!(missing.into_input(), RecoveredConstructionInput::Named(_)));

    let wrong_type = ty.construct::<NewCustomer, _>(
        NamedFields::<Local>::new()
            .field("email", String::from("ada@corp.example"))
            .field("credit_limit_cents", String::from("类型错误")),
    ).expect_err("拒绝错误字段类型");
    assert!(matches!(wrong_type.into_input(), RecoveredConstructionInput::Named(_)));
}
```

返回的新值表示构造成功。构造不会替应用校验鉴权、授信规则等业务约束；保存之前仍应执行应用校验。

## 更新客户

`UpdateInput` 将多个替换值与一个拥有所有权的基值组合起来。更新被拒绝时会保留二者：先检查基值，再通过 `into_input()` 取得完整恢复输入的所有权。

```rust
fn update_owned_demo() {
    use qubit_reflect::construct::UpdateInput;
    use qubit_reflect::value::Local;

    let ty = qubit_reflect::TypeDescriptor::of::<NewCustomer>();
    let customer = NewCustomer { email: String::from("ada@corp.example"), credit_limit_cents: 10_000 };
    let updated = ty.update::<NewCustomer, _>(
        UpdateInput::<Local>::new(customer)
            .field("email", String::from("new@corp.example"))
            .field("credit_limit_cents", 75_000_u64),
    ).expect("更新客户");
    assert_eq!(updated.email, "new@corp.example");
    assert_eq!(updated.credit_limit_cents, 75_000);

    let customer = NewCustomer { email: String::from("before@corp.example"), credit_limit_cents: 20_000 };
    let failure = ty.update::<NewCustomer, _>(
        UpdateInput::<Local>::new(customer)
            .field("email", String::from("after@corp.example"))
            .field("credit_limit_cents", String::from("类型错误")),
    ).expect_err("拒绝错误字段类型");
    assert_eq!(failure.base().downcast_ref::<NewCustomer>().expect("恢复基值").credit_limit_cents, 20_000);
    let recovered = failure.into_input();
    let repeated_failure = ty.update::<NewCustomer, _>(recovered)
        .expect_err("恢复的替换值仍可再次被拒绝");
    assert_eq!(repeated_failure.base().downcast_ref::<NewCustomer>().expect("恢复基值").credit_limit_cents, 20_000);
    drop(repeated_failure.into_input());
}
```

这证明用户代码执行前的类型校验和输入所有权恢复，并不构成事务：它无法撤销任意用户副作用，也不处理析构函数 panic 的回滚。应用仍负责业务校验、持久化以及事务或补偿策略。

## 选择显式 context

`include_type` 选择某个具体类型注册；`include_linked` 汇集 linked registrations；`include_linked_for` 为目标选择其 linked 闭包，并按库的规则纳入全局定义和 capability。它用于选择注册集合，不是安全边界，也不保证排除所有无关元数据。

```rust
fn explicit_context_demo() {
    use qubit_reflect::{Invocation, ReflectionContext, TypeDescriptor};
    use qubit_reflect::value::Local;

    let context = ReflectionContext::builder()
        .include_linked_for::<Customer>()
        .build()
        .expect("构建所选 context");
    let ty = TypeDescriptor::of_in::<Customer>(&context);
    assert!(ty.method("raise_credit_limit").is_ok());
    let mut customer = Customer { email: String::from("ada@corp.example"), credit_limit_cents: 10_000, internal_note: String::new() };
    let output = ty.method("raise_credit_limit").expect("已注册方法")
        .invoke(Invocation::<Local>::on_mut(&mut customer).arg(25_000_u64))
        .unwrap_or_else(|_| panic!("调用已注册方法"));
    assert!(output.into_owned::<bool>().unwrap_or_else(|_| panic!("布尔结果")));
    assert_eq!(customer.credit_limit_cents, 25_000);
}
```

应处理 `build()` 返回的 `ContextError`，并根据诊断定位缺失或无效的注册。选择显式 context 不能替代鉴权。

链接注册工厂会在调用 `include_linked()` 或 `include_linked_for()` 时执行，
另一个 builder 再次选择它们时也可能重跑。工厂应只生成注册数据：不要递归查询或
初始化 `ReflectionContext::global()`，也不要在工厂内构建 linked context 来检查注册项。
同一线程递归初始化 global 会 panic；工厂 panic 会从 include 调用处传播，`build()`
不会捕获它。

## 恢复被拒绝的更新和调用

typed 字段替换失败时返回 `FieldSetFailure<V>`。调用 `into_parts()` 可以同时取回诊断信息和原替换值，便于 handler 保留或报告输入。方法开始执行前，调用校验可能因为 receiver 或参数的类型、传递模式、参数绑定或方法可用性而失败。此时 `InvocationFailure` 会保留执行前被拒绝的输入。

用户代码一旦开始执行，反射无法撤销它产生的副作用。如果方法先修改对象再返回业务错误，调用方应使用自身的事务或补偿策略。普通 `invoke()` 也不会捕获 panic；只有应用已经确定 panic 处理策略时，才选择显式的捕获入口。

## 在框架边界使用动态值

泛型应用代码仍知道 Rust 类型时，优先使用 typed 字段访问。框架收到类型擦除的对象或新值时，可使用 `DynamicOwned<Local>`、`DynamicRef<Local>` 或 `DynamicMut<Local>`。动态操作仍会在访问前检查目标和字段值的精确类型。downcast 失败会归还原值，字段替换失败也会保留替换输入。

当值和适配器满足相应 `Send`、`Sync` 约束时，可以使用 `ThreadSafe` 模式。类型擦除后不能把 `Local` 值升级为 `ThreadSafe`。应在已知所有权和并发要求的边界选择模式。

derive 为 `async fn` 生成的调用适配器目前仅支持 `Local`。稳定过程宏无法推断 opaque future 的 `ThreadSafe` adapter，因此即使某个实现的 future 满足 `Send`，也不会自动生成该 adapter。若要使用 `ThreadSafe` adapter，可用 `bind_method!(... as thread_safe async fn(...))` 显式绑定具体方法；编译器会检查生成的 future 是否满足 `Send`，无需命名 opaque future 类型。

## 绑定泛型方法

Rust 会在编译期生成泛型方法的具体代码。构建显式 context 时，使用 `bind_method!` 注册各个受支持的特化；运行时反射不会依据 `TypeId` 合成新的单态化代码。

对于以 pinned 方式接收泛型参数的方法，选中的绑定还会提供 pinned 调用校验所需的
具体参数类型。校验仍会检查 receiver 和全部参数；拒绝调用时会返回原 pinned
receiver 与参数。

```rust
use qubit_reflect::descriptor::MethodArguments;
use qubit_reflect::value::Local;
use qubit_reflect::descriptor::Reflect as ReflectTrait;
use qubit_reflect::{bind_method, Invocation, ReflectImpl, ReflectionContext};

#[qubit_reflect::Reflect]
struct Converter;

#[ReflectImpl]
impl Converter {
    pub fn convert<T>(&self, value: String) -> T
    where
        T: ReflectTrait + std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Debug,
    {
        value.parse().expect("示例输入可解析为选择的类型")
    }
}

fn convert_demo() {
    let context = ReflectionContext::builder()
        .include_linked()
        .bind(bind_method!(Converter::convert::<u8> as fn(&Converter, String) -> u8))
        .build()
        .expect("显式 context 包含 converter 绑定");
    let method = context
        .type_of::<Converter>()
        .method("convert")
        .expect("泛型方法已反射");
    let specialization = method
        .specialization(&MethodArguments::new().type_of::<u8>())
        .expect("u8 特化已绑定");
    let converter = Converter;
    let output = match specialization
        .invoke(Invocation::<Local>::on_ref(&converter).arg(String::from("42")))
    {
        Ok(output) => output,
        Err(failure) => panic!("converter 方法执行成功: {:?}", failure.error().kind()),
    };
    let value = match output.into_owned::<u8>() {
        Ok(value) => value,
        Err(_) => panic!("结果类型为 u8"),
    };
    assert_eq!(value, 42);
}
```

`Converter::convert` 将文本解析为选定的结果类型；此处 context 绑定了 `u8` 特化。该正文示例由文档片段测试编译并执行。

trait 方法绑定时使用显式身份提示。对本地反射 trait，`reflected_trait` 指向 trait 定义；binder 会从 impl 的 trait 路径调用隐藏 hook，因此 rustc 能解析导入 alias 并补齐默认类型和 const 实参：

```text
.bind(bind_method!(
    <Service as ImportedService>::refresh as fn(&Service) -> String,
    reflected_trait = api::ServiceContract,
))
```

对外部 trait，先用 `reflect_external_trait!` 创建 identity handle，再以 `external_trait = Handle` 绑定显式方法。handle 不提供外部方法声明或默认参数信息；application identity 仅包含 impl 路径中明确写出的类型和 const 实参。此绑定是调用者登记的方法事实。

方法描述符仍表示源码声明。调用前使用 `MethodArguments` 选择具体特化。未绑定或不受支持的方法仍会以元数据和可用性原因呈现，不会被静默地绕过安全检查。

比较保留的类型表达式元数据时，使用 `TypeExpression::structurally_eq`，不要比较 `display()` 输出。显示文本用于诊断；结构比较检查源码事实，不证明 Rust 类型身份。

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
