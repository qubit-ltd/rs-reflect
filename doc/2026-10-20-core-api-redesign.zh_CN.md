# `rs-reflect` vNext 核心 API 设计

日期：2026-10-20

## 1. 文档状态

本文是已经确认的 `rs-reflect` vNext 核心 API 正式设计。设计允许必要的破坏性变更，不以兼容现有公开 API 为目标。

本文当前覆盖：

- `ReflectionContext` 与全局、显式反射环境；
- context-bound descriptor 模型；
- 类型发现、名称与查找规则；
- 类型、字段和 enum variant 的核心 API；
- 方法查找的基本规则；
- 方法声明、有效实现与泛型特化；
- Invocation 输入、输出、失败恢复与调用模式；
- struct 与 enum variant 构造、拥有值更新及恢复；
- 类型安全的 capability 查询、枚举与注册；
- 泛型类型定义、具体泛型参数与结构化类型表达式；
- trait/impl 的源码定义、具体应用、关联项与方法命名空间；
- 拥有、共享借用、可变借用及线程安全模式的动态值模型；
- lookup、context、类型不匹配、恢复与 panic 的总体错误模型；
- 定义信息自动采集、linked registration 与版本化 codegen 协议；
- 必须存在且唯一的便捷查找；
- 动态字段访问的用途；
- enum discriminant 的语义。

核心 API 形状已经覆盖完成，并已完成首轮全文一致性审计。审计统一了：

- context 聚合查询的 `ContextError` 边界和集合返回类型；
- `Lookup<T>`、`LookupError`、严格单数 accessor 与 `lookup_*()` 的统一查找模型；
- `CapabilityError` 的命名和查询返回类型；
- typed 与 dynamic 字段替换共用的 `FieldSetFailure<V>` 恢复模型；
- 未实现 `Reflect` 的字段类型只能保证 `TypeExpression`，具体 `TypeDescriptor` 为可选；
- 不要求携带任意恢复值的 failure 本身实现 `std::error::Error`。

核心 API 形状已经整体确认。第 24—26 节给出实现分层、破坏性迁移顺序和验证矩阵；实际编码仍应先拆分成可独立审查的实施任务。

## 2. 设计目标

新版核心 API 遵循以下原则：

1. 普通用户取得 `TypeDescriptor` 后即可完成反射操作，不需要显式初始化或传递 registry。
2. 全局反射环境是默认行为，但测试、白名单、宿主隔离和自定义泛型方法绑定仍可使用独立环境。
3. descriptor 自动绑定其来源 context，方法查找和调用不允许误用不同 context。
4. 字段、variant 等类型固有信息不依赖反射环境初始化。
5. 方法、trait impl、capability 和全局类型发现等聚合信息按需解析 context。
6. 反射呈现原始 Rust 定义信息，不提供通用的“反射重命名”。
7. 缺失、歧义、类型不匹配和初始化失败均使用结构化 `Result`，常规便捷 API 不通过 panic 报错。
8. 单数 accessor 默认要求结果存在且唯一；`lookup_*()` 暴露完整查找状态；集合 `.get()` 和 `find_*()` 表达可选查询，不提供 `require_*()`。

## 3. 核心对象模型

### 3.1 `ReflectionContext`

`ReflectionContext` 取代 `ReflectRegistry` 作为公开概念，表示一个不可变、可共享的反射环境。

反射环境有两种来源：

- 进程级全局 context：始终存在，内部惰性初始化，成功或失败均缓存；
- 显式 context：由 builder 事务式构建，构建成功后已经完成验证。

```rust
pub struct ReflectionContext {
    // opaque
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReflectionContextId {
    // opaque
}
```

### 3.2 context-bound descriptor

所有面向用户的 descriptor 都是“静态 metadata + context”的轻量只读句柄：

```rust
#[derive(Clone, Copy)]
pub struct TypeDescriptor<'ctx> {
    metadata: &'static TypeMetadata,
    context: &'ctx ReflectionContext,
}

#[derive(Clone, Copy)]
pub struct FieldDescriptor<'ctx> {
    metadata: &'static FieldMetadata,
    context: &'ctx ReflectionContext,
}

#[derive(Clone, Copy)]
pub struct VariantDescriptor<'ctx> {
    metadata: &'static VariantMetadata,
    context: &'ctx ReflectionContext,
}

#[derive(Clone, Copy)]
pub struct MethodDescriptor<'ctx> {
    method: &'ctx ResolvedMethod,
    context: &'ctx ReflectionContext,
}
```

`TypeMetadata`、`FieldMetadata`、`VariantMetadata`、`ResolvedMethod` 等属于宏生成协议，不进入普通公共 API。它们仅通过 `__private` 提供给生成代码。

不再引入单独的 `ReflectedType`。`TypeDescriptor<'ctx>` 本身就是绑定 context 的类型句柄。

### 3.3 默认与显式入口

```rust
impl TypeDescriptor<'static> {
    /// 绑定进程级全局 context；本操作本身不触发初始化。
    pub fn of<T: Reflect + ?Sized>() -> Self;
}
```

以下两种写法等价：

```rust
let ty = TypeDescriptor::of::<Customer>();
let ty = ReflectionContext::global().type_of::<Customer>();
```

显式环境返回同一种 descriptor：

```rust
let context = ReflectionContext::builder()
    .include_linked()
    .build()?;

let ty: TypeDescriptor<'_> = context.type_of::<Customer>();
```

从 descriptor 导航得到的子 descriptor 继承相同 context：

```rust
let field = ty.field("address")?;
let address_type = field.concrete_type()?; // TypeDescriptor<'ctx>

let method = ty.method("save")?;
method.invoke(Invocation::on_ref(&value))?; // 不再传 context
```

### 3.4 descriptor 相等性

不同 context 中同一个 Rust 类型可能观察到不同的方法和 capability，因此公开 descriptor 暂不实现含义模糊的 `Eq` 和 `Hash`。

- 比较 Rust 类型身份：使用 `type_id()`；
- 比较完整反射视图来源：同时比较 `context().id()` 和成员身份。

## 4. 公开模块

```text
qubit_reflect
├── context       ReflectionContext、ReflectionContextBuilder、发现与查找
├── descriptor    Type、Field、Variant、Method、Trait、Impl、泛型定义 descriptor
├── value         DynamicOwned、DynamicRef、DynamicMut、Local、ThreadSafe
├── construct     构造、更新、输入、恢复
├── invoke        Invocation、输出、失败、future
├── capability    CapabilitySpec、Capability、内置能力
├── registration  高级自定义 context 注册 API
├── error         公共错误类型
└── __private     宏生成协议与静态 metadata
```

crate 根只重导高频类型：

```rust
pub use context::ReflectionContext;
pub use descriptor::{
    FieldDescriptor, MethodDescriptor, TypeDescriptor, VariantDescriptor,
};
pub use construct::{ConstructionInput, UpdateInput};
pub use invoke::{Invocation, InvocationOutput};
pub use error::ReflectionError;

#[cfg(feature = "derive")]
pub use qubit_reflect_derive::{
    bind_method,
    reflect_external_trait,
    register_capability,
    register_method_binding,
    register_trait_object,
    Reflect,
    ReflectImpl,
};
```

## 5. `ReflectionContext` API

```rust
impl ReflectionContext {
    /// 获得进程级惰性 context，不触发初始化。
    pub fn global() -> &'static Self;

    /// 提前完成初始化和一致性校验。
    /// 对已经构建好的显式 context 调用时直接成功。
    pub fn initialize(&self) -> Result<&Self, ContextError>;

    pub fn is_initialized(&self) -> bool;
    pub fn id(&self) -> ReflectionContextId;

    /// 返回空的显式 context builder。
    pub fn builder() -> ReflectionContextBuilder;

    /// 返回绑定当前 context 的类型 descriptor。
    /// T 不必属于当前 context 的名称发现索引。
    pub fn type_of<T: Reflect + ?Sized>(&self) -> TypeDescriptor<'_>;

    /// 通过精确的进程内 Rust 类型身份查找已注册类型。
    pub fn find_type_by_id(
        &self,
        id: TypeId,
    ) -> Result<Option<TypeDescriptor<'_>>, ContextError>;

    pub fn type_by_id(
        &self,
        id: TypeId,
    ) -> Result<TypeDescriptor<'_>, LookupError>;

    /// 按类型定义处的完整反射名称查找。
    pub fn lookup_type_by_name(
        &self,
        name: &str,
    ) -> Result<Lookup<TypeDescriptor<'_>>, ContextError>;

    /// 只按最后一个类型标识符执行完整状态查找。
    pub fn lookup_type_by_short_name(
        &self,
        short_name: &str,
    ) -> Result<Lookup<TypeDescriptor<'_>>, ContextError>;

    /// 要求完整反射名称恰好对应一个类型。
    pub fn type_by_name(
        &self,
        name: &str,
    ) -> Result<TypeDescriptor<'_>, LookupError>;

    /// 要求短名称恰好对应一个类型。
    pub fn type_by_short_name(
        &self,
        short_name: &str,
    ) -> Result<TypeDescriptor<'_>, LookupError>;

    pub fn types(&self) -> Result<Types<'_>, ContextError>;

    pub fn type_definitions(
        &self,
    ) -> Result<TypeDefinitions<'_>, ContextError>;

    pub fn trait_definitions(
        &self,
    ) -> Result<TraitDefinitions<'_>, ContextError>;

    pub fn trait_applications(
        &self,
    ) -> Result<Traits<'_>, ContextError>;
}
```

### 5.1 名称与身份

类型提供三种相关信息：

```rust
ty.type_id();        // 精确的进程内 Rust 类型身份
ty.name();           // 类型定义处的完整反射名称
ty.short_name();     // 最后一个类型标识符
ty.rust_type_name(); // 编译器诊断字符串，不用于稳定查找
```

完整反射名称默认形如：

```text
my_crate::customer::Customer
my_crate::page::Page<my_crate::customer::Customer>
```

即使使用完整名称，结果仍可能不唯一。例如同一个 crate 的两个版本可以同时提供 `schema::model::User`，它们的文本名称相同但 `TypeId` 不同。context 允许这种合法情况共存：`type_by_name()` 要求结果唯一，`lookup_type_by_name()` 则返回完整 `Lookup` 状态，而不是让 context 构建失败。

### 5.2 不提供反射重命名

新版删除通用的反射重命名和 `query_name()` 概念。反射 API 只呈现原始 Rust 定义中的名称。

schema、RPC、配置、序列化和插件协议需要的稳定外部名称由对应上层框架维护。例如序列化名称属于 serde，而不是核心反射名称。

### 5.3 编译期已知类型与可发现成员

`type_of::<T>()` 表示调用方在编译期已经知道 `T`。只要 `T: Reflect`，就可以取得 descriptor；该操作不会修改不可变 context。

例如：

```rust
let context = ReflectionContext::builder()
    .include_type::<Customer>()
    .build()?;

let vector = context.type_of::<Vec<Customer>>();

assert!(vector.as_sequence().is_some());
assert!(!vector.is_registered()?);
```

`Vec<Customer>` 的描述逻辑由 `Vec<T>: Reflect` 在编译期单态化，因此可以导航其结构；但它没有被 builder 加入 context 的名称发现索引，所以：

- 不出现在 `context.types()` 中；
- 不能通过 `type_by_name()` 或 `lookup_type_by_name()` 发现；
- 不会暗中改变全局或显式 context；
- 只能观察当前 context 中确实为其注册的方法、impl 和外部 capability。

## 6. 严格 accessor 与完整查找状态

公开查找 API 采用以下命名规则：

- 单数 accessor，例如 `field()`、`method()`、`type_by_name()`，要求结果存在且唯一，返回 `Result<T, LookupError>`；
- `lookup_*()` 暴露 `Missing`、`Unique`、`Ambiguous` 三种完整状态；
- 固有唯一的成员使用集合 `.get()` 做可选查询，例如 `fields().get("name")`；
- 没有自然集合入口的可选查询使用 `find_*()`，例如 `find_definition()`；
- 不提供 `require_*()`，也不让任何上述 API 因普通缺失或歧义而 panic。

```rust
pub enum Lookup<T> {
    Missing,
    Unique(T),
    Ambiguous(Box<[T]>),
}

impl<T> Lookup<T> {
    pub fn is_missing(&self) -> bool;
    pub fn is_unique(&self) -> bool;
    pub fn is_ambiguous(&self) -> bool;
    pub fn len(&self) -> usize;
    pub fn candidates(&self) -> &[T];

    /// 仅在唯一时返回候选；缺失或歧义均返回 None。
    pub fn only(self) -> Option<T>;

    /// 消费查找结果并取得唯一候选，由调用点补充结构化查询上下文。
    pub fn into_unique(
        self,
        subject: LookupSubject,
        query: LookupQuery,
    ) -> Result<T, LookupError>
    where
        T: IntoLookupCandidate;
}
```

当调用方明确要求目标必须存在且唯一时，使用直接便利方法，不需要手写 `match`：

```rust
let ty = context.type_by_name("my_crate::model::Customer")?;
let method = ty.method("save")?;
method.invoke(Invocation::on_ref(&customer))?;
```

缺失和歧义均返回第 21 节定义的统一 `LookupError`；错误中保留查询主体、查询条件和候选身份。

## 7. `ReflectionContextBuilder`

```rust
pub struct ReflectionContextBuilder {
    // opaque
}

impl ReflectionContextBuilder {
    /// 创建空环境。
    pub fn new() -> Self;

    /// 包含当前二进制链接的全部反射注册。
    pub fn include_linked(self) -> Self;

    /// 只加入 T 的类型成员及其类型固有 metadata；
    /// 不隐式加入该类型的所有 impl。
    pub fn include_type<T: Reflect + ?Sized>(self) -> Self;

    /// 加入 T，以及链接目录中以 T 为目标的方法、trait impl、
    /// capability 和所需声明依赖闭包。
    pub fn include_linked_for<T: Reflect + ?Sized>(self) -> Self;

    /// 加入一项精确的高级注册事实。
    pub fn register(self, registration: impl Into<Registration>) -> Self;

    /// 加入 bind_method! 产生的具体泛型方法绑定。
    pub fn bind(self, binding: MethodBinding) -> Self;

    /// 事务式校验并冻结。
    pub fn build(self) -> Result<ReflectionContext, ContextError>;
}

impl Default for ReflectionContextBuilder {
    /// 等价于 new()，即空 context。
    fn default() -> Self;
}
```

常见模式：

```rust
let complete = ReflectionContext::builder()
    .include_linked()
    .build()?;

let customer_only = ReflectionContext::builder()
    .include_linked_for::<Customer>()
    .build()?;

let with_binding = ReflectionContext::builder()
    .include_linked()
    .bind(bind_method!(
        GenericService::<u8>::read
        as fn(&GenericService<u8>) -> &u8
    ))
    .build()?;
```

成功构建的 context 不可变并实现 `Send + Sync`。descriptor 只借用 context；显式 context 释放后，其 descriptor 不能逃逸。builder 不修改全局 context。

## 8. `TypeDescriptor`

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;
    pub fn type_id(self) -> TypeId;
    pub fn name(self) -> &'static str;
    pub fn short_name(self) -> &'static str;
    pub fn rust_type_name(self) -> &'static str;
    pub fn kind(self) -> TypeKind<'ctx>;

    pub fn is_registered(self) -> Result<bool, ContextError>;

    /// 非泛型类型返回空集合。
    pub fn generic_arguments(self) -> GenericArguments<'ctx>;

    pub fn find_definition(self) -> Option<TypeDefinitionDescriptor<'ctx>>;

    /// 要求当前类型具有唯一的源码级名义定义。
    pub fn definition(
        self,
    ) -> Result<TypeDefinitionDescriptor<'ctx>, ReflectionError>;

    pub fn fields(self) -> Fields<'ctx>;
    pub fn field(
        self,
        name: &str,
    ) -> Result<FieldDescriptor<'ctx>, LookupError>;

    pub fn variants(self) -> Variants<'ctx>;
    pub fn variant(
        self,
        name: &str,
    ) -> Result<VariantDescriptor<'ctx>, LookupError>;

    pub fn lookup_method(
        self,
        name: &str,
    ) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError>;

    pub fn method(
        self,
        name: &str,
    ) -> Result<MethodDescriptor<'ctx>, LookupError>;
}
```

单数 accessor 表示“必须存在且唯一”。字段和 variant 的名称在各自声明作用域中必须唯一，因此 `field()` 和 `variant()` 只需处理缺失；可选查询使用 `fields().get()` 和 `variants().get()`。方法可能跨 namespace 同名，因此 `method()` 要求唯一，`lookup_method()` 暴露完整 `Lookup` 状态。

## 9. 字段和 variant 集合视图

`fields()` 和 `variants()` 返回绑定相同 context 的轻量只读集合视图，不直接暴露内部 metadata slice。

```rust
pub struct Fields<'ctx> {
    // context + metadata slice
}

impl<'ctx> Fields<'ctx> {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, name: &str) -> Option<FieldDescriptor<'ctx>>;
    pub fn get_at(&self, index: usize) -> Option<FieldDescriptor<'ctx>>;
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = FieldDescriptor<'ctx>> + '_;
}

pub struct Variants<'ctx> {
    // context + metadata slice
}

impl<'ctx> Variants<'ctx> {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, name: &str) -> Option<VariantDescriptor<'ctx>>;
    pub fn get_at(&self, index: usize) -> Option<VariantDescriptor<'ctx>>;
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = VariantDescriptor<'ctx>> + '_;
}
```

具体返回关系为：

- `ty.variants()` 返回 `Variants<'ctx>`；
- `ty.variants().get(name)` 返回 `Option<VariantDescriptor<'ctx>>`；
- `ty.variants().get_at(index)` 返回 `Option<VariantDescriptor<'ctx>>`；
- `ty.variant(name)` 返回 `Result<VariantDescriptor<'ctx>, LookupError>`。

对非 enum 类型：

- `variants()` 返回空视图；
- `variants().get()`、`variants().get_at()` 返回 `None`；
- `variant()` 返回说明目标类型不是 enum 或成员不存在的结构化错误。

`variants().get_at(index)` 的 index 是 variant 在 Rust 源码声明中的从零开始顺序，不是 enum discriminant。

## 10. 类型种类

`kind()` 直接返回携带具体视图的 enum：

```rust
pub enum TypeKind<'ctx> {
    Primitive(PrimitiveType<'ctx>),
    Text(TextType<'ctx>),
    Struct(StructType<'ctx>),
    Enum(EnumType<'ctx>),
    Tuple(TupleType<'ctx>),
    Array(ArrayType<'ctx>),
    Optional(OptionalType<'ctx>),
    Sequence(SequenceType<'ctx>),
    Set(SetType<'ctx>),
    Map(MapType<'ctx>),
    SmartPointer(SmartPointerType<'ctx>),
    Reference(ReferenceType<'ctx>),
    Slice(SliceType<'ctx>),
    RawPointer(RawPointerType<'ctx>),
    Function(FunctionType<'ctx>),
    TraitObject(TraitObjectType<'ctx>),
    Opaque(OpaqueType<'ctx>),
}
```

只保留少量高频便利转换：

```rust
ty.as_struct();   // Option<StructType<'ctx>>
ty.as_enum();     // Option<EnumType<'ctx>>
ty.as_sequence(); // Option<SequenceType<'ctx>>
```

## 11. `FieldDescriptor`

```rust
impl<'ctx> FieldDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;
    pub fn declaring_type(self) -> TypeDescriptor<'ctx>;
    pub fn declaring_variant(self) -> Option<VariantDescriptor<'ctx>>;

    pub fn index(self) -> usize;
    pub fn name(self) -> &'static str;

    /// 源码中始终存在的字段类型表达式。
    pub fn type_expression(self) -> TypeExpression<'ctx>;

    /// 字段类型具有可用 Reflect metadata 时返回具体 descriptor。
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>>;

    pub fn concrete_type(
        self,
    ) -> Result<TypeDescriptor<'ctx>, ReflectionError>;

    pub fn visibility(self) -> Visibility;
    pub fn access(self) -> FieldAccess;

    pub fn get<'a, T: 'static, V: 'static>(
        self,
        target: &'a T,
    ) -> Result<&'a V, FieldAccessError>;

    pub fn get_mut<'a, T: 'static, V: 'static>(
        self,
        target: &'a mut T,
    ) -> Result<&'a mut V, FieldAccessError>;

    pub fn set<T: 'static, V: 'static>(
        self,
        target: &mut T,
        value: V,
    ) -> Result<(), FieldSetFailure<V>>;

    pub fn get_dynamic<'a, M: ValueMode>(
        self,
        target: DynamicRef<'a, M>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError>;

    pub fn get_dynamic_mut<'a, M: ValueMode>(
        self,
        target: DynamicMut<'a, M>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError>;

    pub fn set_dynamic<M: ValueMode>(
        self,
        target: DynamicMut<'_, M>,
        value: DynamicOwned<M>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>>;
}
```

私有字段可以被描述，但安全访问返回 `FieldAccessError::NotPublic`。对应的 `unsafe get_unsafe`、`get_mut_unsafe` 和 `set_unsafe` 保留在同一类型上，但不进入常规文档主路径。

字段能力合并为：

```rust
pub struct FieldAccess {
    /// 以下标志描述安全动态 API 的可用性。
    pub readable: bool,
    pub mutable: bool,
    pub replaceable: bool,
    pub constructible: bool,
}
```

typed `get`、`get_mut` 和 `set` 仍可通过调用点的准确 `T`、`V` 做类型检查；动态 API 还要求字段类型具有可用的 `Reflect` adapter。动态能力不可用时，对应标志为 `false`，强行调用返回 `FieldAccessError::Unavailable`，而不是隐藏字段。

### 11.1 动态字段访问的用途

泛型业务代码在编译期知道目标和字段值类型时，使用 typed API：

```rust
let email: &String = ty.field("email")?.get(&customer)?;
```

框架代码往往只在运行时获得：

- 一个类型擦除的目标对象；
- 请求中的字段名；
- 解析后但类型擦除的新值。

例如通用对象补丁器：

```rust
fn apply_patch(
    ty: TypeDescriptor<'_>,
    target: DynamicMut<'_, Local>,
    field_name: &str,
    replacement: DynamicOwned<Local>,
) -> Result<(), PatchError> {
    ty.field(field_name)?
        .set_dynamic(target, replacement)?;
    Ok(())
}
```

调用方不需要为每个业务类型编写 `match`：

```rust
match field_name {
    "email" => customer.email = value.downcast::<String>()?,
    "age" => customer.age = value.downcast::<u32>()?,
    _ => return Err(...),
}
```

动态 API 不是放弃类型安全。它在修改目标前仍检查：

- target 的精确 `TypeId` 是否等于字段所属类型；
- replacement 的精确 `TypeId` 是否等于字段类型；
- 字段可见性和访问策略；
- enum variant 字段是否属于当前活动 variant。

检查失败时返回原始 replacement，便于调用方记录、修正或重试。

```rust
pub struct FieldSetFailure<V> {
    error: FieldSetError,
    replacement: V,
}

impl<V> FieldSetFailure<V> {
    pub fn error(&self) -> &FieldSetError;
    pub fn replacement(&self) -> &V;

    pub fn into_parts(
        self,
    ) -> (FieldSetError, V);
}
```

## 12. `VariantDescriptor`

```rust
impl<'ctx> VariantDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;
    pub fn declaring_type(self) -> TypeDescriptor<'ctx>;

    pub fn index(self) -> usize;
    pub fn name(self) -> &'static str;
    pub fn shape(self) -> VariantShape;
    pub fn visibility(self) -> Visibility;

    pub fn fields(self) -> Fields<'ctx>;
    pub fn field(
        self,
        name: &str,
    ) -> Result<FieldDescriptor<'ctx>, LookupError>;

    pub fn discriminant(self) -> VariantDiscriminant;

    pub fn is_active<T: 'static>(
        self,
        value: &T,
    ) -> Result<bool, TypeMismatch>;

    pub fn is_active_dynamic(
        self,
        value: DynamicRef<'_>,
    ) -> Result<bool, TypeMismatch>;
}
```

### 12.1 discriminant 语义

enum discriminant 是 Rust 用于区分 enum variant 的判别身份。例如：

```rust
#[repr(u8)]
enum Status {
    Pending = 1,
    Ready = 2,
}
```

`Status::Ready` 的数值 discriminant 是 `2_u8`，并且该值来自源码中的显式表达式。

API 不应假设每个 variant 都能暴露稳定整数。修订后的形状为：

```rust
pub struct VariantDiscriminant {
    pub origin: DiscriminantOrigin,
    pub numeric: Option<NumericDiscriminant>,
}

pub enum DiscriminantOrigin {
    Implicit,
    Explicit,
}

pub enum NumericDiscriminant {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
}
```

示例：

```rust
#[repr(u8)]
enum Status {
    Pending = 1,
    Ready = 2,
}

// Ready => origin: Explicit, numeric: Some(U8(2))

#[repr(u8)]
enum Direction {
    North,
    South,
}

// South => origin: Implicit, numeric: Some(U8(1))

enum Message {
    Text(String),
    Quit,
}

// variant 身份可通过 is_active() 判断；没有承诺稳定整数表示，
// 因此 numeric 为 None。
```

`variant.index()` 是源码声明顺序，`variant.discriminant()` 是 Rust enum 判别信息，两者不能混用。

## 13. 方法名称与基本查找规则

Rust 不支持按参数签名进行传统 overload，但一个具体类型可以同时具有同名的 inherent method 和多个 trait method：

```text
Service::reset
<Service as CacheControl>::reset
<Service as ConnectionControl>::reset
```

因此不带命名空间限定的方法查找可能歧义：

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    /// 跨 inherent 和全部 trait 命名空间查询。
    pub fn lookup_method(
        self,
        name: &str,
    ) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError>;

    /// 要求跨命名空间查询结果存在且唯一。
    pub fn method(
        self,
        name: &str,
    ) -> Result<MethodDescriptor<'ctx>, LookupError>;

    /// 可选查找 inherent method；该命名空间内名称唯一。
    pub fn find_inherent_method(
        self,
        name: &str,
    ) -> Result<Option<MethodDescriptor<'ctx>>, ContextError>;

    pub fn inherent_method(
        self,
        name: &str,
    ) -> Result<MethodDescriptor<'ctx>, LookupError>;

    /// 可选查找指定 trait 命名空间；trait 内方法名称唯一。
    pub fn find_trait_method(
        self,
        trait_: TraitDescriptor<'ctx>,
        name: &str,
    ) -> Result<Option<MethodDescriptor<'ctx>>, ContextError>;

    pub fn trait_method(
        self,
        trait_: TraitDescriptor<'ctx>,
        name: &str,
    ) -> Result<MethodDescriptor<'ctx>, LookupError>;
}
```

不再提供 `methods_by_name()`。要求唯一时使用 `method()`；需要观察缺失、唯一和歧义三种状态时使用 `lookup_method()`，其中 `Lookup::Ambiguous` 保留全部候选。

同一个泛型方法的多个具体绑定属于一个源码方法声明，不当作多个同名声明。具体 specialization/binding 的选择规则见下一节。

## 14. 方法声明、有效实现与泛型特化

方法反射区分三个层次：

1. 源码中的方法声明；
2. 某个目标类型上的有效实现；
3. 泛型方法的具体可调用特化。

普通用户仍只使用一个主入口 `MethodDescriptor<'ctx>`，内部 metadata 分层不泄漏。

### 14.1 `MethodDescriptor`

```rust
#[derive(Clone, Copy)]
pub struct MethodDescriptor<'ctx> {
    metadata: &'static MethodMetadata,
    effective: &'ctx EffectiveMethod,
    context: &'ctx ReflectionContext,
}
```

它表示“当前 context 中，作用于某个具体类型的一个有效方法声明”。

```rust
impl<'ctx> MethodDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;
    pub fn target(self) -> TypeDescriptor<'ctx>;

    pub fn name(self) -> &'static str;
    pub fn qualified_name(self) -> MethodQualifiedName<'ctx>;
    pub fn namespace(self) -> MethodNamespace<'ctx>;

    pub fn visibility(self) -> Visibility;
    pub fn receiver(self) -> Option<ReceiverDescriptor<'ctx>>;

    pub fn parameters(self) -> Parameters<'ctx>;
    pub fn parameter(
        self,
        name: &str,
    ) -> Result<ParameterDescriptor<'ctx>, LookupError>;

    pub fn output(self) -> ReturnDescriptor<'ctx>;
    pub fn qualifiers(self) -> MethodQualifiers;
    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx>;

    pub fn implementation_source(self) -> MethodImplementationSource;
    pub fn availability(self) -> MethodAvailability;

    pub fn specializations(self) -> MethodSpecializations<'ctx>;

    pub fn specialization(
        self,
        arguments: &MethodArguments,
    ) -> Result<MethodSpecializationDescriptor<'ctx>, LookupError>;

    /// 当且仅当存在一个确定的 specialization 时直接调用。
    pub fn invoke<'call, M: ValueMode>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>>;
}
```

`MethodDescriptor::invoke()` 是普通非泛型方法的主路径：

```rust
let changed = ty
    .method("raise_credit_limit")?
    .invoke(
        Invocation::on_mut(&mut customer)
            .arg(75_000_u64),
    )?
    .into_owned::<bool>()?;
```

对于泛型方法：

- 恰好存在一个具体 specialization 时，`MethodDescriptor::invoke()` 直接转发调用；
- 不存在具体 specialization 时，返回未绑定错误；
- 存在多个具体 specialization 时，返回要求显式选择的错误，不根据参数运行时类型猜测。

### 14.2 方法命名空间

```rust
pub enum MethodNamespace<'ctx> {
    Inherent {
        implementation: ImplDescriptor<'ctx>,
    },
    Trait {
        trait_: TraitDescriptor<'ctx>,
        implementation: ImplDescriptor<'ctx>,
    },
}

pub enum MethodQualifiedName<'ctx> {
    Inherent {
        target: TypeDescriptor<'ctx>,
        name: &'static str,
    },
    Trait {
        target: TypeDescriptor<'ctx>,
        trait_: TraitDescriptor<'ctx>,
        name: &'static str,
    },
}
```

格式化后分别得到：

```text
Service::reset
<Service as CacheControl>::reset
```

限定名称使用结构化数据表达，不能依赖调用方解析拼接字符串。

### 14.3 trait 默认方法与覆盖

```rust
pub enum MethodImplementationSource {
    /// inherent impl 中直接声明的方法。
    Declared,

    /// trait 的必需方法，但当前没有可调用实现。
    Required,

    /// 使用 trait 提供的默认方法体。
    Defaulted,

    /// 具体 impl 覆盖了 trait 默认方法。
    Overridden,
}
```

`MethodDescriptor` 始终代表最终有效行为。普通调用方不需要自行合并 trait 默认方法和 impl 覆盖；诊断工具可以读取声明位置、有效实现位置和 `implementation_source()`。

### 14.4 参数与返回值

```rust
#[derive(Clone, Copy)]
pub struct ParameterDescriptor<'ctx> {
    // metadata + context
}

impl<'ctx> ParameterDescriptor<'ctx> {
    /// 不包含 receiver 的声明顺序。
    pub fn index(self) -> usize;

    /// 只有简单标识符模式才能按名称绑定。
    pub fn name(self) -> Option<&'static str>;

    pub fn pattern(self) -> ParameterPattern;
    pub fn passing(self) -> PassingMode;

    /// 保留泛型和关联类型的源码级类型表达式。
    pub fn type_expression(self) -> TypeExpression<'ctx>;

    /// 能解析到具体 Rust 类型时返回 descriptor。
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>>;
}

pub enum PassingMode {
    Owned,
    Shared,
    Mutable,
}

#[derive(Clone, Copy)]
pub struct ReturnDescriptor<'ctx> {
    // metadata + context
}

impl<'ctx> ReturnDescriptor<'ctx> {
    pub fn kind(self) -> ReturnKind;
    pub fn type_expression(
        self,
    ) -> Option<TypeExpression<'ctx>>;
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>>;
}
```

参数名称只来自原始 Rust 标识符。通配符和解构参数没有可绑定名称，只能按位置传递。

### 14.5 方法限定符

```rust
#[derive(Clone, Copy)]
pub struct MethodQualifiers {
    pub is_const: bool,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub abi: Option<FunctionAbi>,
    pub is_variadic: bool,
}
```

这些属性描述源码事实，不代表方法一定能通过动态边界调用。实际调用能力由 `availability()` 和具体 specialization 决定。

### 14.6 泛型方法特化

运行时 `TypeId` 不能生成新的 Rust 单态化代码，因此泛型方法必须在编译期绑定：

```rust
let context = ReflectionContext::builder()
    .include_linked()
    .bind(bind_method!(
        Converter::convert::<u8>
        as fn(&Converter, String) -> u8
    ))
    .bind(bind_method!(
        Converter::convert::<u64>
        as fn(&Converter, String) -> u64
    ))
    .build()?;
```

方法查找仍只得到一个源码声明：

```rust
let method = context
    .type_of::<Converter>()
    .method("convert")?;
```

具体绑定通过参数选择：

```rust
let u8_method = method.specialization(
    &MethodArguments::new().type_of::<u8>(),
)?;

let output = u8_method.invoke(
    Invocation::on_ref(&converter)
        .arg(String::from("42")),
)?;
```

核心类型：

```rust
pub struct MethodArguments {
    // concrete type/const arguments
}

impl MethodArguments {
    pub fn new() -> Self;
    pub fn type_of<T: Reflect + ?Sized>(self) -> Self;
    pub fn constant<T: Any>(self, value: T) -> Self;
}

#[derive(Clone, Copy)]
pub struct MethodSpecializationDescriptor<'ctx> {
    method: MethodDescriptor<'ctx>,
    resolved: &'ctx ResolvedMethodSpecialization,
}

impl<'ctx> MethodSpecializationDescriptor<'ctx> {
    pub fn method(self) -> MethodDescriptor<'ctx>;
    pub fn arguments(self) -> GenericArguments<'ctx>;
    pub fn availability(self) -> InvocationAvailability;

    pub fn invoke<'call, M: ValueMode>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>>;
}
```

特化规则：

- 非泛型方法始终有且仅有一个空参数 specialization，即使调用不可用；
- 未绑定的泛型方法有零个具体 specialization；
- 一个泛型方法可以有多个不同参数的 specialization；
- 同一 context 中完全相同的 specialization binding 属于构建错误；
- specialization 存在但签名不能安全跨越动态边界时，通过 `InvocationAvailability` 解释原因。

### 14.7 可调用性

```rust
pub enum MethodAvailability {
    Callable,
    RequiresSpecialization,
    RequiresSpecializationSelection {
        count: usize,
    },
    Unavailable {
        reasons: Box<[InvocationUnavailableReason]>,
    },
}

pub enum InvocationAvailability {
    Callable,
    Unavailable {
        reasons: Box<[InvocationUnavailableReason]>,
    },
}

pub enum InvocationUnavailableReason {
    UnsupportedReceiver,
    UnsupportedParameterPattern,
    UnspecializedGeneric,
    UnsafeMethod,
    UnsupportedAbi,
    Variadic,
    UnsupportedBorrowedReturn,
    OpaqueReturn,
    UnsupportedUnsizedValue,
    UnprovenTraitConstraint,
    DisabledByPolicy,
}
```

## 15. Invocation、输出与失败恢复

本节使用单一泛型调用入口，消除 `invoke_local()`、`invoke_thread_safe()`、`invoke_catching_local()` 等组合式 API。

### 15.1 调用模式

```rust
pub trait ValueMode: sealed::Sealed {
    // 动态值和 future 必须满足的边界
}

pub enum Local {}
pub enum ThreadSafe {}
```

`Local` 是默认模式；`ThreadSafe` 要求 owned value 满足 `Send + Sync`，异步 future 满足 `Send`。

```rust
pub struct Invocation<'call, M: ValueMode = Local> {
    // receiver + ordered argument bindings
}
```

普通调用统一为：

```rust
impl<'ctx> MethodDescriptor<'ctx> {
    pub fn invoke<'call, M: ValueMode>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<
        InvocationOutput<'call, M>,
        InvocationFailure<'call, M>,
    >;
}
```

调用模式由 `Invocation` 的类型决定：

```rust
// 默认 Local
method.invoke(
    Invocation::on_ref(&service)
        .arg(String::from("input")),
)?;

// 显式 ThreadSafe
method.invoke(
    Invocation::<ThreadSafe>::on_ref(&service)
        .arg(String::from("input")),
)?;
```

`MethodSpecializationDescriptor` 提供相同调用入口。不再公开 local/thread-safe 成对方法。

### 15.2 Invocation 构造

```rust
impl<'call, M: ValueMode> Invocation<'call, M> {
    /// associated function，没有 receiver。
    pub fn associated() -> Self;

    pub fn on_owned<T: ValueFor<M>>(receiver: T) -> Self;
    pub fn on_ref<T: ValueForRef<M>>(receiver: &'call T) -> Self;
    pub fn on_mut<T: ValueForMut<M>>(receiver: &'call mut T) -> Self;

    pub fn arg<T: ValueFor<M>>(self, value: T) -> Self;
    pub fn arg_ref<T: ValueForRef<M>>(self, value: &'call T) -> Self;
    pub fn arg_mut<T: ValueForMut<M>>(self, value: &'call mut T) -> Self;

    pub fn named_arg<T: ValueFor<M>>(
        self,
        name: impl Into<Box<str>>,
        value: T,
    ) -> Self;

    pub fn named_arg_ref<T: ValueForRef<M>>(
        self,
        name: impl Into<Box<str>>,
        value: &'call T,
    ) -> Self;

    pub fn named_arg_mut<T: ValueForMut<M>>(
        self,
        name: impl Into<Box<str>>,
        value: &'call mut T,
    ) -> Self;
}
```

位置参数与命名参数可以交错。位置参数绑定下一个尚未占用的声明位置；命名参数只能绑定简单标识符参数，通配符和解构参数只能按位置绑定。

高级框架可以直接传入动态值：

```rust
impl<'call, M: ValueMode> Invocation<'call, M> {
    pub fn on_dynamic(receiver: DynamicReceiver<'call, M>) -> Self;

    pub fn arg_dynamic(self, value: DynamicOwned<M>) -> Self;
    pub fn arg_dynamic_ref(self, value: DynamicRef<'call, M>) -> Self;
    pub fn arg_dynamic_mut(self, value: DynamicMut<'call, M>) -> Self;

    pub fn named_arg_dynamic(
        self,
        name: impl Into<Box<str>>,
        value: InvocationArgument<'call, M>,
    ) -> Self;
}
```

### 15.3 动态检查顺序

方法真正开始执行前依次完成：

1. 确认方法 specialization；
2. 检查方法可见性和安全性；
3. 绑定位置参数与命名参数；
4. 检查 receiver 是否存在及其 owned/ref/mut 模式；
5. 检查 receiver 的精确 `TypeId`；
6. 解析显式 receiver adapter；
7. 检查参数数量；
8. 检查每个参数的 passing mode；
9. 检查每个参数的精确 `TypeId`；
10. 选择与 `Local` 或 `ThreadSafe` 对应的调用入口。

所有检查成功后，adapter 才能提取 owned value 并进入用户方法。

### 15.4 输出

```rust
#[must_use]
pub enum InvocationOutput<'call, M: ValueMode = Local> {
    Unit,

    Owned(DynamicOwned<M>),

    Ref {
        value: DynamicRef<'call, M>,
        origins: Box<[BorrowOrigin]>,
    },

    OptionalRef {
        value: Option<DynamicRef<'call, M>>,
        origins: Box<[BorrowOrigin]>,
    },

    RefSlice {
        values: Box<[DynamicRef<'call, M>]>,
        origins: Box<[BorrowOrigin]>,
    },

    Mut {
        value: DynamicMut<'call, M>,
        origin: BorrowOrigin,
    },

    Future(ReflectedFuture<'call, M>),
}

pub enum BorrowOrigin {
    Receiver,
    Parameter(usize),
}
```

提供 typed 提取便利方法，并在失败时保留原始 output：

```rust
impl<'call, M: ValueMode> InvocationOutput<'call, M> {
    pub fn into_unit(self) -> Result<(), OutputMismatch<Self>>;

    pub fn into_owned<T: ValueFor<M>>(
        self,
    ) -> Result<T, OutputMismatch<Self>>;

    pub fn into_ref<T: 'static>(
        self,
    ) -> Result<&'call T, OutputMismatch<Self>>;

    pub fn into_mut<T: 'static>(
        self,
    ) -> Result<&'call mut T, OutputMismatch<Self>>;

    pub fn into_optional_ref<T: 'static>(
        self,
    ) -> Result<Option<&'call T>, OutputMismatch<Self>>;

    pub fn into_ref_slice<T: 'static>(
        self,
    ) -> Result<Box<[&'call T]>, OutputMismatch<Self>>;

    pub fn into_future(
        self,
    ) -> Result<ReflectedFuture<'call, M>, OutputMismatch<Self>>;
}
```

异步方法返回 future，由调用方选择 executor 并负责 poll：

```rust
let future = method
    .invoke(Invocation::on_ref(&service).arg(request))?
    .into_future()?;

let output = future.await;
let response: Response = output.into_owned()?;
```

### 15.5 调用失败和输入恢复

```rust
pub struct InvocationFailure<'call, M: ValueMode = Local> {
    error: InvocationError,
    recovery: InvocationRecovery<'call, M>,
}

impl<'call, M: ValueMode> InvocationFailure<'call, M> {
    pub fn error(&self) -> &InvocationError;
    pub fn recovery(&self) -> &InvocationRecovery<'call, M>;

    pub fn into_parts(
        self,
    ) -> (
        InvocationError,
        InvocationRecovery<'call, M>,
    );

    pub fn into_invocation(self) -> Invocation<'call, M>;
}
```

恢复保证：

- 用户方法执行前发生的失败完整保留 receiver 和全部参数；
- owned 参数不会在检查阶段提前消费；
- 调用方可以修正输入后重试；
- 用户方法一旦开始执行，不承诺回滚状态或恢复已消费的 owned value。

主要错误类型：

```rust
pub enum InvocationErrorKind {
    SpecializationRequired,
    MethodUnavailable {
        reasons: Box<[InvocationUnavailableReason]>,
    },

    VisibilityDenied,
    UnsafeMethod,

    ReceiverMissing,
    UnexpectedReceiver,
    ReceiverModeMismatch,
    ReceiverTypeMismatch,
    ReceiverAdapterUnavailable,
    ReceiverAdapterRejected,

    UnknownArgumentName,
    NamedArgumentUnavailable,
    DuplicateArgumentBinding,
    MissingArgument,
    TooManyArguments,
    ArgumentModeMismatch,
    ArgumentTypeMismatch,

    ValueModeUnavailable,
}
```

### 15.6 panic 行为

普通 `invoke()` 保持 Rust 原始语义：用户方法 panic 时继续 unwind，不被反射层默认吞掉。

需要隔离第三方调用时使用显式入口：

```rust
pub enum CaughtInvocationFailure<'call, M: ValueMode> {
    Rejected(InvocationFailure<'call, M>),
    Panicked(InvocationPanic),
}

impl<'ctx> MethodDescriptor<'ctx> {
    pub fn invoke_catching<'call, M: ValueMode>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<
        InvocationOutput<'call, M>,
        CaughtInvocationFailure<'call, M>,
    >;
}
```

语义：

- 前置检查失败返回 `Rejected`，输入完整可恢复；
- 用户方法开始后 panic 返回 `Panicked`，不承诺输入恢复或状态回滚；
- `panic=abort`、异步方法或未生成 catching adapter 时，在执行前返回可恢复的 `ValueModeUnavailable`；
- 异步 future 在后续 poll 时发生的 panic 不由同步调用入口捕获。

### 15.7 私有和 unsafe 方法

安全 `invoke()` 只允许调用公开且安全的方法。突破 Rust 源码访问或安全边界必须使用明确的 unsafe 入口：

```rust
impl<'ctx> MethodDescriptor<'ctx> {
    pub unsafe fn invoke_unchecked<'call, M: ValueMode>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<
        InvocationOutput<'call, M>,
        InvocationFailure<'call, M>,
    >;
}
```

该入口只绕过方法可见性检查和 `unsafe fn` 的安全调用限制，仍检查 specialization、receiver、参数数量、passing mode 和精确 `TypeId`。调用方承担私有不变量和 `unsafe fn` 契约。

### 15.8 pinned receiver

Pinned receiver 使用专用输入类型，避免将 `Pin<&mut T>` 无条件擦除进普通动态 receiver：

```rust
pub enum PinnedInvocation<
    'call,
    T: ?Sized,
    M: ValueMode = Local,
> {
    Shared {
        receiver: Pin<&'call T>,
        arguments: Box<[InvocationArgument<'call, M>]>,
    },
    Mutable {
        receiver: Pin<&'call mut T>,
        arguments: Box<[InvocationArgument<'call, M>]>,
    },
}
```

调用入口统一为：

```rust
pub fn invoke_pinned<'call, T, M>(
    self,
    invocation: PinnedInvocation<'call, T, M>,
) -> Result<
    InvocationOutput<'call, M>,
    PinnedInvocationFailure<'call, T, M>,
>
where
    T: ?Sized + 'static,
    M: ValueMode;
```

失败时保留原始 `Pin` receiver 和参数，不再拆分成 `invoke_pinned_ref()`、`invoke_pinned_mut()` 两套公开方法。

## 16. 构造与拥有值更新

构造 API 使用一个操作和三种输入形状，取代 named、tuple、unit、typed、dynamic 的方法组合。

### 16.1 构造输入

```rust
pub struct NamedFields<M: ValueMode = Local> {
    // ordered bindings
}

pub struct PositionalFields<M: ValueMode = Local> {
    // ordered values
}

pub struct UnitFields;

impl<M: ValueMode> NamedFields<M> {
    pub fn new() -> Self;

    pub fn field<T: ValueFor<M>>(
        self,
        name: impl Into<Box<str>>,
        value: T,
    ) -> Self;

    pub fn field_dynamic(
        self,
        name: impl Into<Box<str>>,
        value: DynamicOwned<M>,
    ) -> Self;
}

impl<M: ValueMode> PositionalFields<M> {
    pub fn new() -> Self;

    pub fn value<T: ValueFor<M>>(self, value: T) -> Self;

    pub fn value_dynamic(
        self,
        value: DynamicOwned<M>,
    ) -> Self;
}
```

三种输入通过 sealed trait 统一：

```rust
pub trait ConstructionInput<M: ValueMode>: sealed::Sealed {
    fn shape(&self) -> ConstructionShape;
}

pub enum ConstructionShape {
    Named,
    Positional,
    Unit,
}
```

调用方始终持有具体输入形状，不会出现对 named input 调用 `.value()` 或对 positional input 调用 `.field()` 的无效状态。

### 16.2 从 `TypeDescriptor` 构造 struct

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub fn construction(self) -> ConstructionDescriptor<'ctx>;

    pub fn construct<T, M, I>(
        self,
        input: I,
    ) -> Result<T, ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: ValueMode,
        I: ConstructionInput<M>;

    pub fn construct_dynamic<M, I>(
        self,
        input: I,
    ) -> Result<DynamicOwned<M>, ConstructionFailure<M>>
    where
        M: ValueMode,
        I: ConstructionInput<M>;
}
```

普通调用依靠返回类型推导 `T`：

```rust
let customer: Customer = TypeDescriptor::of::<Customer>()
    .construct(
        NamedFields::new()
            .field("id", 1001_u64)
            .field("email", String::from("ada@example.com")),
    )?;

let point: Point = TypeDescriptor::of::<Point>()
    .construct(
        PositionalFields::new()
            .value(10_i32)
            .value(20_i32),
    )?;

let marker: Marker =
    TypeDescriptor::of::<Marker>().construct(UnitFields)?;
```

如果 descriptor 不是 struct，返回 `ConstructionError::WrongTargetKind`，不从 enum 类型自动猜测 variant。

### 16.3 构造 enum variant

enum 构造始终从明确的 `VariantDescriptor` 发起：

```rust
impl<'ctx> VariantDescriptor<'ctx> {
    pub fn construction(self) -> ConstructionDescriptor<'ctx>;

    pub fn construct<T, M, I>(
        self,
        input: I,
    ) -> Result<T, ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: ValueMode,
        I: ConstructionInput<M>;

    pub fn construct_dynamic<M, I>(
        self,
        input: I,
    ) -> Result<DynamicOwned<M>, ConstructionFailure<M>>
    where
        M: ValueMode,
        I: ConstructionInput<M>;
}
```

示例：

```rust
let message: Message = TypeDescriptor::of::<Message>()
    .variant("Text")?
    .construct(
        PositionalFields::new()
            .value(String::from("hello")),
    )?;
```

输入形状必须与声明一致：

- named struct/variant 接受 `NamedFields`；
- tuple struct/variant 接受 `PositionalFields`；
- unit struct/variant 接受 `UnitFields`。

### 16.4 构造能力描述

`construction()` 始终返回描述对象，而不是 `Option`，使调用方能够检查不可用原因：

```rust
#[derive(Clone, Copy)]
pub struct ConstructionDescriptor<'ctx> {
    target: ConstructionTarget<'ctx>,
}

pub enum ConstructionTarget<'ctx> {
    Struct(StructType<'ctx>),
    Variant(VariantDescriptor<'ctx>),
}

impl<'ctx> ConstructionDescriptor<'ctx> {
    pub fn target(self) -> ConstructionTarget<'ctx>;
    pub fn shape(self) -> ConstructionShape;
    pub fn availability(self) -> ConstructionAvailability;
    pub fn fields(self) -> ConstructionFields<'ctx>;
}

pub enum ConstructionAvailability {
    Available,
    Unavailable {
        reasons: Box<[ConstructionUnavailableReason]>,
    },
}
```

字段级策略：

```rust
pub enum ConstructionFieldPolicy {
    /// 调用方必须提供值。
    Required,

    /// 调用方可以省略，由生成的 provider 提供默认值。
    Defaulted,

    /// 禁止调用方赋值，只能由生成的 provider 提供。
    ProviderOnly,

    /// 无法安全初始化。
    Unavailable {
        reason: ConstructionUnavailableReason,
    },
}
```

私有字段不会因为宏代码位于类型模块内就自动允许外部调用方写入：

- private + `Required`：安全构造不可用；
- private + `Defaulted` 或 `ProviderOnly`：可以由类型定义方提供的 provider 初始化；
- 显式 unsafe 构造可以绕过可见性，但调用方承担类型不变量。

### 16.5 拥有值更新

更新消费一个拥有所有权的 base value，验证全部 replacement 后生成新值，不对现有对象逐字段原地写入。

```rust
pub struct UpdateInput<M: ValueMode = Local> {
    base: DynamicOwned<M>,
    replacements: NamedFields<M>,
}

impl<M: ValueMode> UpdateInput<M> {
    pub fn new<T: ValueFor<M>>(base: T) -> Self;
    pub fn from_dynamic(base: DynamicOwned<M>) -> Self;

    pub fn field<T: ValueFor<M>>(
        self,
        name: impl Into<Box<str>>,
        value: T,
    ) -> Self;

    pub fn field_dynamic(
        self,
        name: impl Into<Box<str>>,
        value: DynamicOwned<M>,
    ) -> Self;
}
```

调用：

```rust
let updated: Customer = TypeDescriptor::of::<Customer>()
    .update(
        UpdateInput::new(customer)
            .field("email", String::from("new@example.com")),
    )?;
```

动态调用：

```rust
let updated: DynamicOwned<ThreadSafe> = ty.update_dynamic(
    UpdateInput::<ThreadSafe>::from_dynamic(base)
        .field_dynamic("email", replacement),
)?;
```

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub fn update<T, M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ValueMode;

    pub fn update_dynamic<M: ValueMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>>;
}
```

只允许 struct 使用 `update()`；enum variant 切换通过重新构造目标 variant 明确表达。

### 16.6 更新策略

```rust
pub enum UpdateFieldPolicy {
    /// 可以由调用方替换。
    Replaceable,

    /// 保留 base 中的原值，禁止 replacement。
    Preserved,

    /// 整个拥有值更新不可用。
    Unavailable {
        reason: UpdateUnavailableReason,
    },
}
```

adapter 消费输入前必须完成：

1. base 精确类型检查；
2. 目标 struct 检查；
3. replacement 字段存在性检查；
4. 重复字段检查；
5. 字段更新策略检查；
6. 可见性检查；
7. replacement 精确类型检查；
8. 全部 replacement 的整体检查。

因此验证失败不会造成部分更新。

### 16.7 失败恢复

构造和更新分别使用明确的失败类型：

```rust
pub struct ConstructionFailure<M: ValueMode> {
    error: ConstructionError,
    recovery: ConstructionRecovery<M>,
}

pub struct UpdateFailure<M: ValueMode> {
    error: UpdateError,
    recovery: UpdateRecovery<M>,
}
```

构造恢复保留全部输入及原顺序：

```rust
impl<M: ValueMode> ConstructionFailure<M> {
    pub fn error(&self) -> &ConstructionError;
    pub fn recovery(&self) -> &ConstructionRecovery<M>;

    pub fn into_parts(
        self,
    ) -> (ConstructionError, ConstructionRecovery<M>);

    pub fn into_input(self) -> RecoveredConstructionInput<M>;
}
```

更新恢复保留 base 和全部 replacement：

```rust
impl<M: ValueMode> UpdateFailure<M> {
    pub fn error(&self) -> &UpdateError;
    pub fn recovery(&self) -> &UpdateRecovery<M>;

    pub fn into_parts(
        self,
    ) -> (UpdateError, UpdateRecovery<M>);

    pub fn into_input(self) -> UpdateInput<M>;
}
```

恢复保证只覆盖生成 adapter 执行前的拒绝。adapter 开始执行后，不承诺恢复已消费值，不回滚 provider 副作用；provider 或用户代码 panic 时遵循普通 Rust panic 语义。

### 16.8 unsafe 入口

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub unsafe fn construct_unchecked<T, M, I>(
        self,
        input: I,
    ) -> Result<T, ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: ValueMode,
        I: ConstructionInput<M>;

    pub unsafe fn update_unchecked<T, M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ValueMode;
}
```

`VariantDescriptor` 同样提供 `construct_unchecked()`。这些入口只绕过源码可见性和由可见性保护的类型不变量，仍执行形状、字段、重复项和精确类型检查。

## 17. Capability API

Capability 用于给类型附加经过类型检查的可执行扩展能力，而不把所有扩展硬编码进 `TypeDescriptor`。典型能力包括动态 `Clone`、动态 `Default`、`Send`/`Sync` 标记和显式 receiver adapter。

### 17.1 capability 契约

使用类型表达 capability 契约，不要求普通调用方构造和传递 `CapabilityKey<A>`：

```rust
pub trait CapabilitySpec: 'static {
    type Adapter: Send + Sync + 'static;

    /// 在反射生态中稳定标识该能力。
    const ID: &'static str;
}
```

用户定义能力：

```rust
pub struct ValidateCapability;

impl CapabilitySpec for ValidateCapability {
    type Adapter = ValidateAdapter;
    const ID: &'static str = "example.validate";
}
```

该模型保证 capability ID 和 adapter 类型不会被调用方错误配对，并让 built-in 与自定义 capability 使用同一查询方式。

### 17.2 context-bound capability handle

```rust
#[derive(Clone, Copy)]
pub struct Capability<'ctx, C: CapabilitySpec> {
    target: CapabilityTarget<'ctx>,
    adapter: &'ctx C::Adapter,
    origin: CapabilityOrigin<'ctx>,
}

pub enum CapabilityTarget<'ctx> {
    Type(TypeDescriptor<'ctx>),
    Definition(TypeDefinitionDescriptor<'ctx>),
}

impl<'ctx, C: CapabilitySpec> Capability<'ctx, C> {
    pub fn target(self) -> CapabilityTarget<'ctx>;
    pub fn adapter(self) -> &'ctx C::Adapter;
    pub fn origin(self) -> CapabilityOrigin<'ctx>;
}
```

Capability 自动绑定查询它的 context，不接受额外 registry/context 参数。

### 17.3 类型 capability 查询

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub fn find_capability<C: CapabilitySpec>(
        self,
    ) -> Result<Option<Capability<'ctx, C>>, CapabilityError>;

    pub fn capability<C: CapabilitySpec>(
        self,
    ) -> Result<Capability<'ctx, C>, CapabilityError>;

    pub fn has_capability<C: CapabilitySpec>(
        self,
    ) -> Result<bool, CapabilityError>;

    /// 用于调试、管理界面和框架枚举。
    pub fn capabilities(
        self,
    ) -> Result<Capabilities<'ctx>, CapabilityError>;
}
```

示例：

```rust
if customer_type.has_capability::<ValidateCapability>()? {
    let capability =
        customer_type.capability::<ValidateCapability>()?;

    capability.adapter().validate_dynamic(&customer)?;
}
```

缺失、contract mismatch 和冲突必须区分：

```rust
#[non_exhaustive]
pub enum CapabilityError {
    Context(ContextError),

    Missing {
        target: CapabilityTargetId,
        capability_id: &'static str,
    },

    ContractMismatch {
        capability_id: &'static str,
        expected_adapter: TypeId,
        actual_adapter: TypeId,
        origin: RegistrationOrigin,
    },

    Conflict {
        capability_id: &'static str,
        registrations: Box<[RegistrationOrigin]>,
    },
}
```

如果 ID 存在但 adapter 类型错误，不能将其当作 capability 缺失，否则会掩盖注册错误。

### 17.4 capability 枚举

泛型 `Capability<'ctx, C>` 用于类型安全调用；类型擦除的 descriptor 只用于发现和诊断：

```rust
pub struct Capabilities<'ctx> {
    // context-bound view
}

impl<'ctx> Capabilities<'ctx> {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;

    pub fn get(
        &self,
        id: &str,
    ) -> Option<CapabilityDescriptor<'ctx>>;

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = CapabilityDescriptor<'ctx>> + '_;
}

#[derive(Clone, Copy)]
pub struct CapabilityDescriptor<'ctx> {
    // erased metadata
}

impl<'ctx> CapabilityDescriptor<'ctx> {
    pub fn id(self) -> &'static str;
    pub fn adapter_type_id(self) -> TypeId;
    pub fn target(self) -> CapabilityTarget<'ctx>;
    pub fn origin(self) -> CapabilityOrigin<'ctx>;

    pub fn downcast<C: CapabilitySpec>(
        self,
    ) -> Result<Capability<'ctx, C>, CapabilityContractError>;
}
```

类型擦除 descriptor 不直接执行 adapter，必须先按 `CapabilitySpec` 恢复类型契约。

### 17.5 capability 来源与解析

```rust
pub enum CapabilityOrigin<'ctx> {
    /// 由类型自己的 Reflect metadata 提供。
    Intrinsic {
        target: TypeDescriptor<'ctx>,
    },

    /// 来自链接目录中的外部注册。
    Linked {
        registration: RegistrationOrigin<'ctx>,
    },

    /// 来自显式 ReflectionContextBuilder。
    Explicit {
        registration: RegistrationOrigin<'ctx>,
    },
}
```

解析规则：

- intrinsic capability 属于类型自身；
- linked 和 explicit capability 属于当前 context；
- 显式 context 不回退查询全局 context；
- 同一 target 上一个 capability ID 只能有一个有效提供者；
- 不定义隐式优先级，不允许 external registration 静默覆盖 intrinsic capability；
- 重复或冲突注册在 context 构建时失败；
- 未进入发现索引但通过 `type_of::<T>()` 获得的类型，其 intrinsic capability 仍可用；
- 该类型只能观察当前 context 中明确包含的 external capability。

### 17.6 built-in capability

```rust
pub struct SendCapability;
pub struct SyncCapability;
pub struct CloneCapability;
pub struct DefaultCapability;

impl CapabilitySpec for SendCapability {
    type Adapter = ();
    const ID: &'static str = "qubit.reflect.send";
}

impl CapabilitySpec for SyncCapability {
    type Adapter = ();
    const ID: &'static str = "qubit.reflect.sync";
}

impl CapabilitySpec for CloneCapability {
    type Adapter = CloneAdapter;
    const ID: &'static str = "qubit.reflect.clone";
}

impl CapabilitySpec for DefaultCapability {
    type Adapter = DefaultAdapter;
    const ID: &'static str = "qubit.reflect.default";
}
```

`Send` 和 `Sync` 是 marker capability，使用 `()` adapter，不再需要 fact-only 状态。

动态 Clone：

```rust
let clone = ty.capability::<CloneCapability>()?;
let copied: DynamicOwned<Local> =
    clone.adapter().clone_dynamic(value)?;
```

动态 Default：

```rust
let default = ty.capability::<DefaultCapability>()?;
let value: DynamicOwned<Local> =
    default.adapter().create_dynamic();
```

adapter 支持 `Local`/`ThreadSafe` 模式：

```rust
impl CloneAdapter {
    pub fn clone_dynamic<M: ValueMode>(
        &self,
        value: DynamicRef<'_, M>,
    ) -> Result<DynamicOwned<M>, CapabilityInvocationError>;
}

impl DefaultAdapter {
    pub fn create_dynamic<M: ValueMode>(
        &self,
    ) -> DynamicOwned<M>;
}
```

生成 adapter 时必须由 Rust trait bound 证明相应能力，不能仅根据字符串声称某类型实现了 `Clone`、`Default`、`Send` 或 `Sync`。

### 17.7 泛型定义 capability

某些 capability 描述泛型定义本身，而不是具体 monomorph：

```rust
impl<'ctx> TypeDefinitionDescriptor<'ctx> {
    pub fn find_capability<C: CapabilitySpec>(
        self,
    ) -> Result<Option<Capability<'ctx, C>>, CapabilityError>;

    pub fn capability<C: CapabilitySpec>(
        self,
    ) -> Result<Capability<'ctx, C>, CapabilityError>;

    pub fn capabilities(
        self,
    ) -> Result<Capabilities<'ctx>, CapabilityError>;
}
```

定义级 capability 不自动成为每个具体类型实例的 capability。需要继承或实例化时，由该 capability 自己的 adapter 明确完成，核心层不猜测传播规则。

### 17.8 注册 API

```rust
pub struct CapabilityRegistration {
    // opaque typed fact
}

impl CapabilityRegistration {
    pub fn for_type<T, C>(
        adapter: C::Adapter,
    ) -> Self
    where
        T: Reflect + ?Sized,
        C: CapabilitySpec;

    pub fn for_definition<C>(
        definition: TypeDefinitionId,
        adapter: C::Adapter,
    ) -> Self
    where
        C: CapabilitySpec;
}
```

显式 context：

```rust
let context = ReflectionContext::builder()
    .include_type::<Customer>()
    .register(CapabilityRegistration::for_type::<
        Customer,
        ValidateCapability,
    >(ValidateAdapter::new()))
    .build()?;
```

需要进入 linked context 的 capability 使用独立的类型化注册宏：

```rust
register_capability!(Customer: CloneCapability);
register_capability!(Customer: DefaultCapability);
```

该宏生成经过 Rust trait bound 验证的 registration fragment，不能伪造未实现的 trait。capability 注册不属于类型定义 metadata，也不是 `#[Reflect]` 的 helper attribute。

## 18. 泛型定义与类型表达式

### 18.1 三个层次

泛型反射明确区分三个层次。以以下定义为例：

```rust
struct Envelope<T, const N: usize> {
    payload: T,
    bytes: [u8; N],
}
```

- `TypeDefinitionDescriptor` 描述源码中的 `Envelope<T, N>` 声明；
- `TypeExpression` 描述 `T`、`[u8; N]`、`Vec<T>` 等源码级类型表达式；
- `TypeDescriptor` 描述已经由 Rust 编译器产生的具体类型，例如 `Envelope<String, 16>`。

```text
Envelope<T, N>                 TypeDefinitionDescriptor
├── T                          TypeExpression::Parameter
├── [u8; N]                    TypeExpression::Array
└── Envelope<String, 16>       TypeDescriptor
```

`TypeDefinitionDescriptor` 只描述泛型声明，不能凭运行时参数要求 Rust 生成一个从未编译过的单态化类型。

### 18.2 `TypeDescriptor` 与类型定义

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    /// 对应的源码级名义类型定义。
    ///
    /// tuple、数组、slice、引用、函数指针等没有独立源码声明的类型
    /// 返回 None。
    pub fn find_definition(self) -> Option<TypeDefinitionDescriptor<'ctx>>;

    pub fn definition(
        self,
    ) -> Result<TypeDefinitionDescriptor<'ctx>, ReflectionError>;

    /// 当前具体类型应用的泛型参数。
    ///
    /// 非泛型类型返回空集合。
    pub fn generic_arguments(self) -> GenericArguments<'ctx>;
}
```

示例：

```rust
let ty = TypeDescriptor::of::<Envelope<String, 16>>();

let definition = ty.definition()?;
assert_eq!(definition.name(), "Envelope");
assert_eq!(definition.path(), "my_crate::model::Envelope");

let arguments = ty.generic_arguments();

let item = arguments.argument("T")?.type_argument()?;
assert_eq!(item.concrete_type()?.type_id(), TypeId::of::<String>());

let length = arguments.argument("N")?.const_argument()?;
assert_eq!(
    length.expression(),
    ConstExpression::UnsignedInteger(16),
);
```

### 18.3 `TypeDefinitionDescriptor`

类型定义 descriptor 同样绑定其来源 context：

```rust
#[derive(Clone, Copy)]
pub struct TypeDefinitionDescriptor<'ctx> {
    // opaque metadata + context
}

impl<'ctx> TypeDefinitionDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;

    pub fn id(self) -> TypeDefinitionId;

    /// 源码中的短名称，例如 Envelope。
    pub fn name(self) -> &'static str;

    /// 完整反射名，例如 my_crate::model::Envelope。
    pub fn path(self) -> &'static str;

    pub fn kind(self) -> TypeDefinitionKind;

    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx>;

    /// 仅 struct 定义返回字段定义。
    pub fn fields(self) -> Option<FieldDefinitions<'ctx>>;

    pub fn field(
        self,
        name: &str,
    ) -> Result<FieldDefinitionDescriptor<'ctx>, LookupError>;

    /// 仅 enum 定义返回 variant 定义。
    pub fn variants(self) -> Option<VariantDefinitions<'ctx>>;

    pub fn variant(
        self,
        name: &str,
    ) -> Result<VariantDefinitionDescriptor<'ctx>, LookupError>;

    /// 当前 context 中实际存在的已编译具体应用。
    pub fn instances(self) -> Types<'ctx>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeDefinitionKind {
    Struct(StructKind),
    Enum,
    Union,
    Opaque,
}
```

定义级字段返回类型表达式，具体类型字段返回具体类型 descriptor：

```rust
let definition = TypeDescriptor::of::<Envelope<String, 16>>()
    .definition()?;

let payload_definition = definition.field("payload")?;
assert!(matches!(
    payload_definition.ty().kind(),
    TypeExpressionKind::Parameter(_),
));

let concrete = TypeDescriptor::of::<Envelope<String, 16>>();
let payload = concrete.field("payload")?;
assert_eq!(
    payload.concrete_type()?.type_id(),
    TypeId::of::<String>(),
);
```

```rust
impl<'ctx> FieldDefinitionDescriptor<'ctx> {
    pub fn name(self) -> &'static str;
    pub fn index(self) -> usize;
    pub fn visibility(self) -> Visibility;
    pub fn ty(self) -> TypeExpression<'ctx>;
}

impl<'ctx> VariantDefinitionDescriptor<'ctx> {
    pub fn name(self) -> &'static str;
    pub fn index(self) -> usize;
    pub fn fields(self) -> FieldDefinitions<'ctx>;
    pub fn discriminant(self) -> Option<VariantDiscriminant>;
}
```

### 18.4 泛型定义与参数

```rust
#[derive(Clone, Copy)]
pub struct GenericDefinitionDescriptor<'ctx> {
    // opaque
}

impl<'ctx> GenericDefinitionDescriptor<'ctx> {
    /// 按源码声明顺序返回 lifetime、type 和 const 参数。
    pub fn parameters(self) -> GenericParameters<'ctx>;

    pub fn parameter(
        self,
        name: &str,
    ) -> Result<GenericParameterDescriptor<'ctx>, LookupError>;

    pub fn predicates(self) -> Predicates<'ctx>;

    pub fn is_empty(self) -> bool;
}
```

```rust
#[derive(Clone, Copy)]
pub struct GenericParameterDescriptor<'ctx> {
    // opaque
}

impl<'ctx> GenericParameterDescriptor<'ctx> {
    pub fn name(self) -> &'static str;
    pub fn index(self) -> usize;
    pub fn kind(self) -> GenericParameterKind<'ctx>;
}

pub enum GenericParameterKind<'ctx> {
    Lifetime {
        bounds: Lifetimes<'ctx>,
    },

    Type {
        bounds: Predicates<'ctx>,
        default: Option<TypeExpression<'ctx>>,
    },

    Const {
        ty: TypeExpression<'ctx>,
        default: Option<ConstExpression<'ctx>>,
    },
}
```

`where` 子句和参数上的约束统一表示为结构化 predicate：

```rust
pub struct PredicateDescriptor<'ctx> {
    // opaque
}

pub enum PredicateKind<'ctx> {
    TraitBound {
        ty: TypeExpression<'ctx>,
        trait_: TraitExpression<'ctx>,
        modifier: TraitBoundModifier,
    },

    LifetimeOutlives {
        lifetime: LifetimeExpression<'ctx>,
        outlives: LifetimeExpression<'ctx>,
    },

    TypeOutlives {
        ty: TypeExpression<'ctx>,
        lifetime: LifetimeExpression<'ctx>,
    },

    TypeEquality {
        left: TypeExpression<'ctx>,
        right: TypeExpression<'ctx>,
    },

    ConstEvaluatable {
        expression: ConstExpression<'ctx>,
    },
}
```

这些 predicate 是源码事实，不承诺在运行时重新执行 Rust trait solver。

### 18.5 具体泛型参数

```rust
#[derive(Clone, Copy)]
pub struct GenericArguments<'ctx> {
    // opaque
}

impl<'ctx> GenericArguments<'ctx> {
    /// 只包含参与运行时具体类型身份的 type 和 const 参数。
    ///
    /// lifetime 参数不在此集合中。
    pub fn iter(self) -> impl ExactSizeIterator<
        Item = GenericArgumentDescriptor<'ctx>,
    > + 'ctx;

    pub fn get(
        self,
        name: &str,
    ) -> Option<GenericArgumentDescriptor<'ctx>>;

    pub fn argument(
        self,
        name: &str,
    ) -> Result<GenericArgumentDescriptor<'ctx>, LookupError>;

    pub fn is_empty(self) -> bool;
}

pub enum GenericArgumentDescriptor<'ctx> {
    Type(TypeArgumentDescriptor<'ctx>),
    Const(ConstArgumentDescriptor<'ctx>),
}

impl<'ctx> GenericArgumentDescriptor<'ctx> {
    pub fn type_argument(
        self,
    ) -> Result<TypeArgumentDescriptor<'ctx>, ReflectionError>;

    pub fn const_argument(
        self,
    ) -> Result<ConstArgumentDescriptor<'ctx>, ReflectionError>;
}
```

类型参数同时保留结构表达式和可用的准确 descriptor：

```rust
impl<'ctx> TypeArgumentDescriptor<'ctx> {
    pub fn parameter(self) -> GenericParameterDescriptor<'ctx>;

    pub fn expression(self) -> TypeExpression<'ctx>;

    /// 当该参数具有可导航的反射 metadata 时返回准确类型。
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>>;

    pub fn concrete_type(
        self,
    ) -> Result<TypeDescriptor<'ctx>, ReflectionError>;
}
```

`find_concrete_type()` 返回 `Option`，因为泛型类型可能仅要求 `T: 'static` 而没有要求 `T: Reflect`。框架可以知道参数在结构上是什么，但不一定拥有 `T` 的完整反射 descriptor。

const 参数：

```rust
impl<'ctx> ConstArgumentDescriptor<'ctx> {
    pub fn parameter(self) -> GenericParameterDescriptor<'ctx>;

    pub fn expression(self) -> ConstExpression<'ctx>;

    /// 若该 const 参数可以安全进入动态值边界，则构造一个新的值。
    pub fn value<M: ValueMode>(
        self,
    ) -> Result<Option<DynamicOwned<M>>, ReflectionError>;
}
```

### 18.6 lifetime 的处理

lifetime 参数只属于定义级反射：

```rust
struct Borrowed<'a, T> {
    value: &'a T,
}
```

```rust
let definition = TypeDescriptor::of::<Borrowed<'static, String>>()
    .definition()?;

let lifetime = definition.generics().parameter("a")?;
assert!(matches!(
    lifetime.kind(),
    GenericParameterKind::Lifetime { .. },
));
```

但 lifetime 不进入具体泛型参数集合：

```rust
let arguments =
    TypeDescriptor::of::<Borrowed<'static, String>>().generic_arguments();

assert!(arguments.get("a").is_none());
assert!(arguments.get("T").is_some());
```

lifetime 在运行时被擦除，API 不会把 `'static` 或某个局部 lifetime 伪装成可靠的运行时类型参数。

### 18.7 `TypeExpression`

`TypeExpression` 是绑定 context 的只读句柄，不直接暴露宏内部 AST：

```rust
#[derive(Clone, Copy)]
pub struct TypeExpression<'ctx> {
    // opaque
}

impl<'ctx> TypeExpression<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;
    pub fn kind(self) -> TypeExpressionKind<'ctx>;

    /// 用于日志和错误信息，不参与身份判断。
    pub fn display(self) -> impl Display + 'ctx;
}
```

核心形状：

```rust
pub enum TypeExpressionKind<'ctx> {
    Named(NamedTypeExpression<'ctx>),
    Parameter(GenericParameterDescriptor<'ctx>),
    SelfType,

    AssociatedType(AssociatedTypeExpression<'ctx>),

    Reference {
        lifetime: Option<LifetimeExpression<'ctx>>,
        mutable: bool,
        target: TypeExpression<'ctx>,
    },

    RawPointer {
        mutable: bool,
        target: TypeExpression<'ctx>,
    },

    Slice(TypeExpression<'ctx>),

    Array {
        element: TypeExpression<'ctx>,
        length: ConstExpression<'ctx>,
    },

    Tuple(TypeExpressions<'ctx>),

    FunctionPointer(FunctionPointerExpression<'ctx>),
    TraitObject(TraitObjectExpression<'ctx>),
    Opaque(OpaqueTypeExpression<'ctx>),

    Never,
}
```

名义类型表达式使用定义身份，而不是仅依靠字符串：

```rust
impl<'ctx> NamedTypeExpression<'ctx> {
    /// 对于当前 context 已知的反射定义返回 Some。
    pub fn find_definition(self) -> Option<TypeDefinitionDescriptor<'ctx>>;

    /// 始终保留原始完整 Rust 路径。
    pub fn path(self) -> &'static str;

    pub fn arguments(self) -> GenericExpressionArguments<'ctx>;
}
```

关联类型投影能够表达 `<T as Iterator>::Item`：

```rust
impl<'ctx> AssociatedTypeExpression<'ctx> {
    pub fn self_type(self) -> TypeExpression<'ctx>;
    pub fn trait_(self) -> TraitExpression<'ctx>;
    pub fn name(self) -> &'static str;
}
```

### 18.8 不提供任意运行时单态化

不提供容易产生错误预期的 API：

```rust
// 不提供
definition.apply([String, 16]) -> TypeDescriptor
```

Rust 反射库不能在运行时让编译器产生一个新的 `Envelope<String, 16>`。API 只允许发现当前程序中已经生成并登记的具体应用：

```rust
let definition = context
    .type_definition_by_name("my_crate::model::Envelope")?;

for instance in definition.instances() {
    println!("{}", instance.rust_type_name());
}
```

若具体类型在编译期已知，直接使用：

```rust
let ty = context.type_of::<Envelope<String, 16>>();
```

这也与之前确认的原则一致：按需产生数组、引用、tuple 等复合 descriptor，不代表向不可变 context 的发现索引中反向登记新类型。

### 18.9 类型定义查找

类型定义同样可能因多个 crate 版本而出现相同完整路径：

```rust
impl ReflectionContext {
    pub fn type_definitions(
        &self,
    ) -> Result<TypeDefinitions<'_>, ContextError>;

    pub fn find_type_definition_by_id(
        &self,
        id: TypeDefinitionId,
    ) -> Result<Option<TypeDefinitionDescriptor<'_>>, ContextError>;

    pub fn type_definition_by_id(
        &self,
        id: TypeDefinitionId,
    ) -> Result<TypeDefinitionDescriptor<'_>, LookupError>;

    pub fn lookup_type_definition_by_name(
        &self,
        path: &str,
    ) -> Result<Lookup<TypeDefinitionDescriptor<'_>>, ContextError>;

    pub fn type_definition_by_name(
        &self,
        path: &str,
    ) -> Result<TypeDefinitionDescriptor<'_>, LookupError>;

    pub fn type_definitions_by_short_name(
        &self,
        name: &str,
    ) -> Result<TypeDefinitions<'_>, ContextError>;
}
```

名称规则继续沿用已经确认的约定：

- `type_definition_by_name()` 使用完整原始反射名并要求结果唯一；
- `lookup_type_definition_by_name()` 保留完整查找状态；
- 不支持反射重命名；
- 短名称查询显式使用 `type_definitions_by_short_name()`；
- 复数集合查询不隐式要求唯一。

## 19. Trait 与 Impl Descriptor

### 19.1 定义与具体应用

trait 和 impl 都区分源码定义与具体应用：

```text
trait Repository<T>                 TraitDefinitionDescriptor
Repository<Customer>               TraitDescriptor

impl<T> Repository<T> for Store<T>  ImplDefinitionDescriptor
impl Repository<Customer>
    for Store<Customer>             ImplDescriptor
```

```rust
pub struct TraitDefinitionDescriptor<'ctx> {
    // opaque metadata + context
}

pub struct TraitDescriptor<'ctx> {
    // applied trait metadata + context
}

pub struct ImplDefinitionDescriptor<'ctx> {
    // opaque metadata + context
}

pub struct ImplDescriptor<'ctx> {
    // concrete impl metadata + context
}
```

- `TraitDefinitionDescriptor` 表示源码中的 trait 声明；
- `TraitDescriptor` 表示应用了具体泛型参数的 trait，例如 `From<String>`；
- `ImplDefinitionDescriptor` 表示源码中的某个 `impl` block；
- `ImplDescriptor` 表示该 impl 在具体目标类型上的已编译应用。

四者全部绑定 `ReflectionContext`。

### 19.2 Trait 定义

```rust
impl<'ctx> TraitDefinitionDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;
    pub fn id(self) -> TraitDefinitionId;

    pub fn name(self) -> &'static str;
    pub fn path(self) -> &'static str;
    pub fn visibility(self) -> Visibility;

    pub fn completeness(self) -> TraitCompleteness;
    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx>;

    /// 源码中声明的直接 supertrait 表达式。
    pub fn supertraits(self) -> TraitExpressions<'ctx>;

    pub fn methods(self) -> MethodDefinitions<'ctx>;

    pub fn method(
        self,
        name: &str,
    ) -> Result<MethodDefinitionDescriptor<'ctx>, LookupError>;

    pub fn associated_types(
        self,
    ) -> AssociatedTypeDefinitions<'ctx>;

    pub fn associated_type(
        self,
        name: &str,
    ) -> Result<AssociatedTypeDefinitionDescriptor<'ctx>, LookupError>;

    pub fn associated_consts(
        self,
    ) -> AssociatedConstDefinitions<'ctx>;

    pub fn associated_const(
        self,
        name: &str,
    ) -> Result<AssociatedConstDefinitionDescriptor<'ctx>, LookupError>;

    /// 当前 context 中已经登记的具体 trait application。
    pub fn applications(
        self,
    ) -> Result<Traits<'ctx>, ContextError>;
}
```

trait 内不允许出现两个同名方法或两个同名关联项，因此这里不需要 `Lookup`；单数 accessor 返回唯一项或错误，可选查询通过对应集合的 `.get()` 完成。

### 19.3 完整与不完整 trait

允许描述没有使用 `#[Reflect]` 的外部 trait：

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraitCompleteness {
    /// trait 声明、supertrait 和关联项均可导航。
    Complete,

    /// 只知道身份和路径，不能假设拥有完整成员信息。
    ExternalIncomplete,
}
```

例如，反射系统可以记录第三方 trait 的具体 impl：

```rust
impl ExternalTrait for Customer {
    // ...
}
```

如果没有 `ExternalTrait` 的完整反射 metadata：

```rust
trait_definition.completeness()
    == TraitCompleteness::ExternalIncomplete
```

此时：

- `methods()`、`associated_types()` 等只能返回明确登记的事实；
- 不推断未登记的关联项；
- 不声称拥有完整 supertrait closure；
- 不运行 Rust trait solver 补全信息。

### 19.4 方法定义与有效方法

定义级方法使用独立 descriptor：

```rust
#[derive(Clone, Copy)]
pub struct MethodDefinitionDescriptor<'ctx> {
    // source declaration metadata + context
}
```

它只描述源码声明：

```rust
impl<'ctx> MethodDefinitionDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;

    pub fn name(self) -> &'static str;
    pub fn visibility(self) -> Visibility;

    pub fn owner(self) -> MethodDefinitionOwner<'ctx>;
    pub fn receiver(self) -> Option<ReceiverDefinitionDescriptor<'ctx>>;

    pub fn parameters(self) -> ParameterDefinitions<'ctx>;
    pub fn output(self) -> ReturnDefinitionDescriptor<'ctx>;
    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx>;

    pub fn qualifiers(self) -> MethodQualifiers;
    pub fn has_default_body(self) -> bool;
}

pub enum MethodDefinitionOwner<'ctx> {
    Trait(TraitDefinitionDescriptor<'ctx>),
    Impl(ImplDefinitionDescriptor<'ctx>),
}
```

前文定义的 `MethodDescriptor` 仍表示具体类型上的最终有效方法：

```rust
impl<'ctx> MethodDescriptor<'ctx> {
    pub fn definition(self) -> MethodDefinitionDescriptor<'ctx>;

    pub fn target(self) -> TypeDescriptor<'ctx>;
    pub fn namespace(self) -> MethodNamespace<'ctx>;

    pub fn implementation_source(self) -> MethodImplementationSource;
}
```

```text
MethodDefinitionDescriptor
        │
        ├── trait 默认实现
        ├── impl 显式覆盖
        └── 具体泛型特化
                │
                ▼
        MethodDescriptor
```

调用只发生在 `MethodDescriptor` 或 `MethodSpecializationDescriptor` 上，定义级 descriptor 不提供 `invoke()`。

### 19.5 具体 Trait application

```rust
impl<'ctx> TraitDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;

    pub fn definition(self) -> TraitDefinitionDescriptor<'ctx>;
    pub fn arguments(self) -> GenericArguments<'ctx>;

    pub fn name(self) -> &'static str;
    pub fn path(self) -> &'static str;

    /// 已经代入具体泛型参数的直接 supertrait。
    pub fn direct_supertraits(self) -> Traits<'ctx>;

    /// 去重后的完整传递闭包；对于不完整外部 trait 可能不完整。
    pub fn all_supertraits(self) -> Traits<'ctx>;

    pub fn supertrait_completeness(
        self,
    ) -> SupertraitCompleteness;
}
```

`TraitDescriptor` 表示 `Repository<Customer>`、`From<String>` 或 `Iterator`。它不绑定目标类型。`From<String>` 和 `From<u8>` 是不同的具体 application，但共享一个 `TraitDefinitionDescriptor`。

关联类型不作为 trait application 的泛型身份：

```text
Iterator                         trait application
Iterator<Item = String>          trait expression / bound
```

- `Iterator` 的具体 impl 通过 `ImplDescriptor` 保存 `Item = String`；
- `dyn Iterator<Item = String>` 中的 `Item = String` 由 `TraitExpression` 保存；
- 关联类型绑定不混入 trait impl 的一致性身份。

### 19.6 Trait 表达式

源码中出现的 trait bound 使用结构化表达式：

```rust
#[derive(Clone, Copy)]
pub struct TraitExpression<'ctx> {
    // opaque
}

impl<'ctx> TraitExpression<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;

    pub fn find_definition(
        self,
    ) -> Option<TraitDefinitionDescriptor<'ctx>>;

    pub fn path(self) -> &'static str;
    pub fn arguments(self) -> GenericExpressionArguments<'ctx>;

    pub fn associated_type_constraints(
        self,
    ) -> AssociatedTypeConstraints<'ctx>;
}
```

它能够表达：

```rust
Iterator<Item = String>
for<'a> Fn(&'a str) -> bool
Service<Request, Error = ServiceError>
```

与 `TypeExpression` 一样，`TraitExpression` 描述源码事实，不表示运行时重新求解 trait bound。

### 19.7 关联类型与关联常量定义

```rust
impl<'ctx> AssociatedTypeDefinitionDescriptor<'ctx> {
    pub fn name(self) -> &'static str;
    pub fn visibility(self) -> Visibility;

    pub fn bounds(self) -> Predicates<'ctx>;
    pub fn default(self) -> Option<TypeExpression<'ctx>>;
}

impl<'ctx> AssociatedConstDefinitionDescriptor<'ctx> {
    pub fn name(self) -> &'static str;
    pub fn visibility(self) -> Visibility;

    pub fn ty(self) -> TypeExpression<'ctx>;
    pub fn default(self) -> Option<ConstExpression<'ctx>>;
}
```

这些对象只描述 trait 声明中的定义，不代表某个具体 impl 最终采用的值。

### 19.8 Impl 定义

```rust
impl<'ctx> ImplDefinitionDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;
    pub fn id(self) -> ImplDefinitionId;

    pub fn kind(self) -> ImplKind;

    /// 可能含有 T、关联类型等符号参数。
    pub fn target(self) -> TypeExpression<'ctx>;

    /// inherent impl 返回 None。
    pub fn implemented_trait(
        self,
    ) -> Option<TraitExpression<'ctx>>;

    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx>;
    pub fn qualifiers(self) -> ImplQualifiers;

    /// 只包含这个 impl block 中显式声明的方法。
    pub fn methods(self) -> MethodDefinitions<'ctx>;

    pub fn method(
        self,
        name: &str,
    ) -> Result<MethodDefinitionDescriptor<'ctx>, LookupError>;

    /// 此 impl block 显式提供的关联类型定义。
    pub fn associated_types(
        self,
    ) -> ImplAssociatedTypeDefinitions<'ctx>;

    /// 此 impl block 显式提供的关联常量定义。
    pub fn associated_consts(
        self,
    ) -> ImplAssociatedConstDefinitions<'ctx>;

    /// 当前 context 中登记的具体应用。
    pub fn instances(
        self,
    ) -> Result<Impls<'ctx>, ContextError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImplKind {
    Inherent,
    Trait,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImplQualifiers {
    pub is_unsafe: bool,
}
```

impl block 没有 Rust 名称，因此不提供 `impl_definition_by_name()`。`ImplDefinitionId` 是生成代码提供的精确源码 fragment 身份，用于诊断、关联具体应用和检测重复注册。

### 19.9 具体 Impl

```rust
impl<'ctx> ImplDescriptor<'ctx> {
    pub fn context(self) -> &'ctx ReflectionContext;

    pub fn definition(self) -> ImplDefinitionDescriptor<'ctx>;
    pub fn kind(self) -> ImplKind;

    pub fn target(self) -> TypeDescriptor<'ctx>;

    /// inherent impl 返回 None。
    pub fn implemented_trait(
        self,
    ) -> Option<TraitDescriptor<'ctx>>;

    /// impl 自身泛型参数在本次具体应用中的绑定。
    pub fn arguments(self) -> GenericArguments<'ctx>;

    /// 包含 trait 默认方法在内的最终有效方法。
    pub fn methods(self) -> Methods<'ctx>;

    pub fn method(
        self,
        name: &str,
    ) -> Result<MethodDescriptor<'ctx>, LookupError>;

    pub fn associated_types(
        self,
    ) -> AssociatedTypeBindings<'ctx>;

    pub fn associated_type(
        self,
        name: &str,
    ) -> Result<AssociatedTypeBindingDescriptor<'ctx>, LookupError>;

    pub fn associated_consts(
        self,
    ) -> AssociatedConstBindings<'ctx>;

    pub fn associated_const(
        self,
        name: &str,
    ) -> Result<AssociatedConstBindingDescriptor<'ctx>, LookupError>;
}
```

`ImplDescriptor::methods()` 返回最终有效方法，包括：

- inherent impl 中声明的方法；
- trait impl 显式实现的方法；
- trait 提供且没有被覆盖的默认方法；
- 必需但没有可调用实现的方法仍可作为不可调用 descriptor 出现，以供诊断。

### 19.10 关联项最终绑定

```rust
impl<'ctx> AssociatedTypeBindingDescriptor<'ctx> {
    pub fn definition(
        self,
    ) -> AssociatedTypeDefinitionDescriptor<'ctx>;

    pub fn expression(self) -> TypeExpression<'ctx>;

    pub fn find_concrete_type(
        self,
    ) -> Option<TypeDescriptor<'ctx>>;

    pub fn source(self) -> AssociatedItemSource;
}

impl<'ctx> AssociatedConstBindingDescriptor<'ctx> {
    pub fn definition(
        self,
    ) -> AssociatedConstDefinitionDescriptor<'ctx>;

    pub fn expression(self) -> ConstExpression<'ctx>;

    pub fn value<M: ValueMode>(
        self,
    ) -> Result<Option<DynamicOwned<M>>, ReflectionError>;

    pub fn source(self) -> AssociatedItemSource;
}

pub enum AssociatedItemSource {
    /// impl 显式提供。
    Declared,

    /// 使用 trait 默认值。
    Defaulted,

    /// impl 覆盖 trait 默认值。
    Overridden,

    /// trait 要求该项，但没有可用绑定。
    RequiredButUnavailable,
}
```

### 19.11 从具体类型导航 impl

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub fn impls(
        self,
    ) -> Result<Impls<'ctx>, ContextError>;

    pub fn inherent_impls(
        self,
    ) -> Result<Impls<'ctx>, ContextError>;

    pub fn trait_impls(
        self,
    ) -> Result<Impls<'ctx>, ContextError>;

    /// 按精确 trait application 查询，例如精确查找 From<String>。
    pub fn find_trait_impl(
        self,
        trait_: TraitDescriptor<'ctx>,
    ) -> Result<Option<ImplDescriptor<'ctx>>, ContextError>;

    pub fn trait_impl(
        self,
        trait_: TraitDescriptor<'ctx>,
    ) -> Result<ImplDescriptor<'ctx>, LookupError>;

    /// 按 trait 定义查询，例如查询所有 From<_>，因此可能返回多个。
    pub fn lookup_trait_impl_by_definition(
        self,
        trait_: TraitDefinitionDescriptor<'ctx>,
    ) -> Result<Lookup<ImplDescriptor<'ctx>>, ContextError>;

    /// 要求该类型对指定 trait 定义恰好存在一个 application。
    pub fn trait_impl_by_definition(
        self,
        trait_: TraitDefinitionDescriptor<'ctx>,
    ) -> Result<ImplDescriptor<'ctx>, LookupError>;
}
```

同一类型可以合法实现同一泛型 trait 的多个 application：

```rust
impl From<String> for CustomerId {
    // ...
}

impl From<u64> for CustomerId {
    // ...
}
```

因此 `lookup_trait_impl_by_definition(from_definition)` 可能歧义，而 `trait_impl(from_string)` 使用精确 application 并要求结果存在。

### 19.12 方法查找与 impl 的关系

之前确认的三个方法入口分别具有以下语义：

```rust
ty.inherent_method("reset")
ty.trait_method(cache_control, "reset")
ty.method("reset")
```

```text
inherent_method   所有 inherent impl 的有效方法聚合
trait_method      指定精确 trait application 的 impl 命名空间
method            所有 inherent 和 trait impl 命名空间聚合
```

context 构建时验证：

- 同一具体类型的所有 inherent impl 中不能产生两个同名有效方法；
- 同一精确 trait application 对同一具体类型最多有一个正向 impl；
- 同一 impl 命名空间内方法名称唯一；
- 不同 trait 命名空间允许同名方法；
- crate 的不同版本拥有不同定义身份，不会因为路径字符串相同而被错误合并。

### 19.13 Context 中的查询入口

原本含义不明确的 `traits()` 拆为定义和具体应用：

```rust
impl ReflectionContext {
    pub fn trait_definitions(
        &self,
    ) -> Result<TraitDefinitions<'_>, ContextError>;

    pub fn find_trait_definition_by_id(
        &self,
        id: TraitDefinitionId,
    ) -> Result<Option<TraitDefinitionDescriptor<'_>>, ContextError>;

    pub fn trait_definition_by_id(
        &self,
        id: TraitDefinitionId,
    ) -> Result<TraitDefinitionDescriptor<'_>, LookupError>;

    pub fn lookup_trait_definition_by_name(
        &self,
        path: &str,
    ) -> Result<Lookup<TraitDefinitionDescriptor<'_>>, ContextError>;

    pub fn trait_definition_by_name(
        &self,
        path: &str,
    ) -> Result<TraitDefinitionDescriptor<'_>, LookupError>;

    pub fn trait_definitions_by_short_name(
        &self,
        name: &str,
    ) -> Result<TraitDefinitions<'_>, ContextError>;

    /// context 中实际登记的具体 trait applications。
    pub fn trait_applications(
        &self,
    ) -> Result<Traits<'_>, ContextError>;

    pub fn impl_definitions(
        &self,
    ) -> Result<ImplDefinitions<'_>, ContextError>;
}
```

不提供运行时 trait solver，也不允许凭泛型参数任意生成新的 `TraitDescriptor`。具体 application 来自：

- 已登记的具体 trait impl；
- 已登记的 trait object；
- 已登记的方法或类型边界；
- 宏明确生成的 application metadata。

### 19.14 示例

```rust
trait Repository<T> {
    type Error;

    fn save(&self, value: T) -> Result<(), Self::Error>;
}

impl Repository<Customer> for Store {
    type Error = StoreError;

    fn save(&self, value: Customer) -> Result<(), StoreError> {
        // ...
    }
}
```

```rust
let store = context.type_of::<Store>();

let repository = context
    .trait_definition_by_name(
        "my_crate::repository::Repository",
    )?;

let implementation =
    store.trait_impl_by_definition(repository)?;

let applied = implementation
    .implemented_trait()
    .expect("trait impl");

let argument = applied.arguments().argument("T")?.type_argument()?;
assert_eq!(
    argument.concrete_type()?.type_id(),
    TypeId::of::<Customer>(),
);

let error = implementation.associated_type("Error")?;
assert_eq!(
    error.find_concrete_type().unwrap().type_id(),
    TypeId::of::<StoreError>(),
);

implementation
    .method("save")?
    .invoke(
        Invocation::on_ref(&store_value)
            .arg(customer),
    )?;
```

### 19.15 类型表达式名称统一

参数和返回值描述中早期暂用的 `TypeExpressionDescriptor<'ctx>` 统一为已经确认的 `TypeExpression<'ctx>`：

```rust
pub fn type_expression(self) -> TypeExpression<'ctx>;
```

## 20. 动态值模型

### 20.1 核心语义

动态值跨越反射边界时仍然保留所有权、借用生命周期和线程安全保证：

```rust
pub struct DynamicOwned<M: ValueMode = Local> {
    // opaque
}

pub struct DynamicRef<'a, M: ValueMode = Local> {
    // opaque
}

pub struct DynamicMut<'a, M: ValueMode = Local> {
    // opaque
}
```

三者分别对应：

```text
DynamicOwned<M>    T
DynamicRef<'a, M>  &'a T
DynamicMut<'a, M>  &'a mut T
```

动态值本身不绑定 `ReflectionContext`。同一个值可以安全传给不同 context 中描述同一 Rust 类型的 descriptor。

### 20.2 统一的 `ValueMode`

Invocation、构造、字段访问和动态值统一使用一个 sealed mode trait：

```rust
pub trait ValueMode: sealed::Sealed {
    // internal storage and auto-trait policy
}

pub enum Local {}
pub enum ThreadSafe {}

impl ValueMode for Local {}
impl ValueMode for ThreadSafe {}
```

所有相关 API 统一使用：

```rust
MethodDescriptor::invoke<M: ValueMode>(...)
DynamicOwned<M>
DynamicRef<'a, M>
DynamicMut<'a, M>
NamedFields<M>
ConstructionInput<M>
Invocation<'a, M>
InvocationOutput<'a, M>
```

不再保留单独的公开 `InvocationMode` 或旧实现中的 `Mode`。

### 20.3 `Local` 与 `ThreadSafe`

`Local` 接受普通本地值，其 wrapper 明确不实现 `Send` 和 `Sync`：

```text
DynamicOwned<Local>    !Send + !Sync
DynamicRef<Local>      !Send + !Sync
DynamicMut<Local>      !Send + !Sync
```

`ThreadSafe` 在构造边界执行编译期约束：

| 包装器 | 底层约束 |
|---|---|
| `DynamicOwned<ThreadSafe>` | `T: Send + Sync` |
| `DynamicRef<ThreadSafe>` | `T: Sync` |
| `DynamicMut<ThreadSafe>` | `T: Send + Sync` |

由此保证：

```text
DynamicOwned<ThreadSafe>    Send + Sync
DynamicRef<ThreadSafe>      Send + Sync
DynamicMut<ThreadSafe>      Send + Sync
```

借用值能否实际跨线程还受到其生命周期限制。

### 20.4 值约束

使用三个 sealed marker trait 表达不同所有权边界：

```rust
pub trait ValueFor<M: ValueMode>:
    Reflect + 'static + sealed::Sealed
{
}

pub trait ValueForRef<M: ValueMode>:
    Reflect + sealed::Sealed
{
}

pub trait ValueForMut<M: ValueMode>:
    Reflect + sealed::Sealed
{
}
```

这些 trait 由 crate 自动提供 blanket implementation，用户不能手工声称一个类型满足线程安全要求。它们表达：

```text
ValueFor<Local>           T: Reflect + 'static
ValueFor<ThreadSafe>      T: Reflect + 'static + Send + Sync

ValueForRef<Local>        T: Reflect + ?Sized
ValueForRef<ThreadSafe>   T: Reflect + Sync + ?Sized

ValueForMut<Local>        T: Reflect + ?Sized
ValueForMut<ThreadSafe>   T: Reflect + Send + Sync + ?Sized
```

动态值要求 `Reflect`，因为它必须携带准确的静态类型 metadata，而不只是一个无法导航的 `Any`。外部类型通过内置或 ecosystem integration、拥有该类型或 `Reflect` trait 的集成 crate，或者本地 newtype 获得 `Reflect`；仅出现在定义中的未知外部类型仍可保留结构化 `TypeExpression`，但不能直接进入动态值边界。

### 20.5 `DynamicOwned`

```rust
impl<M: ValueMode> DynamicOwned<M> {
    pub fn new<T: ValueFor<M>>(value: T) -> Self;

    pub fn type_id(&self) -> TypeId;

    /// 绑定默认全局 context，不触发 context 初始化。
    pub fn ty(&self) -> TypeDescriptor<'static>;

    /// 把相同静态类型 metadata 绑定到指定 context。
    pub fn ty_in<'ctx>(
        &self,
        context: &'ctx ReflectionContext,
    ) -> TypeDescriptor<'ctx>;

    pub fn is<T: ValueFor<M>>(&self) -> bool;

    pub fn downcast_ref<T: ValueFor<M>>(
        &self,
    ) -> Option<&T>;

    pub fn downcast_mut<T: ValueFor<M>>(
        &mut self,
    ) -> Option<&mut T>;

    /// 成功时取出 T；失败时保留原值。
    pub fn downcast<T: ValueFor<M>>(
        self,
    ) -> Result<T, DowncastFailure<Self>>;

    pub fn as_ref(&self) -> DynamicRef<'_, M>;
    pub fn as_mut(&mut self) -> DynamicMut<'_, M>;
}
```

失败恢复：

```rust
pub struct DowncastFailure<V> {
    expected: TypeId,
    actual: TypeId,
    value: V,
}

impl<V> DowncastFailure<V> {
    pub fn expected(&self) -> TypeId;
    pub fn actual(&self) -> TypeId;

    pub fn value(&self) -> &V;
    pub fn into_value(self) -> V;

    pub fn into_parts(self) -> (TypeMismatch, V);
}
```

```rust
let value = DynamicOwned::<Local>::new(String::from("ready"));

assert_eq!(value.type_id(), TypeId::of::<String>());
assert_eq!(
    value.downcast_ref::<String>().map(String::as_str),
    Some("ready"),
);

let value = match value.downcast::<u64>() {
    Ok(_) => unreachable!(),
    Err(error) => error.into_value(),
};

let text = value.downcast::<String>()?;
```

### 20.6 `DynamicRef`

```rust
impl<'a, M: ValueMode> DynamicRef<'a, M> {
    pub fn new<T>(value: &'a T) -> Self
    where
        T: ValueForRef<M> + ?Sized;

    pub fn type_id(self) -> TypeId;

    pub fn ty(self) -> TypeDescriptor<'static>;

    pub fn ty_in<'ctx>(
        self,
        context: &'ctx ReflectionContext,
    ) -> TypeDescriptor<'ctx>;

    pub fn is<T>(self) -> bool
    where
        T: ValueForRef<M> + ?Sized;

    /// 返回生命周期受当前 wrapper 借用限制的引用。
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: ValueForRef<M> + ?Sized;

    /// 消费 wrapper，成功时恢复原始 'a 生命周期。
    pub fn downcast<T>(
        self,
    ) -> Result<&'a T, DowncastFailure<Self>>
    where
        T: ValueForRef<M> + ?Sized;
}
```

`DynamicRef` 可以实现 `Clone + Copy`，复制的是共享借用句柄，不复制底层值：

```rust
let text: &str = "hello";
let dynamic = DynamicRef::<Local>::new(text);

let restored: &str = dynamic.downcast::<str>()?;
```

### 20.7 `DynamicMut`

```rust
impl<'a, M: ValueMode> DynamicMut<'a, M> {
    pub fn new<T>(value: &'a mut T) -> Self
    where
        T: ValueForMut<M> + ?Sized;

    pub fn type_id(&self) -> TypeId;

    pub fn ty(&self) -> TypeDescriptor<'static>;

    pub fn ty_in<'ctx>(
        &self,
        context: &'ctx ReflectionContext,
    ) -> TypeDescriptor<'ctx>;

    pub fn is<T>(&self) -> bool
    where
        T: ValueForMut<M> + ?Sized;

    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: ValueForRef<M> + ?Sized;

    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: ValueForMut<M> + ?Sized;

    /// 消费 wrapper，成功时恢复原始可变借用。
    pub fn downcast<T>(
        self,
    ) -> Result<&'a mut T, DowncastFailure<Self>>
    where
        T: ValueForMut<M> + ?Sized;

    /// 产生生命周期缩短后的共享借用。
    pub fn as_ref(&self) -> DynamicRef<'_, M>;

    /// 产生生命周期缩短后的可变重借用。
    pub fn reborrow(&mut self) -> DynamicMut<'_, M>;
}
```

`DynamicMut` 不实现 `Clone` 或 `Copy`：

```rust
let mut count = 10_u64;
let mut dynamic = DynamicMut::<Local>::new(&mut count);

*dynamic.downcast_mut::<u64>().unwrap() += 1;
assert_eq!(count, 11);
```

### 20.8 DST 支持

公开 API 不再通过 `new_str()`、`new_str_mut()` 或 `as_str()` 特判 `str`。`DynamicRef::new()` 和 `DynamicMut::new()` 接受 `?Sized` reflected type：

```rust
let text: &str = "hello";
let text = DynamicRef::<Local>::new(text);

let numbers: &[u32] = &[1, 2, 3];
let numbers = DynamicRef::<Local>::new(numbers);
```

支持范围由 `Reflect` metadata 的擦除和恢复 adapter 决定，包括：

- `str`；
- `[T]`；
- 已登记的 trait object；
- 其他明确支持的 DST。

adapter 属于 `__private` 宏协议。公开 API 不依赖 `Any` 对 DST 的限制。

`DynamicOwned` 仍只持有 sized value，因为 Rust 不能按值拥有裸 `str` 或 `[T]`：

```rust
DynamicOwned::new(String::from("hello"));
DynamicOwned::new(Box::<str>::from("hello"));
```

后一个值的准确类型是 `Box<str>`，不是 `str`。

### 20.9 不公开 `Any` 存储

不提供以下底层存储 API：

```rust
// 不提供
as_any()
as_any_mut()
into_any()
from_box()
```

原因是：

- `Any` 不能完整表达需要支持的 DST 边界；
- `Local` 和 `ThreadSafe` 会暴露不同 trait object 类型；
- 调用方真正需要的是精确类型检查、downcast 和失败恢复；
- 暴露内部 box 会绕过 `Reflect` metadata 与 mode 保证。

内部实现仍可按需使用 `Any`，但它不是核心公开契约。

### 20.10 统一动态值枚举

容器、Invocation 和恢复 API 使用统一的 tagged value：

```rust
pub enum DynamicValue<'a, M: ValueMode = Local> {
    Owned(DynamicOwned<M>),
    Ref(DynamicRef<'a, M>),
    Mut(DynamicMut<'a, M>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicValueKind {
    Owned,
    Ref,
    Mut,
}
```

```rust
impl<'a, M: ValueMode> DynamicValue<'a, M> {
    pub fn kind(&self) -> DynamicValueKind;
    pub fn type_id(&self) -> TypeId;

    pub fn ty(&self) -> TypeDescriptor<'static>;

    pub fn ty_in<'ctx>(
        &self,
        context: &'ctx ReflectionContext,
    ) -> TypeDescriptor<'ctx>;

    /// 获得生命周期缩短后的共享视图。
    pub fn as_ref(&self) -> DynamicRef<'_, M>;
}
```

所有权相关操作仍通过 enum 匹配显式完成，避免隐藏消费行为。

### 20.11 Mode 转换

线程安全值可以安全降级为本地值：

```rust
impl DynamicOwned<ThreadSafe> {
    pub fn into_local(self) -> DynamicOwned<Local>;
}

impl<'a> DynamicRef<'a, ThreadSafe> {
    pub fn into_local(self) -> DynamicRef<'a, Local>;
}

impl<'a> DynamicMut<'a, ThreadSafe> {
    pub fn into_local(self) -> DynamicMut<'a, Local>;
}
```

不提供擦除后的动态升级：

```rust
// 不提供
local.try_into_thread_safe()
```

即使底层值碰巧满足 `Send + Sync`，`Local` 存储也没有保留足以证明这一点的公开契约。知道具体类型的调用方可以 downcast 后重新包装：

```rust
let value: MyValue = local.downcast()?;
let value = DynamicOwned::<ThreadSafe>::new(value);
```

### 20.12 Context 独立性

动态值保存静态类型 metadata，但不保存 context：

```rust
let value = DynamicOwned::<Local>::new(Customer::default());

let global_type = value.ty();
let isolated_type = value.ty_in(&isolated_context);

assert_eq!(global_type.type_id(), isolated_type.type_id());
```

这两个 descriptor 描述同一个 Rust 类型，但可以观察到不同的方法、capability、trait impl 和调用策略。反射操作只按精确 Rust 类型身份验证值，不要求值和 descriptor 原先来自同一个 context。

### 20.13 精确类型匹配

动态边界不隐式执行：

- 数值转换；
- `Deref` 或自动引用；
- trait object upcast；
- `String` 与 `str` 转换；
- `Vec<T>` 与 `[T]` 转换；
- 父子类型或用户定义转换。

```rust
// 方法要求 u64
.arg(1_u32) // TypeMismatch
.arg(1_u64) // 正确

// 方法要求共享 str
.arg_ref("hello")               // T = str，正确
.arg(String::from("hello"))     // T = String，不匹配
```

转换必须由调用方显式执行，或者由上层序列化、绑定框架提供转换策略。

### 20.14 字段访问 mode

动态字段访问使用同一个 `ValueMode`：

```rust
impl<'ctx> FieldDescriptor<'ctx> {
    pub fn get_dynamic<'a, M: ValueMode>(
        self,
        target: DynamicRef<'a, M>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError>;

    pub fn get_dynamic_mut<'a, M: ValueMode>(
        self,
        target: DynamicMut<'a, M>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError>;

    pub fn set_dynamic<M: ValueMode>(
        self,
        target: DynamicMut<'_, M>,
        value: DynamicOwned<M>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>>;
}
```

字段类型不匹配时保留 replacement：

```rust
pub struct FieldSetFailure<V> {
    error: FieldSetError,
    replacement: V,
}

impl<V> FieldSetFailure<V> {
    pub fn error(&self) -> &FieldSetError;
    pub fn replacement(&self) -> &V;

    pub fn into_parts(
        self,
    ) -> (FieldSetError, V);
}
```

### 20.15 安全与析构保证

所有安全构造器保证：

- 动态 wrapper 不超过原始借用生命周期；
- `DynamicMut` 不能被安全复制；
- downcast 只接受精确类型身份；
- DST 指针 metadata 由对应 `Reflect` adapter 保存和恢复；
- `DynamicOwned` 中的值恰好析构一次；
- downcast 失败不丢失拥有值或借用句柄；
- `ThreadSafe` 不通过运行时布尔值伪造 `Send + Sync`；
- context 切换不改变底层值的 Rust 类型身份。

不提供公开的 `new_unchecked()`。生成代码通过 `__private` 中受约束的 adapter API 构造 erased value，避免普通调用方伪造类型身份或指针 metadata。

## 21. 总体错误模型

### 21.1 三种失败形态

核心 API 区分三种失败形态：

```text
Error      只包含诊断信息，不持有输入
Failure    Error + 可恢复输入
Panic      用户代码已经开始执行后的 panic
```

`FieldAccessError`、`LookupError` 和 `ContextError` 属于普通错误。`FieldSetFailure<V>`、`InvocationFailure<'a, M>`、`ConstructionFailure<M>`、`UpdateFailure<M>` 和 `DowncastFailure<V>` 属于可恢复失败。用户方法产生的 panic 由 `InvocationPanic` 和 `CaughtInvocationFailure` 单独表示。

### 21.2 命名规则

统一命名约定：

- `*Error`：只含错误原因和诊断上下文；
- `*Failure`：包含一个 `*Error` 和尚可恢复的输入；
- `*Panic`：包含捕获的 panic 信息；
- `*UnavailableReason`：解释某项能力为什么不可用，不一定表示已经执行失败。

不再使用含义模糊的泛型错误：

```rust
// 不提供
FieldSetError<DynamicOwned<M>>
```

改为明确拆分诊断和恢复数据：

```rust
pub enum FieldSetError {
    // diagnostic only
}

pub struct FieldSetFailure<V> {
    error: FieldSetError,
    replacement: V,
}
```

### 21.3 统一 lookup 错误

不再按对象种类分别定义 `TypeLookupError`、`TraitLookupError`、`MethodLookupError`、`MemberLookupError`、`ImplLookupError` 或 `SpecializationLookupError`，统一使用：

```rust
#[non_exhaustive]
pub enum LookupError {
    Context(ContextError),

    Missing {
        subject: LookupSubject,
        query: LookupQuery,
    },

    Ambiguous {
        subject: LookupSubject,
        query: LookupQuery,
        candidates: Box<[LookupCandidate]>,
    },
}
```

查找对象：

```rust
#[non_exhaustive]
pub enum LookupSubject {
    Type,
    TypeDefinition,
    TraitDefinition,
    TraitApplication,
    Impl,
    Field,
    Variant,
    Method,
    MethodSpecialization,
    Parameter,
    AssociatedType,
    AssociatedConst,
    Capability,
}
```

查询条件使用结构化数据：

```rust
#[non_exhaustive]
pub enum LookupQuery {
    Name(Box<str>),
    ShortName(Box<str>),
    TypeId(TypeId),
    ItemId(StableItemId),
    Index(usize),
    GenericArguments(Box<[GenericArgumentIdentity]>),
}
```

候选摘要不持有 context-bound descriptor：

```rust
pub struct LookupCandidate {
    pub identity: ItemIdentity,
    pub display_name: Box<str>,
    pub origin: Option<RegistrationOrigin>,
}
```

因此 `LookupError` 不携带 `'ctx`，可以离开 descriptor 的借用范围，也不要求错误对象永久持有整个 context。

### 21.4 `Lookup<T>`

```rust
#[must_use]
pub enum Lookup<T> {
    Missing,
    Unique(T),
    Ambiguous(Box<[T]>),
}
```

```rust
impl<T> Lookup<T> {
    pub fn is_missing(&self) -> bool;
    pub fn is_unique(&self) -> bool;
    pub fn is_ambiguous(&self) -> bool;

    pub fn len(&self) -> usize;
    pub fn candidates(&self) -> &[T];

    pub fn only(self) -> Option<T>;

    pub fn into_unique(
        self,
        subject: LookupSubject,
        query: LookupQuery,
    ) -> Result<T, LookupError>
    where
        T: IntoLookupCandidate;
}
```

严格单数 accessor 自动填充 subject 和 query，普通调用方通常不需要直接构造它们。候选顺序由 context 构建阶段稳定排序，不能依赖 `HashMap` 遍历顺序。

### 21.5 严格单数 accessor 不 panic

`field()`、`method()`、`type_by_name()` 等严格单数 accessor 都表示要求结果存在且唯一，否则返回结构化错误。它们不等于 `unwrap()` 或 `expect()`：

```rust
let method = ty.method("save")?;
```

该调用可能返回：

```rust
LookupError::Context(...)
LookupError::Missing { ... }
LookupError::Ambiguous { ... }
```

如果失败确实属于调用方程序错误，由调用方显式决定 panic：

```rust
let method = ty
    .method("save")
    .expect("Store must expose exactly one save method");
```

反射库本身不替调用方 panic。

### 21.6 查找 API 返回类型

不需要初始化 context 的局部查找直接返回 `LookupError`：

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub fn field(
        self,
        name: &str,
    ) -> Result<FieldDescriptor<'ctx>, LookupError>;

    pub fn variant(
        self,
        name: &str,
    ) -> Result<VariantDescriptor<'ctx>, LookupError>;
}
```

需要 context 聚合信息的严格 accessor 也返回 `LookupError`，通过 `Context` variant 保留初始化失败：

```rust
impl<'ctx> TypeDescriptor<'ctx> {
    pub fn method(
        self,
        name: &str,
    ) -> Result<MethodDescriptor<'ctx>, LookupError>;

    pub fn trait_impl(
        self,
        trait_: TraitDescriptor<'ctx>,
    ) -> Result<ImplDescriptor<'ctx>, LookupError>;
}

impl ReflectionContext {
    pub fn type_by_name(
        &self,
        name: &str,
    ) -> Result<TypeDescriptor<'_>, LookupError>;

    pub fn trait_definition_by_name(
        &self,
        name: &str,
    ) -> Result<TraitDefinitionDescriptor<'_>, LookupError>;
}
```

`lookup_*()` 查询保留更精确的双层结果：

```rust
pub fn lookup_type_by_name(
    &self,
    name: &str,
) -> Result<Lookup<TypeDescriptor<'_>>, ContextError>;
```

其含义是：

- 外层 `Err`：context 无法建立或访问；
- `Lookup::Missing`：context 正常，但没有匹配项；
- `Lookup::Ambiguous`：context 正常，但匹配项不唯一。

### 21.7 `ContextError`

context 构建是事务式的。任何验证错误都会使整个新 context 构建失败，不暴露半初始化对象：

```rust
#[derive(Clone)]
pub struct ContextError {
    data: Arc<ContextErrorData>,
}

impl ContextError {
    pub fn issues(&self) -> &[ContextIssue];
    pub fn primary_issue(&self) -> &ContextIssue;
}
```

一次构建可以报告多个独立问题：

```rust
#[non_exhaustive]
pub enum ContextIssue {
    DuplicateTypeIdentity {
        type_id: TypeId,
        registrations: Box<[RegistrationOrigin]>,
    },

    ConflictingTypeDefinition {
        identity: TypeDefinitionId,
        registrations: Box<[RegistrationOrigin]>,
    },

    ConflictingTraitDefinition {
        identity: TraitDefinitionId,
        registrations: Box<[RegistrationOrigin]>,
    },

    DuplicateTraitImpl {
        target: TypeIdentity,
        trait_: TraitIdentity,
        registrations: Box<[RegistrationOrigin]>,
    },

    DuplicateMethodSpecialization {
        method: MethodIdentity,
        arguments: Box<[GenericArgumentIdentity]>,
        registrations: Box<[RegistrationOrigin]>,
    },

    ConflictingCapability {
        target: TypeIdentity,
        capability: CapabilityId,
        registrations: Box<[RegistrationOrigin]>,
    },

    MissingDependency {
        item: ItemIdentity,
        dependency: ItemIdentity,
    },

    DependencyCycle {
        items: Box<[ItemIdentity]>,
    },

    InvalidAdapter {
        item: ItemIdentity,
        reason: AdapterValidationError,
    },

    InvalidMetadata {
        item: ItemIdentity,
        reason: MetadataValidationError,
    },
}
```

全局 context 的初始化成功或失败都缓存。多次失败时返回共享同一底层诊断数据的廉价 `ContextError::clone()`，不会重复扫描 linked inventory。

### 21.8 身份与注册来源

错误对象不保存 descriptor 引用，而保存独立的诊断身份：

```rust
pub struct TypeIdentity {
    pub type_id: TypeId,
    pub rust_name: &'static str,
}

pub struct ItemIdentity {
    pub kind: ItemKind,
    pub stable_id: StableItemId,
    pub display_name: Box<str>,
}

pub struct RegistrationOrigin {
    pub crate_name: &'static str,
    pub crate_version: Option<&'static str>,
    pub module_path: &'static str,
    pub file: Option<&'static str>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
```

`RegistrationOrigin` 只用于诊断，不参与 Rust 类型身份或反射名称匹配。

### 21.9 `TypeMismatch`

所有动态边界共用一种类型不匹配结构：

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMismatch {
    expected: TypeIdentity,
    actual: TypeIdentity,
}

impl TypeMismatch {
    pub fn expected(&self) -> &TypeIdentity;
    pub fn actual(&self) -> &TypeIdentity;
}
```

具体错误通过 variant 说明发生位置：

```rust
#[non_exhaustive]
pub enum FieldAccessError {
    TargetTypeMismatch(TypeMismatch),
    Unavailable(FieldAccessUnavailableReason),
    NotReadable,
    NotMutable,
    NotPublic,
    InactiveVariant {
        expected: VariantIdentity,
        actual: VariantIdentity,
    },
}

#[non_exhaustive]
pub enum FieldSetError {
    TargetTypeMismatch(TypeMismatch),
    ValueTypeMismatch(TypeMismatch),
    Unavailable(FieldAccessUnavailableReason),
    NotReplaceable,
    NotPublic,
    InactiveVariant {
        expected: VariantIdentity,
        actual: VariantIdentity,
    },
}

#[non_exhaustive]
pub enum FieldAccessUnavailableReason {
    FieldTypeNotReflectable,
    UnsupportedFieldShape,
    AdapterUnavailable,
}
```

Invocation、构造、更新和 downcast 同样复用 `TypeMismatch`，不各自重新定义 expected/actual 字段。

### 21.10 错误路径

嵌套操作可以携带结构化路径：

```rust
#[derive(Clone, Debug, Default)]
pub struct ErrorPath {
    frames: Box<[ErrorFrame]>,
}

#[non_exhaustive]
pub enum ErrorFrame {
    Type(TypeIdentity),
    Trait(TraitIdentity),
    Impl(ImplIdentity),

    Field {
        name: Box<str>,
        index: usize,
    },

    Variant {
        name: Box<str>,
        index: usize,
    },

    Method(MethodIdentity),

    Parameter {
        name: Option<Box<str>>,
        index: usize,
    },

    Argument {
        name: Option<Box<str>>,
        index: usize,
    },

    Capability(CapabilityId),
}
```

例如嵌套构造错误可以表达：

```text
Customer
└── field "address"
    └── field "postal_code"
        └── expected PostalCode, actual String
```

错误路径是机器可导航的数据；`Display` 只负责将其格式化成人类可读文本。

### 21.11 窄 API 返回窄错误

```text
field.get(...)          FieldAccessError
field.set(...)          FieldSetFailure<V>
method.invoke(...)      InvocationFailure<'a, M>
ty.construct(...)       ConstructionFailure<M>
ty.update(...)          UpdateFailure<M>
value.downcast(...)     DowncastFailure<V>
```

调用方可以直接处理恢复数据，不需要从巨大的顶层 enum 中解包：

```rust
match field.set_dynamic(target, replacement) {
    Ok(()) => {}
    Err(failure) => {
        let (error, replacement) = failure.into_parts();
        log::warn!("field update rejected: {error}");
        retry_elsewhere(replacement);
    }
}
```

### 21.12 `RecoverableFailure`

各个命名 failure 类型共享只读和拆分协议：

```rust
pub trait RecoverableFailure {
    type Error: std::error::Error + Send + Sync + 'static;
    type Recovery;

    fn error(&self) -> &Self::Error;
    fn recovery(&self) -> &Self::Recovery;

    fn into_parts(self) -> (Self::Error, Self::Recovery);
}
```

实现者包括：

```text
FieldSetFailure<V>
InvocationFailure<'a, M>
ConstructionFailure<M>
UpdateFailure<M>
DowncastFailure<V>
```

具体 failure 仍可提供 `into_replacement()`、`into_invocation()`、`into_input()` 或 `into_value()` 等便利方法。`Local` 恢复对象可能不是 `Send` 或 `Sync`，所以只要求错误原因满足 `Send + Sync`，不对 `Recovery` 作统一线程安全要求。

### 21.13 恢复边界

仅当 adapter 或用户代码尚未开始消费输入时保证完整恢复：

```text
验证 receiver、参数、字段和类型
          │
          ├── 失败：保证恢复
          │
          ▼
调用生成 adapter
          │
          ├── adapter 拒绝：按 adapter 契约决定
          │
          ▼
执行用户代码
          └── 不保证回滚或恢复
```

- lookup、类型检查、可见性检查和参数绑定失败时完整恢复；
- 构造 provider 尚未执行时完整恢复；
- 用户方法开始执行后返回业务错误时，它是正常方法返回值，不是反射失败；
- 用户方法 panic 时不承诺输入恢复；
- 用户代码产生的外部副作用不由反射层回滚。

### 21.14 Panic 模型

```rust
pub struct InvocationPanic {
    message: Option<Box<str>>,
    payload: Box<dyn Any + Send + 'static>,
}

impl InvocationPanic {
    pub fn message(&self) -> Option<&str>;
    pub fn payload(&self) -> &(dyn Any + Send);
    pub fn into_payload(self) -> Box<dyn Any + Send>;
}
```

```rust
#[non_exhaustive]
pub enum CaughtInvocationFailure<'call, M: ValueMode> {
    Rejected(InvocationFailure<'call, M>),
    Panicked(InvocationPanic),
}
```

普通 `invoke()` 不捕获 panic。只有显式 `invoke_catching()` 使用该模型。

### 21.15 顶层 `ReflectionError`

需要统一 `?` 传播的应用可以使用 umbrella error：

```rust
#[non_exhaustive]
pub enum ReflectionError {
    Context(ContextError),
    Lookup(LookupError),
    TypeMismatch(TypeMismatch),

    FieldAccess(FieldAccessError),
    FieldSet(FieldSetError),

    Invocation(InvocationError),
    Construction(ConstructionError),
    Update(UpdateError),

    Capability(CapabilityError),
    Unsupported(UnsupportedOperationError),
}
```

普通错误提供标准转换：

```rust
impl From<ContextError> for ReflectionError;
impl From<LookupError> for ReflectionError;
impl From<FieldAccessError> for ReflectionError;
impl From<ConstructionError> for ReflectionError;
// ...
```

不为可恢复 failure 提供可能丢失恢复数据的转换：

```rust
// 不提供
impl From<FieldSetFailure<_>> for ReflectionError;
impl From<InvocationFailure<_, _>> for ReflectionError;
```

调用方必须显式拆分：

```rust
let (error, recovery) = failure.into_parts();
let error = ReflectionError::from(error);
```

### 21.16 标准 trait 与稳定性

所有纯诊断错误实现：

```text
Debug
Display
std::error::Error
Send
Sync
'static
```

- `Display` 面向人类，不承诺格式稳定，调用方不得解析字符串；
- enum variant 和访问器是机器处理接口；
- 预计未来扩展的公开错误 enum 使用 `#[non_exhaustive]`；
- source chain 使用 `Error::source()`；
- 核心 API 不返回 `Box<dyn Error>`；
- 正常的缺失、歧义、类型不匹配和不可用能力不通过 panic 表示。

`Failure` 是否实现 `Send`、`Sync` 由其恢复数据和 `ValueMode` 决定。

## 22. 注册协议与宏边界

### 22.1 第一性原则

反射宏的职责是收集 Rust 定义原本具有的信息，而不是要求用户用 helper attribute 再声明一套平行的类型系统。设计明确区分：

```text
源码事实                自动采集
动态操作能力            根据源码和 Rust 类型系统自动推导
应用运行策略            在 context 或调用点配置
```

例如，宏应从以下方法自动采集名称、visibility、`async`、`unsafe`、receiver、泛型参数、predicate、参数和返回值，并派生动态调用可用性：

```rust
pub async unsafe fn save<T>(
    &mut self,
    value: T,
) -> Result<(), Error>
where
    T: Serialize + Send;
```

不能要求用户通过 attribute 重复或覆盖这些事实。

### 22.2 两类 metadata

```text
类型固有 metadata     不依赖 context，TypeDescriptor::of<T>() 可直接获得
聚合注册事实          由 ReflectionContext 收集、校验和冻结
```

类型固有 metadata 包括：

- 类型身份和原始 Rust 名称；
- 类型种类；
- struct 字段和 enum variant；
- 泛型定义和类型表达式；
- 可安全生成的字段访问、构造和动态值 adapter。

聚合注册事实包括：

- 可发现类型和类型定义；
- trait 定义及 application；
- impl 定义及具体 application；
- 有效方法和泛型方法绑定；
- capability；
- 跨 fragment 依赖关系。

因此 `TypeDescriptor::of::<Customer>()` 不需要初始化全局 context，而 `ty.methods()`、`ty.impls()` 和 `ty.capabilities()` 可能触发绑定 context 的惰性初始化。

### 22.3 `Reflect` trait

`Reflect` 是类型固有 metadata 的编译期入口：

```rust
pub trait Reflect: 'static {
    #[doc(hidden)]
    fn __reflect_registration(
    ) -> &'static __private::TypeRegistration<Self>;
}

#[doc(hidden)]
pub struct TypeRegistration<T: Reflect + ?Sized> {
    // typed metadata and adapters
}
```

`TypeRegistration<T>` 通过类型参数把 metadata 与准确的 `T` 绑定。普通用户不直接调用隐藏方法，而是使用：

```rust
impl TypeDescriptor<'static> {
    pub fn of<T: Reflect + ?Sized>() -> Self;
}

impl ReflectionContext {
    pub fn type_of<T: Reflect + ?Sized>(
        &self,
    ) -> TypeDescriptor<'_>;
}
```

`Reflect` 保持为安全 trait，但 runtime 不得因为错误的手写描述产生未定义行为。涉及原始指针、DST metadata 和 adapter 的构造器保持类型化；动态操作执行前仍检查准确类型身份；无效 metadata 在 context 构建或操作入口处被拒绝。

直接手写 `Reflect` 不是主要扩展路径，推荐使用宏。

### 22.4 `#[Reflect]`

```rust
#[Reflect]
struct Customer {
    id: CustomerId,
    email: String,
}
```

struct 或 enum 上的 `#[Reflect]` 生成：

1. `Reflect` 实现；
2. 类型固有 metadata；
3. 类型定义 metadata；
4. 字段、variant 和类型表达式；
5. 可安全生成的字段访问 adapter；
6. 可安全生成的构造和更新 adapter；
7. 动态值擦除和恢复 adapter；
8. linked registration fragment。

trait 上的 `#[Reflect]` 自动生成 trait 定义、泛型参数、predicate、supertrait expression、方法定义、关联项、默认实现事实和 linked trait-definition fragment：

```rust
#[Reflect]
trait Repository<T> {
    type Error;

    fn save(&self, value: T) -> Result<(), Self::Error>;
}
```

### 22.5 `#[ReflectImpl]`

```rust
#[ReflectImpl]
impl Repository<Customer> for Store {
    type Error = StoreError;

    fn save(
        &self,
        value: Customer,
    ) -> Result<(), StoreError> {
        // ...
    }
}
```

宏自动生成：

1. `ImplDefinitionDescriptor`；
2. target 和 implemented trait 的结构表达式；
3. impl 泛型定义；
4. 方法定义；
5. 关联类型和关联常量绑定；
6. 可以安全生成的 Invocation adapter；
7. 具体 impl 的 `ImplDescriptor` application；
8. linked impl-definition 和 impl-application fragment。

对于泛型或 blanket impl，宏始终可以登记源码级定义，但不能在运行时凭空生成所有具体 application。具体 application 只来自具体 impl、具体方法绑定、显式高级 registration，或者其他已经登记 metadata 的明确引用。

### 22.6 唯一 helper attribute

唯一保留的 helper attribute 是：

```rust
#[reflect(skip)]
```

它表示用户明确将一个成员排除出反射契约：

```rust
#[Reflect]
struct Account {
    pub id: AccountId,

    #[reflect(skip)]
    internal_cache: Cache,
}
```

结果是：

- 该成员不出现在 descriptor 中；
- 不能通过名称查询；
- 不生成访问、构造或更新 adapter；
- Rust 成员本身不受影响；
- `skip` 不是隐私或权限控制机制。

### 22.7 删除其他 helper attributes

不提供：

```rust
#[reflect(opaque)]
#[reflect(no_invoke)]
#[reflect(catch_unwind)]
#[reflect(thread_safe)]
#[reflect(capabilities(...))]
#[reflect(dyn_compatible)]
#[reflect(rename = "...")]
#[reflect(definition_provider_v2 = ...)]
#[reflect(crate = ...)]
```

理由如下：

| 旧属性 | 替代设计 |
|---|---|
| `opaque` | 始终保留原始 `TypeExpression`；无法导航时由 `concrete_type() == None` 和 availability reason 表达 |
| `no_invoke` | 方法定义始终保留；是否可调用由签名自动推导 |
| `catch_unwind` | 由调用者选择 `invoke()` 或 `invoke_catching()` |
| `thread_safe` | 由 `ValueMode` 和 Rust 的 `Send + Sync` bound 证明 |
| `capabilities(...)` | 使用独立的类型化 capability registration |
| `dyn_compatible` | 从 trait 定义自动分析，必要时由使用点让编译器证明 |
| `rename` | 反射只保留原始 Rust 名称 |
| `definition_provider_v2` | 删除旧 codegen 内部泄漏 |
| `crate = ...` | proc macro 自动解析 runtime crate 的依赖名称 |

### 22.8 未反射成员类型

字段类型没有实现 `Reflect` 时，字段仍然属于原始定义，不能因为缺少可执行 adapter 而被隐式隐藏：

```rust
#[Reflect]
struct Message {
    payload: ThirdPartyPayload,
}
```

```rust
let field = TypeDescriptor::of::<Message>()
    .field("payload")?;

field.type_expression(); // 保留 ThirdPartyPayload
field.find_concrete_type(); // 可能为 None
field.access();          // 记录动态操作可用性
```

定义信息是否存在、能否取得具体 descriptor、能否执行动态操作是三个独立问题。

### 22.9 自动推导方法可用性

方法无论能否动态调用，都保留定义 metadata：

```rust
let method = trait_definition.method("save")?;

match method.availability() {
    MethodAvailability::Callable => { /* ... */ }
    MethodAvailability::RequiresSpecialization => { /* ... */ }
    MethodAvailability::Unavailable { reasons } => { /* ... */ }
}
```

ABI、variadic、borrowed return、泛型是否完成绑定、receiver 形态等均从源码和具体 binding 自动派生。应用授权属于 context 或上层框架策略，不通过方法 helper attribute 篡改定义 metadata。

### 22.10 panic 与线程模式

panic 是否捕获由调用点决定：

```rust
method.invoke(invocation)?;
method.invoke_catching(invocation)?;
```

`panic = "abort"`、平台不支持 unwind 或某个异步边界不能安全捕获时，自动报告结构化不可用原因。

线程安全由类型系统证明：

```rust
DynamicOwned::<ThreadSafe>::new(value)
```

要求底层类型满足对应的 `Send + Sync` bound。泛型方法的 ThreadSafe specialization 同样在具体绑定点证明，不能通过 attribute 声称。

### 22.11 capability 是独立扩展

`Clone`、`Default`、`Send` 和 `Sync` 不能由 proc macro 仅从类型声明可靠判断，因为 impl 可能来自 derive、其他模块、cfg、auto trait 或具体泛型参数。

需要 capability 时使用独立 registration：

```rust
register_capability!(Customer: CloneCapability);
register_capability!(Customer: DefaultCapability);
```

或者显式 context：

```rust
ReflectionContext::builder()
    .register(
        CapabilityRegistration::for_type::<
            Customer,
            CloneCapability,
        >(CloneAdapter::for_type::<Customer>())
    )
    .build()?;
```

registration 构造器通过 Rust trait bound 验证能力，不根据字符串作出声明。`Send` 和 `Sync` 主要由 `ValueMode` 使用；只有确实需要枚举时才注册为 capability。

### 22.12 dyn compatibility

trait 定义自动提供：

```rust
pub enum DynCompatibility {
    Compatible,
    Incompatible {
        reasons: Box<[DynIncompatibilityReason]>,
    },
    Unknown {
        reasons: Box<[DynCompatibilityUnknownReason]>,
    },
}

impl<'ctx> TraitDefinitionDescriptor<'ctx> {
    pub fn dyn_compatibility(self) -> DynCompatibility;
}
```

完整本地 trait 通常可以从语法自动分析；涉及不完整外部 supertrait 时可以返回 `Unknown`。需要登记具体 trait-object 类型时，在使用点让编译器证明：

```rust
register_trait_object!(dyn Repository<Customer>);
```

不允许 attribute 强行声称 trait dyn-compatible。

### 22.13 泛型方法绑定

泛型方法只有在编译器已经完成具体单态化时才能动态调用：

```rust
let binding = bind_method!(
    Converter::convert::<u64>
    as fn(&Converter, String) -> u64
);
```

`bind_method!` 返回普通 `MethodBinding` 值，不自动修改全局 context：

```rust
let context = ReflectionContext::builder()
    .include_linked()
    .bind(binding)
    .build()?;
```

确实需要 linked 全局 registration 时使用独立宏：

```rust
register_method_binding!(
    Converter::convert::<u64>
    as fn(&Converter, String) -> u64
);
```

### 22.14 外部类型

由于 Rust orphan rule，下游 crate 不能为“外部 trait + 外部类型”实现 `Reflect`。正式支持路径是：

- `qubit-reflect` 内置实现；
- ecosystem integration feature；
- 拥有该类型或 `Reflect` trait 的集成 crate；
- 本地 newtype；
- 只作为不可导航的结构化类型表达式保存。

不提供声称能够绕过 orphan rule 的 `register_external_type!`。

### 22.15 外部 trait 身份限制

`#[ReflectImpl]` 可以从源码自动采集外部 trait 的书写路径：

```rust
#[ReflectImpl]
impl serde::Serialize for Customer {
    // ...
}
```

但稳定版 proc macro 无法向 rustc 查询依赖 alias、package 版本、路径重命名或 trait 的编译器级唯一身份，Rust 也没有 `TypeId::of::<Trait>()`。默认情况下将其记录为 `ExternalIncomplete`，并保留源码中写出的路径。

要求跨 crate、跨 alias 精确合并时，使用独立的类型化身份 handle：

```rust
reflect_external_trait! {
    pub SerdeSerialize = serde::Serialize;
}
```

该 handle 不是反射重命名；descriptor 仍呈现原始 trait 路径。它只解决稳定 Rust 无法自动提供的精确外部 trait 身份。

### 22.16 公开 registration API

普通用户主要使用宏。高级用户使用类型化 registration，不直接构造底层 fragment：

```rust
pub enum Registration {
    Type(TypeRegistrationHandle),
    Trait(TraitRegistration),
    Impl(ImplRegistration),
    MethodBinding(MethodBinding),
    Capability(CapabilityRegistration),
    TraitObject(TraitObjectRegistration),
}
```

具体字段保持私有，通过类型安全构造器建立：

```rust
impl ReflectionContextBuilder {
    pub fn register(
        self,
        registration: impl Into<Registration>,
    ) -> Self;

    pub fn bind(
        self,
        binding: MethodBinding,
    ) -> Self;
}
```

`RegistrationFragment`、`FragmentPayload`、`RuntimeIdentity`、`StaticFragmentIdentity` 和 `inventory::submit!` 不进入手写应用 API。

### 22.17 linked registration fragment

生成代码使用极小的静态 fragment：

```rust
#[doc(hidden)]
pub struct RegistrationFragment {
    protocol: ProtocolVersion,
    kind: FragmentKind,
    identity: StaticFragmentIdentity,

    target_identity: fn() -> RuntimeIdentity,
    materialize: fn() -> FragmentPayload,
}
```

fragment 只保存静态身份、种类、函数指针和编译期来源信息。链接发现阶段不执行用户方法、构造 provider、capability 业务逻辑或应用回调。materialize 只构造 immutable metadata 和 typed adapter 表。

### 22.18 context 构建过程

```text
发现 linked fragments
        │
        ▼
按稳定身份排序
        │
        ▼
物化 immutable metadata
        │
        ▼
校验身份、依赖、冲突和 adapter
        │
        ▼
解析 definition/application 关系
        │
        ▼
生成有效 method/capability 视图
        │
        ▼
冻结并发布 ReflectionContext
```

任何阶段失败都返回 `ContextError`，不发布半初始化 context。fragment 的链接顺序不能影响类型遍历、lookup 候选、冲突报告或有效方法合并结果。

### 22.19 全局 context 不可变

不提供：

```rust
ReflectionContext::register_global(...)
ReflectionContext::global_mut(...)
```

全局 context 第一次需要聚合信息时惰性构建，成功或失败均缓存，构建后永久不可变。创建 `MethodBinding` 或 `Registration` 值不会修改它。

测试隔离、插件白名单或运行时组装使用显式 context：

```rust
let context = ReflectionContext::builder()
    .include_linked_for::<Customer>()
    .register(custom_capability)
    .bind(method_binding)
    .build()?;
```

### 22.20 泛型类型与发现索引

```rust
#[Reflect]
struct Envelope<T> {
    value: T,
}
```

linked context 登记一次 `Envelope<T>` 类型定义，不自动枚举无限多个 monomorph。`TypeDescriptor::of::<Envelope<String>>()` 仍可直接产生具体 descriptor，但只有被 linked fragment、显式 registration 或 builder 纳入的具体应用才出现在 `context.types()` 和 `definition.instances()` 中。直接取得 descriptor 不修改发现索引。

### 22.21 版本化 codegen 协议

生成代码只通过版本化私有模块引用 runtime：

```rust
qubit_reflect::__private::codegen_v4
```

```text
__private::codegen_v4
├── descriptor
├── expression
├── access
├── value
├── construct
├── invoke
├── capability
├── registration
└── protocol
```

该模块 `#[doc(hidden)]`，不属于手写应用 API。同一协议版本内不静默改变契约；不兼容变更使用新版本；本次破坏性设计不要求保留 v3。宏与 runtime 不匹配时应产生编译错误。`v4` 只是 codegen/runtime 握手版本，不是产品名称或公开 API 版本号。

### 22.22 生成代码安全边界

生成代码应允许用户 crate 使用：

```rust
#![forbid(unsafe_code)]

#[Reflect]
struct Customer {
    email: String,
}
```

- adapter 使用泛型和函数签名进行编译期约束；
- 原始指针和 DST metadata 操作集中在 runtime crate；
- 生成代码调用 `__private` 中的类型化安全构造器；
- 动态入口执行准确类型身份检查；
- context 发布前验证 metadata 和 adapter 一致性；
- 普通用户不能构造伪造身份的 raw fragment。

### 22.23 feature 边界

```toml
[features]
default = ["derive"]
derive = ["dep:qubit-reflect-derive"]
ecosystem-types = ["..."]
```

- `derive` 只控制 procedural macro 依赖和重导；
- 关闭 `derive` 后，runtime descriptor、动态值、显式 context 和类型化 registration API 仍可使用；
- linked registration 基础设施属于 runtime 核心；
- ecosystem feature 只增加外部类型集成，不改变核心语义；
- feature 未启用的类型不会伪装成已反射类型。

### 22.24 编译期与 context 诊断边界

能从单个声明确定的问题由宏在编译期报告，例如不支持的 item、重复配置、`skip` 使用位置错误和无法形成类型正确 metadata。

只有跨 crate、跨 fragment 才能判断的问题进入 `ContextError`，例如重复 trait impl registration、重复 specialization、capability 冲突、缺少 linked dependency 和 crate 多版本产生的查找歧义。

### 22.25 最终宏表面

crate 根重导：

```rust
#[cfg(feature = "derive")]
pub use qubit_reflect_derive::{
    bind_method,
    reflect_external_trait,
    register_capability,
    register_method_binding,
    register_trait_object,
    Reflect,
    ReflectImpl,
};
```

高频用户路径只有：

```rust
#[Reflect]
#[ReflectImpl]
#[reflect(skip)]
bind_method!(...)
```

其余宏是按需使用的独立类型化 registration，不是定义 helper attribute，也不修改原始 definition metadata。

## 23. 已确认删除或替换的旧概念

- `ReflectRegistry` 作为普通用户必须传递的对象：替换为自动绑定的 `ReflectionContext`；
- `ReflectedType`：不引入，职责由 `TypeDescriptor<'ctx>` 承担；
- `methods_global()`、`methods_in()` 等成对 API：由 descriptor 所绑定的 context 决定；
- method lookup 后再次向 `invoke()` 传 registry：删除；
- `query_name()` 和通用 `#[reflect(rename = "...")]`：删除；
- 核心 API 中的 `type_by_rust_name()`：删除，保留 descriptor 上只读诊断属性 `rust_type_name()`；
- `require_*()` 命名：删除；严格语义由 `field()`、`method()`、`type_by_name()` 等单数 accessor 直接承担；
- 与 `method()` 重复的 `methods_by_name()`：删除；
- `discriminant_origin()` 与 `numeric_discriminant()` 两个分散入口：合并为 `VariantDiscriminant`。

## 24. 实现分层

### 24.1 依赖方向

实现按以下单向依赖组织：

```text
源码语法
   │
   ▼
derive IR ──► codegen_v4 静态 metadata / registration fragment
                         │
                         ▼
类型固有 metadata ──► ReflectionContext 聚合、校验和索引
                         │
                         ▼
               context-bound descriptor
                         │
                         ▼
       access / invoke / construct / capability adapter
```

约束：

- 类型固有 metadata 不依赖 `ReflectionContext`；
- context 可以引用静态 metadata，但静态 metadata 不反向引用具体 context；
- descriptor 是 metadata 与 context 的只读组合，不拥有或修改二者；
- execution adapter 只消费已经校验的 descriptor 和动态值；
- derive crate 只依赖公开语法模型和 `codegen_v4` 协议，不依赖 runtime 内部布局；
- `__private::codegen_v4` 不被普通用户代码调用。

### 24.2 Runtime 模块职责

现有源码应按责任演进，而不是在一个提交中进行无关重排：

| 模块 | vNext 职责 | 主要现有位置 |
|---|---|---|
| `descriptor` | context-bound 类型、字段、variant、方法、trait、impl descriptor 和集合视图 | `src/descriptor/**` |
| `expression` | 泛型定义、predicate、`TypeExpression`、`TraitExpression`、const/lifetime expression | `src/expression/**` |
| `context` | `ReflectionContext`、builder、linked fragment 收集、校验、冻结和索引 | 由 `src/registry/**` 演进 |
| `value` | `DynamicOwned`、`DynamicRef`、`DynamicMut`、`ValueMode` 和 downcast 恢复 | `src/value/**` |
| `access` | typed/dynamic 字段访问、variant 检查和替换恢复 | `src/access/**` |
| `invoke` | Invocation、绑定检查、执行、future、panic 和恢复 | `src/invoke/**` |
| `construct` | struct/variant 构造、拥有值更新和输入恢复 | `src/construct/**` |
| `capability` | 类型化 capability 契约、adapter、注册、解析和诊断 | `src/capability/**` |
| `error` | `LookupError`、`ContextError`、`TypeMismatch` 和 umbrella error | `src/error/**` |
| `registration` | 高级类型化 registration facade | 从现有 registry/private registration 中提取 |
| `__private::codegen_v4` | 宏与 runtime 的版本化握手协议 | 新增 `src/private/codegen_v4/**` |

公开模块名使用 `context`，不再把 `registry` 暴露为用户必须理解的核心概念。实现迁移期间可以暂时复用现有 `src/registry/**` 的算法，但最终公开 facade 不重导 `ReflectRegistry`。

### 24.3 Context 内部状态

全局 context 使用一次性惰性状态：

```text
Uninitialized
     │ first aggregate query / initialize()
     ▼
Building
  ├── success ──► Ready(Arc<ContextData>)
  └── failure ──► Failed(Arc<ContextErrorData>)
```

- `TypeDescriptor::of::<T>()` 和类型固有成员导航不触发状态转换；
- 第一个聚合查询负责构建，其他线程等待同一次结果；
- 成功和失败都永久缓存，不重复扫描 inventory；
- 不在持有全局初始化锁时执行用户方法或 capability adapter；
- 显式 builder 直接产生 `Ready` context，不经过全局状态；
- context 发布后完全不可变。

### 24.4 Context 数据与索引

`ContextData` 至少维护以下冻结索引：

- `TypeId -> type registration`；
- 完整反射名、短名到稳定排序候选集；
- type/trait/impl definition identity 到定义；
- concrete trait application 与 impl application；
- `(target, namespace, method name)` 到有效方法；
- `(method identity, generic arguments)` 到 specialization；
- `(capability target, capability id)` 到经过 adapter 类型校验的 capability；
- registration origin 与依赖边，用于冲突和缺失依赖诊断。

候选集合在冻结阶段排序并转为不可变 slice。公开遍历顺序必须由身份和 registration origin 的稳定比较决定，不能泄漏 `HashMap` 或 inventory 链接顺序。

### 24.5 Descriptor 表示

descriptor 采用小型 copy handle：

```rust
pub struct TypeDescriptor<'ctx> {
    metadata: &'static TypeMetadata,
    context: &'ctx ReflectionContext,
}
```

动态解析产生的有效方法等 context-owned 数据使用 `&'ctx` 引用；源码固有 metadata 使用 `&'static` 引用。descriptor 不进行引用计数分配，也不缓存可变状态。

所有导航遵守 context 传递不变量：

```text
descriptor.context() == descriptor 导航所得所有子 descriptor.context()
```

动态值例外地不保存 context，只保存静态类型身份和 adapter；调用 `ty_in(context)` 时才绑定观察环境。

### 24.6 宏实现管线

derive crate 保留“解析—规范化—验证—生成”四阶段：

```text
syn AST
  └── parse：完整提取源码事实
       └── normalize：形成与 Rust 语义一致的 IR
            └── validate：只检查单声明可判定问题
                 └── emit：生成 codegen_v4 metadata 和 fragment
```

`#[reflect(skip)]` 在 parse/normalize 阶段删除对应反射成员。其他动态能力不得通过 helper attribute 写入 IR，而必须从语法、trait bound 或独立 registration 推导。

跨 crate 冲突不由宏猜测，在 context 构建阶段统一诊断。宏生成路径中不得出现 `codegen_v3` 类型。

## 25. 破坏性迁移方案

### 25.1 迁移原则

- 本次为明确的破坏性版本，不维护旧公开 API 的兼容层；
- 内部实现允许 v3 与 v4 短期并存，以保持每个中间提交可编译；
- crate 根切换到 vNext 后，不保留 deprecated `ReflectRegistry`、`CapabilityKey` 或 `require_*()` alias；
- 删除旧代码前必须先迁移所有仓库内调用点、UI fixture、示例和文档；
- 不通过大规模路径移动掩盖语义变化，每一阶段先建立测试再切换实现。

### 25.2 实现阶段

#### 阶段 1：冻结 vNext 公共契约

- 将本设计中的公开签名编码为编译期 API contract 测试；
- 为 `field()`、`method()`、`lookup_method()`、`type_by_name()` 和 `lookup_type_by_name()` 建立最小用例；
- 建立“不存在公开 `require_*`、`ReflectRegistry`、`CapabilityKey`”的 UI/文档检查；
- 记录当前 benchmark 基线，避免后续无法判断回归来源。

#### 阶段 2：值、身份和错误基础

- 实现统一 `ValueMode` 与三种动态 wrapper；
- 实现独立诊断错误和 `*Failure<Recovery>`；
- 实现 `Lookup<T>`、`LookupError`、身份摘要和稳定候选顺序；
- 先迁移 field access、downcast 和 invocation recovery，再迁移更高层 API。

#### 阶段 3：静态 metadata 与基础 descriptor

- 建立 `Reflect::__reflect_registration()` 和类型化 `TypeRegistration<T>`；
- 实现不触发 context 初始化的 `TypeDescriptor::of`、字段、variant、类型种类和类型表达式；
- 对未实现 `Reflect` 的成员类型保留 `TypeExpression`，并验证 `find_concrete_type() == None`；
- 实现严格单数 accessor、集合 `.get()` 与 `find_*()` 命名规则。

#### 阶段 4：ReflectionContext

- 将现有 registry 构建算法包入新的不可变 `ReflectionContext`；
- 实现全局惰性初始化、失败缓存和显式 builder；
- 建立名称、定义、trait/impl、方法和 capability 索引；
- 让全部公开 descriptor 绑定 context，并删除调用点传 registry 的参数。

#### 阶段 5：可执行反射

- 依次迁移字段动态访问、构造/更新、方法 invocation、panic 捕获和 pinned receiver；
- 在每条执行路径中先验证再进入 adapter；
- 验证所有前置失败都归还约定的输入，用户代码开始执行后不承诺回滚；
- 接入方法 specialization 和独立 method binding registration。

#### 阶段 6：Trait、Impl、泛型和 capability

- 实现定义/application 分离及关联项最终绑定；
- 实现有效方法合并、默认方法来源和命名空间歧义；
- 实现类型化 `CapabilitySpec`、注册冲突和 adapter contract 校验；
- 验证泛型定义存在但 runtime 不执行任意单态化或 trait solving。

#### 阶段 7：codegen_v4 与宏表面

- 新增完整 `__private::codegen_v4`；
- 让 `#[Reflect]` 和 `#[ReflectImpl]` 自动采集全部源码事实；
- 删除除 `#[reflect(skip)]` 外的 helper attribute 解析和生成逻辑；
- 实现独立 registration 宏；
- 切换 derive crate 生成 v4 协议，并用版本不匹配 fixture 验证编译期失败。

#### 阶段 8：公开切换与清理

- crate 根切换到本设计的模块和重导；
- 迁移所有 examples、README、用户指南和 test crates；
- 删除 v3 public facade、兼容 alias、旧 helper attribute fixture 和无引用实现；
- 最后删除 `codegen_v3`，避免提前破坏宏/runtime 的可编译中间状态。

### 25.3 用户代码迁移表

| 旧写法 | vNext 写法 |
|---|---|
| `ReflectRegistry::initialize()?` | 普通路径删除；需要预热时 `ReflectionContext::global().initialize()?` |
| `registry.type_of::<T>()` | `context.type_of::<T>()` 或 `TypeDescriptor::of::<T>()` |
| `descriptor.method(name, registry)` | `descriptor.method(name)` |
| `require_field(name)` | `field(name)` |
| `require_method(name)` | `method(name)` |
| 原 `field(name) -> Option<_>` | `fields().get(name)` |
| 原 `method(name) -> Lookup<_>` | `lookup_method(name)` |
| `require_type_by_name(name)` | `type_by_name(name)` |
| 原 `type_by_name(name) -> Lookup<_>` | `lookup_type_by_name(name)` |
| `CapabilityKey<A>` 查询 | `capability::<C>()` |
| `#[reflect(rename = "...")]` | 删除；外部协议名称由上层框架维护 |
| `#[reflect(no_invoke)]` | 删除；从签名自动推导 `MethodAvailability` |
| `#[reflect(catch_unwind)]` | 调用点选择 `invoke_catching()` |
| `#[reflect(thread_safe)]` | 使用 `ThreadSafe` mode 和 Rust trait bound |

## 26. 验证计划与验收标准

### 26.1 测试层次

| 层次 | 验证内容 | 建议位置 |
|---|---|---|
| 单元测试 | 索引、稳定排序、lookup、错误路径、恢复边界、表达式与 identity | `src/tests/**` |
| 模块集成测试 | descriptor、access、invoke、construct、value、capability | `tests/descriptor/**`、`tests/access/**`、`tests/invoke/**`、`tests/value/**` |
| 跨 crate 测试 | linked fragment、crate 多版本、impl 聚合、feature 组合 | `test-crates/**`、`tests/registry/**` |
| UI 测试 | 宏诊断、非法 helper attribute、trait bound、生命周期和线程模式 | `tests/ui/**` |
| 文档测试 | crate 根示例、README、用户指南核心路径 | `src/lib.rs`、`README*.md`、`doc/user_guide*.md` |
| fuzz | fragment 组合、类型表达式、动态操作和恢复不变量 | `fuzz/**` |
| benchmark | context 初始化、descriptor lookup、动态访问和 invocation | `benches/**` |

### 26.2 必测语义矩阵

#### Context

- 全局 context 在并发首次访问时只构建一次；
- 初始化错误被缓存且可重复观察；
- 显式 context 相互隔离，不修改全局 context；
- `TypeDescriptor::of`、字段和 variant 导航不触发 context 初始化；
- context 销毁后 descriptor 不能逃逸，由借用检查器保证。

#### 查找

- 单数 accessor 对唯一结果成功，对缺失返回 `LookupError::Missing`；
- `method()`、`type_by_name()` 等对多候选返回 `LookupError::Ambiguous`；
- `lookup_*()` 精确区分三种状态并保留全部候选；
- 集合 `.get()` 对缺失返回 `None`；
- 候选顺序在不同链接顺序和多次运行间保持确定；
- 同 crate 多版本的同名类型可共存且不会错误合并。

#### Descriptor 与 metadata

- 所有子 descriptor 继承相同 context；
- 未反射成员类型仍保留准确 `TypeExpression`；
- `find_concrete_type()` 对无 adapter 类型返回 `None`；
- 字段、variant、泛型、trait、impl 和关联项均保留源码顺序与原始名称；
- 不存在通过 helper attribute 重写定义事实的路径。

#### 动态值与恢复

- owned/ref/mut 生命周期和别名规则由类型系统与 UI 测试共同覆盖；
- downcast 只接受精确类型身份；
- 失败时 owned value、replacement、Invocation 和 construction input 按契约完整归还；
- `ThreadSafe` 只接受满足相应 bound 的值；
- DST 指针 metadata 正确保存和恢复；
- 每个 owned value 恰好析构一次。

#### Invocation、构造和 capability

- receiver、参数、返回值、借用来源和 pinned receiver 全覆盖；
- 普通 `invoke()` 传播 panic，`invoke_catching()` 仅在可用平台捕获；
- `panic = "abort"` fixture 不宣称可捕获；
- struct、tuple struct、unit struct 和所有 enum variant 形状均可验证；
- capability ID、adapter 类型、目标和来源冲突分别产生结构化错误；
- trait 默认方法、impl 覆盖和跨 namespace 同名方法解析正确。

#### 宏与 feature

- `#[reflect(skip)]` 是唯一接受的 helper attribute；
- 被删除的 helper attribute 产生明确编译错误；
- `derive` 关闭时 runtime API 仍可编译；
- `ecosystem-types` 和 `qubit-types` 各 feature 组合不改变核心语义；
- 使用 `#![forbid(unsafe_code)]` 的用户 crate 可以正常使用生成代码；
- runtime 与 derive 的 codegen 协议不匹配时编译失败，而不是运行时失败。

### 26.3 性能守门

基线至少记录：

- 全局 context 首次初始化耗时和峰值分配；
- 已初始化后的 `type_by_id`、`type_by_name`、`method` 和 capability 查询；
- typed 与 dynamic 字段读取/替换；
- Invocation 参数验证和 adapter 调用；
- 大量 linked fragment 下的构建时间。

性能目标不是零开销承诺，而是：

- 类型固有导航不因 context 引入额外分配；
- descriptor clone/copy 不分配；
- 热查询不重复构建候选集合；
- vNext 相比迁移前基线出现显著回归时必须有测量结果和设计理由。

### 26.4 完成定义

只有同时满足以下条件，vNext 才算完成：

1. 本文列出的公开 API 在 crate 根和对应模块可用；
2. 所有公开 descriptor 都绑定 context，动态值除外；
3. 公开表面不存在 `ReflectRegistry`、`CapabilityKey`、`ReflectedType` 和 `require_*()`；
4. helper attribute 仅剩 `#[reflect(skip)]`；
5. 全部单元、集成、跨 crate、UI 和 doctest 通过；
6. 默认 feature、无默认 feature 和支持的 feature 组合通过检查；
7. fuzz smoke test 和关键 benchmark 已运行并记录；
8. README、双语用户指南、示例和 rustdoc 已迁移；
9. `codegen_v3` 已在所有 v4 调用点迁移完成后删除；
10. 全文 API 名称、错误类型和返回值与实现一致。
