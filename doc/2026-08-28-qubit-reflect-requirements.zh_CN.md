# `qubit-reflect` 最终需求规范

- 日期：2026-08-28
- 最近审核：2026-08-29
- 状态：需求确认稿；审核占位符已经全部关闭
- 适用范围：`qubit-reflect` 的声明宏、运行时 descriptor、动态访问、动态调用与动态构造能力
- 面向读者：架构审核者、实现者、测试者、下游框架开发者和后续用户手册维护者
- 参考文档：`rs-model-derive` 的[最终需求规范](../../rs-model-derive/doc/2026-08-28-rs-model-derive-requirements.md)
  与[目标 API 用户手册](../../rs-model-derive/doc/2026-08-28-rs-model-derive-target-api-guide.zh_CN.md)

## 0. 文档定位与规范用语

本文描述重构完成后 `qubit-reflect` 必须提供的最终能力和可观察行为，不描述现有代码，也不规定实现步骤。
本文既是实现和测试的对齐标准，也是后续编写 API 手册的事实来源。

本文参考 Java Reflection 中 `Class`、`Field`、`Method`、`Constructor`、`Parameter` 等对象化描述方式，
但不会照搬 Java 的运行时模型。Rust 的所有权、借用、可见性、泛型单态化和缺少原生运行时反射等约束优先；
尤其不得提供类似 `setAccessible(true)` 的权限绕过机制。

规范用语如下：

- “必须”表示实现与调用者不可偏离；
- “应”表示默认必须遵循，只有补充明确规范后才可偏离；
- “不得”表示禁止；
- “可以”表示允许但不强制；

每条可验收约束均有独立需求编码，本文不保留未决审核占位符。

文中的 Rust 接口表示已确认的公共 API 方向；`ignore` 示例可以省略与主题无关的细节，但不得改变能力和安全契约。
若示例签名与带编码需求冲突，以带编码需求为准。

本文定义最终目标能力，不按“首版先省略、后续再补齐”的阶段性思路降低设计完整度。所有影响安全边界、能力范围或
公共 API 形态的审核问题都必须形成明确结论；某项能力可以因 Rust 语言安全边界或组件职责而被明确列为非目标，但不能
只以“以后再实现”为理由留空。

## 1. 系统概要

### 1.1 系统目标

`qubit-reflect` 为显式选择加入的 Rust 类型建立一套基于过程宏的反射机制。它必须能够：

1. 描述 struct、enum、trait、inherent impl 和 trait impl；
2. 描述字段、variant、方法、参数、返回值、receiver、可见性和类型关系；
3. 通过 `T: Reflect` 按 Rust 类型取得根 descriptor，并在根 descriptor 下按名称或索引查询成员；
4. 在保持 Rust 类型安全和借用规则的前提下动态读取、写入字段；
5. 校验参数后动态调用已登记的方法；
6. 动态构造具名 struct、tuple struct、newtype、unit struct 及 enum variant；
7. 为 `Model`、`Enum`、`Value`、`Entity`、`Projection` 等上层领域抽象提供统一的结构事实。

典型声明如下：

```rust
use qubit_reflect::{reflect, reflect_impl, Reflect};

#[derive(Reflect)]
pub struct User {
    pub id: Id,
    username: String,
}

#[reflect]
pub trait Identifiable {
    fn id(&self) -> Id;
}

#[reflect_impl]
impl User {
    pub fn display_name(&self) -> String {
        self.username.clone()
    }

    pub fn set_password(&mut self, password: String) {
        // ...
    }
}

#[reflect_impl]
impl Identifiable for User {
    fn id(&self) -> Id {
        self.id
    }
}
```

### 1.2 逻辑组件

| 组件 | 主要职责 | 不负责的内容 |
| --- | --- | --- |
| 声明宏 | 解析 `#[derive(Reflect)]`、`#[reflect]`、`#[reflect_impl]` 并生成静态描述和安全适配代码 | 不扫描未标注源码，不调用 rustc 私有 API |
| 类型 descriptor | 描述类型身份、shape、字段、variant、方法和 impl | 不承载 Entity、identifier、validation 等领域语义 |
| 成员 descriptor | 描述字段、方法、参数、receiver、variant 和可见性 | 不绕过 Rust 的借用与调用约束 |
| trait/impl descriptor | 描述 trait 声明以及某类型的 inherent/trait impl | 不推断未标注 impl，不枚举无限 blanket impl 实例 |
| 动态值边界 | 对擦除类型后的借用值、可变值和拥有值做受检转换 | 不用字符串序列化代替 Rust 值 |
| 动态操作 | 执行字段读写、方法调用和类型/variant 构造 | 不执行未登记项，不隐式转换不兼容值 |
| descriptor 汇聚 | 将类型主体与分散在不同文件中的 impl 描述合并为稳定查询视图 | 不声称知道用户漏标的 impl |
| 下游适配层 | 供 model metadata、schema、序列化、绑定、测试生成等系统消费 | 不允许上层领域概念反向污染基础反射层 |

### 1.3 依赖方向

```text
Rust declarations
  ├─ #[derive(Reflect)] ───────┐
  ├─ #[reflect] trait ─────────┼─> qubit-reflect descriptors and operations
  └─ #[reflect_impl] impl ─────┘                  │
                                                  ▼
                                      qubit-model-metadata
                                                  │
                                                  ▼
                                    schema / validation / DAO / tools
```

- **REQ-SYS-001**：`qubit-reflect` 必须是独立的基础设施层，不得依赖 `qubit-model-derive` 或
  `qubit-model-metadata`。
- **REQ-SYS-002**：所有 descriptor 必须是只读、不可变、可静态共享且可安全并发查询的对象。
- **REQ-SYS-003**：反射必须显式选择加入；系统不得承诺发现未使用反射宏的类型、trait 或 impl。
- **REQ-SYS-004**：descriptor 必须表达强类型结构，不得用任意字符串键值表充当核心公共 API。
- **REQ-SYS-005**：查询 descriptor 不得要求先构造被描述类型的实例。
- **REQ-SYS-006**：动态操作必须保留 Rust 的类型、所有权、借用和线程安全边界；输入不匹配时必须返回错误，
  不得产生未定义行为。
- **REQ-SYS-007**：能从当前宏输入判断的错误必须在编译期报告；只有跨声明汇聚、运行时值和动态参数相关问题
  才可作为运行时错误。
- **REQ-SYS-008**：公共 descriptor API、错误和宏行为必须具备 Rustdoc，且最终用户手册中的示例必须作为验收输入。
- **REQ-SYS-009**：descriptor 图必须允许递归类型关系，但任何遍历或格式化都不得无限递归。
- **REQ-SYS-010**：相同声明片段集合在不同编译、链接和查询中的成员次序必须确定；字段与 variant 采用源码顺序，
  分散 impl 不得采用链接器枚举顺序或首次查询线程决定次序。
- **REQ-SYS-011**：目标 API 要求 `std`，并以仓库 `Cargo.toml` 声明的 `rust-version` 作为最低支持 Rust 版本；
  `no_std`/`alloc` 属于非目标。平台保证范围以仓库 CI 明确覆盖的平台为准；不支持静态分布式注册的平台必须在
  编译期明确诊断，不得静默得到不完整 impl 集合。
- **REQ-SYS-012**：反射层只承诺结构正确和动态操作安全，不承诺 validation、唯一性、引用存在性、跨字段条件或
  其他领域合法性。
- **REQ-SYS-013**：`qubit-reflect` 第一方运行时代码及宏生成代码必须兼容 `#![forbid(unsafe_code)]`；可以依赖内部
  使用 unsafe 的第三方库，但其 unsafe 必须封装在安全 API 后，且不得把未检查不变量转嫁给 `qubit-reflect` 调用者。

## 2. 反射声明宏

### 2.1 struct 与 enum：`#[derive(Reflect)]`

`Reflect` derive 用于声明类型本体及其字段或 variant。目标支持 struct 与 enum；union 不在目标范围内。

```rust
#[derive(Reflect)]
pub struct Account {
    pub id: u64,
    owner: String,
}

#[derive(Reflect)]
pub enum LoginResult {
    Success { user_id: u64 },
    Retry(u32),
    Locked,
}
```

- **REQ-MAC-001**：struct 和 enum 必须使用 `#[derive(Reflect)]` 选择加入反射。
- **REQ-MAC-002**：除显式类型级 `#[reflect(opaque)]` 外，derive 必须记录全部直接字段，包括私有字段、tuple 字段和
  enum variant 字段；类型级 opaque 必须隐藏全部内部成员。
- **REQ-MAC-003**：非 opaque derive 必须记录字段的源码顺序、Rust 名称、声明类型和可见性；tuple 字段没有名称但有
  索引。
- **REQ-MAC-004**：derive 必须为类型实现公共查询 trait `Reflect`，使泛型代码可以声明 `T: Reflect`。
- **REQ-MAC-005**：不得接受 union；应给出指向声明位置的编译错误。

### 2.2 trait：`#[reflect]`

trait 不能使用 derive，因而使用 attribute macro：

```rust
#[reflect]
pub trait Named: Send + Sync {
    fn name(&self) -> &str;

    fn display_name(&self) -> String {
        self.name().to_owned()
    }
}
```

- **REQ-MAC-006**：trait 必须使用 `#[reflect]` 选择加入反射。
- **REQ-MAC-007**：宏必须至少描述 trait 身份、可见性、supertrait、方法签名、receiver、参数和返回类型。
- **REQ-MAC-008**：trait 宏必须描述关联类型、关联常量、泛型方法和默认方法；只有生成了明确 concrete 调用适配器的
  项才可动态执行。
- **REQ-MAC-009**：`#[reflect]` 不得改变 trait 的业务语义、object safety 或调用分派方式。

### 2.3 impl block：`#[reflect_impl]`

所有希望纳入反射结果的 impl block 均由使用方显式标注，无论它们位于哪个文件中：

```rust
// user/display.rs
#[reflect_impl]
impl User {
    pub fn display_name(&self) -> String { /* ... */ }
}

// user/identity.rs
#[reflect_impl]
impl Identifiable for User {
    fn id(&self) -> Id { /* ... */ }
}
```

trait impl 同样必须标注 `#[reflect_impl]`。trait 方法在 impl 中不能书写 `pub`；其公开性来自 trait。

- **REQ-MAC-010**：被反射 struct/enum 的每个**希望纳入反射结果**的 impl block 都必须由使用方标注
  `#[reflect_impl]`；无需反射的 impl 可以不标注。
- **REQ-MAC-011**：`#[reflect_impl]` 必须同时支持 inherent impl 与 trait impl。
- **REQ-MAC-012**：未标注的 impl block 及其中方法不得出现在反射结果中；漏标责任属于使用方，框架不报“缺失”错误。
- **REQ-MAC-013**：同一类型的已标注 impl 可以分布在多个模块和文件中，最终查询必须汇聚其描述。
- **REQ-MAC-014**：对 trait impl 生成完整 trait 声明描述时，该 trait 本身必须已使用 `#[reflect]`；未反射 trait
  的 impl 仍可生成明确标记为 external/incomplete 的描述，但不得声称知道未出现在 impl block 中的 trait 事实。
- **REQ-MAC-015**：宏不得改变原 impl 的方法解析、可见性、泛型约束或 trait 实现语义。
- **REQ-MAC-016**：过程宏必须位于独立发布的 `qubit-reflect-derive` proc-macro crate，运行时 API 位于
  `qubit-reflect`；`qubit-reflect` 默认重新导出 `Reflect` derive、`reflect` 和 `reflect_impl`，普通用户只需声明
  一个直接依赖。
- **REQ-MAC-017**：生成代码必须正确解析依赖被重命名的情况，不得硬编码只有依赖名恰为 `qubit_reflect` 才能编译；
  derive crate 不得反向依赖运行时 crate 形成 Cargo 循环依赖。

### 2.4 辅助属性

```rust,ignore
#[derive(Reflect)]
#[reflect(capabilities(Clone, Default))]
struct User {
    #[reflect(rename = "login", read_only)]
    username: String,
    #[reflect(default = default_locale)]
    locale: Locale,
}

#[reflect_impl(external_trait_id = "std.fmt.Display")]
impl Display for User { /* ... */ }
```

- **REQ-MAC-018**：未反射 trait 的 `#[reflect_impl]` 必须使用 `external_trait_id = "..."` 提供
  `ExternalTraitId`；已反射 trait 禁止重复提供该参数。缺失、格式错误或保留命名空间冲突必须编译失败。
- **REQ-MAC-019**：helper 统一使用 `#[reflect(...)]`：类型接受 `rename`/`opaque`/`capabilities(...)`，字段接受
  `rename`/`skip`/`opaque`/`read_only`/`no_construct`/`default`/`default = path`，variant 接受
  `rename`/`skip`/`no_construct`，方法接受 `rename`/`skip`/`no_invoke`/`catch_unwind`/`thread_safe`/
  `specialize(...)`，trait 接受 `rename`/`supertrait(...)`/`external_trait(...)`/`dyn_compatible`/
  `dyn_compatible(Supertrait::AssociatedType, ...)`。属性放错目标、
  同一键重复或互斥策略组合必须编译失败。
- **REQ-MAC-020**：泛型 impl 的 concrete 登记使用 `#[reflect_impl(specialize(...))]`，泛型方法使用
  `#[reflect(specialize(...))]`；specialization 必须按泛型参数名称提供完整类型/const 实参，重复、未知、遗漏或不满足
  where predicates 必须编译失败。
- **REQ-MAC-021**：反射 trait 声明中的外部 supertrait 或 bound 必须使用
  `#[reflect(external_trait(path, id = "..."))]` 显式映射到 `ExternalTraitId`；同一路径或同一 ID 的冲突映射、
  缺少映射以及保留命名空间冲突必须编译失败。映射只建立 trait 身份，不得推断外部 trait 未公开的关联项。

## 3. 公共 descriptor 模型

### 3.1 对象关系

Java Reflection 将 `Class`、`Field`、`Method`、`Constructor` 和 `Parameter` 建模为可导航对象。
`qubit-reflect` 采用同类的 descriptor 导航体验，同时增加 Rust 特有的 impl、receiver、借用和值类别：

```text
TypeDescriptor
  ├─ FieldDescriptor[] ──> TypeRef ──> resolved / opaque / symbolic
  ├─ VariantDescriptor[] ──> FieldDescriptor[] ──> TypeRef
  └─ ImplDescriptor[]
       ├─ inherent
       └─ trait ──> TraitDescriptor
             └─ MethodDescriptor[] ──> ParameterDescriptor[]

TraitDescriptor
  ├─ supertraits[]
  └─ MethodDescriptor[]
```

- **REQ-DESC-001**：每种由公共查询方法返回的 descriptor 类型必须有完整、公开、可导航的只读接口。
- **REQ-DESC-002**：descriptor 之间的引用必须具有静态或与根 descriptor 一致的有效期，不得返回悬垂引用。
- **REQ-DESC-003**：类型、trait、字段、variant、impl、方法和参数必须各自具有明确身份，不得只靠 Debug 文本比较。
- **REQ-DESC-004**：Rust 名称和面向查询的名称必须区分；未使用重命名属性时二者相同。
- **REQ-DESC-005**：descriptor 必须能够区分“声明在类型本体的成员”和“由某个 impl 提供的成员”。
- **REQ-DESC-006**：descriptor 身份必须可用于结构化错误、去重和限定查询；类型使用当前进程 `TypeId`，已反射
  trait 使用 marker `TraitId`，成员使用“所属 descriptor 身份 + 成员类别 + 声明序号/impl 片段身份”的复合身份。
  所有身份必须实现当前进程内一致的 `Eq`/`Hash`，但不承诺跨编译、跨进程或持久化相等。
- **REQ-DESC-007**：`#[text]` 等领域属性不得成为内建 descriptor 语义。反射层不提供任意 Rust attribute 的通用
  发现，也不公开 `syn`/`TokenStream`；只描述本规范定义的 `#[reflect(...)]` 辅助属性。领域宏直接生成其上层
  metadata。
- **REQ-DESC-008**：impl 片段身份必须由声明 crate、模块路径、源码位置和宏生成的内容指纹构成；同一身份重复注册、
  或相同身份对应不同内容，都必须产生确定性汇聚错误。对象地址和链接顺序不得成为身份组成部分。
- **REQ-DESC-009**：可见性必须归一化为 public、crate、super、restricted path 与 private；`pub(self)` 和无修饰符
  归入 private，原 restricted path 保留为诊断路径。trait item 和 enum variant 字段必须明确标记其可见性来自所属
  声明，不得伪造 item 自身的 `pub`。
- **REQ-DESC-010**：`TypeExpression` 必须是与 `syn` 解耦的结构化树，至少表达 concrete type、类型参数、`Self`、
  关联类型、reference、raw pointer、slice、array、tuple、function pointer、dyn trait、opaque `impl Trait` 与 never；
  generic arguments、bounds 和 `LifetimeExpression` 必须可导航，源码文本只作诊断补充。
- **REQ-DESC-011**：type、trait、field、variant 和 method descriptor 必须分别公开不可变的 Rust name/path 与 query
  name；所有按名称 lookup 使用 query name，诊断同时包含两者。未 rename 时二者相同；rename 不得改变 Rust 身份、
  `type_name()` 或成员复合身份。

### 3.2 `Reflect` 与 `TypeDescriptor`

以下接口表示已确认的目标形态；实现可以按 Rust 命名惯例细化辅助类型，但不得改变导航关系和错误契约：

```rust,ignore
pub trait Reflect: 'static {
    fn type_descriptor() -> &'static TypeDescriptor;
}

pub enum TypeKind {
    Primitive(PrimitiveKind),
    Text(TextKind),
    Struct(StructKind),
    Enum,
    Tuple,
    Array,
    Optional,
    Sequence,
    Set,
    Map,
    SmartPointer(SmartPointerKind),
    Reference(ReferenceKind),
    Slice,
    RawPointer(Mutability),
    FunctionPointer(FunctionPointerKind),
    TraitObject,
    Opaque,
}

pub enum TypeRef {
    Resolved(&'static TypeDescriptor),
    Opaque(&'static OpaqueTypeDescriptor),
    Symbolic(TypeExpression),
}

impl OpaqueTypeDescriptor {
    pub fn type_id(&self) -> TypeId;
    pub fn type_name(&self) -> &'static str;
}

pub enum StructKind {
    Named,
    Tuple,
    Newtype,
    Unit,
}

pub enum Local {}
pub enum ThreadSafe {}

pub struct DynamicRef<'a, Mode> { /* ... */ }
pub struct DynamicMut<'a, Mode> { /* ... */ }
pub struct DynamicOwned<Mode> { /* ... */ }

pub type ReflectedRef<'a> = DynamicRef<'a, Local>;
pub type ReflectedMut<'a> = DynamicMut<'a, Local>;
pub type ReflectedOwned = DynamicOwned<Local>;
pub type SendReflectedRef<'a> = DynamicRef<'a, ThreadSafe>;
pub type SendReflectedMut<'a> = DynamicMut<'a, ThreadSafe>;
pub type SendReflectedOwned = DynamicOwned<ThreadSafe>;

impl TypeDescriptor {
    pub fn of<T: Reflect + ?Sized>() -> &'static TypeDescriptor;

    pub fn type_id(&self) -> TypeId;
    pub fn type_name(&self) -> &'static str;
    pub fn query_name(&self) -> &'static str;
    pub fn kind(&self) -> TypeKind;

    pub fn as_struct(&self) -> Option<&StructTypeDescriptor>;
    pub fn as_sequence(&self) -> Option<&SequenceTypeDescriptor>;
    pub fn as_map(&self) -> Option<&MapTypeDescriptor>;
    pub fn as_function(&self) -> Option<&FunctionTypeDescriptor>;
    // 其他 kind 对应同类 typed view。

    pub fn fields(&self) -> &[FieldDescriptor];
    pub fn field(&self, name: &str) -> Option<&FieldDescriptor>;
    pub fn field_at(&self, index: usize) -> Option<&FieldDescriptor>;

    pub fn variants(&self) -> &[VariantDescriptor];
    pub fn variant(&self, name: &str) -> Option<&VariantDescriptor>;
    pub fn variant_at(&self, index: usize) -> Option<&VariantDescriptor>;

    pub fn impls(&self) -> Result<&[ImplDescriptor], RegistryError>;
    pub fn methods(&self) -> Result<&[MethodDescriptor], RegistryError>;
    pub fn methods_named(&self, name: &str)
        -> Result<MethodCandidates<'_>, RegistryError>;
}
```

```rust,ignore
let user = TypeDescriptor::of::<User>();
assert_eq!(user.type_id(), TypeId::of::<User>());
assert_eq!(user.type_name(), std::any::type_name::<User>());
assert_eq!(user.query_name(), std::any::type_name::<User>());
assert_eq!(user.kind(), TypeKind::Struct(StructKind::Named));

let username = user.field("username").unwrap();
assert_eq!(username.index(), 1);
assert_eq!(username.rust_name(), Some("username"));
assert_eq!(username.query_name(), Some("username"));
```

- **REQ-TYPE-001**：已知且实现 `Reflect` 的 Rust 类型必须可以通过 `TypeDescriptor::of::<T>()` 取得唯一静态
  根 descriptor；未实现 `Reflect` 的类型不得调用该入口。
- **REQ-TYPE-002**：`Reflect::type_descriptor()` 与 `TypeDescriptor::of::<T>()` 必须返回同一对象。
- **REQ-TYPE-003**：`type_id()` 表示当前进程内的 Rust `TypeId`，不得当作跨编译、跨进程或持久化稳定 ID。
- **REQ-TYPE-004**：`type_name()` 必须提供 Rust 类型诊断名，但调用方不得把它当作稳定协议 ID。
- **REQ-TYPE-005**：struct 的 `fields()` 返回直接字段；enum 根类型的字段为空，variant 字段从 `VariantDescriptor` 查询。
- **REQ-TYPE-006**：不适用于当前 kind 的导航必须返回空集合或 `None`，不得 panic。
- **REQ-TYPE-007**：内建描述范围必须包含 primitive、`String`、`()`、`Option<T>`、`Vec<T>`、数组、tuple、
  `str`、slice、`Box<T>`、`Rc<T>`、`Arc<T>`、`HashMap<K, V>`、`BTreeMap<K, V>`、`HashSet<T>` 和
  `BTreeSet<T>`。内建描述不等于所有容器都必须支持通用动态构造。
- **REQ-TYPE-008**：所有 lookup 必须区分“没有结果”和“名称歧义”；方法查找尤其不得静默选中任意同名项。
- **REQ-TYPE-009**：`Reflect` 是 `qubit-reflect` 对“类型可提供静态 descriptor”能力的唯一公共泛型约束名称；
  `rs-model-derive`/`qubit-model-metadata` 不得另行定义同义的 `HasTypeDescriptor` 体系。
- **REQ-TYPE-010**：`TypeDescriptor::of::<T>()` 不得只适用于 derive 的 struct/enum；必须为
  `REQ-TYPE-007` 列出的标量和组合类型提供 `Reflect`。被反射声明中的字段默认要求其 concrete 字段类型实现
  `Reflect`；框架不得因查询顺序、当前 registry 内容或可见 impl 集合而自动把缺少 `Reflect` 的类型降级为 opaque。
  类型作者只能通过字段上的 `#[reflect(opaque)]` 显式选择不解析该字段内部结构，此时字段类型只需满足 `'static`。
- **REQ-TYPE-011**：类型 shape 必须能区分具名、tuple/newtype 与 unit struct；仅返回笼统的
  `TypeKind::Struct` 不足以驱动 `REQ-CON-001` 要求的正确构造入口；必须通过 `StructKind` typed view 取得准确 shape。
- **REQ-TYPE-012**：`TypeDescriptor::of::<T>()` 是静态查询主入口；全局 `ReflectRegistry` 必须支持按 `TypeId`
  查找、枚举全部已注册根 descriptor，以及按诊断性 `type_name` 返回零到多个候选。名称查询不得静默选中一个结果，
  `type_name` 不得升级为稳定模型 ID。
- **REQ-TYPE-013**：`TypeKind` 必须采用分层类别模型，至少区分 primitive、text、struct、enum、tuple、array、optional、
  sequence、set、map、smart pointer、reference、slice、raw pointer、function pointer、trait object 与 opaque；
  struct 再区分 named、tuple、newtype 与 unit。`()` 必须归入 arity 为 0 的 tuple，不得再建 `Unit` primitive 或
  独立顶层 kind。闭包和不可命名的 function item 只能在声明级 `TypeExpression` 中按 opaque 表达，不得伪装成
  函数指针，也不得自动获得根 descriptor。
  不得为每个具体标准库类型建立无导航关系的扁平顶层 variant。
- **REQ-TYPE-014**：必须提供可扩展的 `TypeCapabilities`，内建能力至少包含 `Send`、`Sync`、`Clone` 与
  `Default`，并允许上层注册不会与内建项或其他扩展冲突的 capability。未知扩展能力必须可被保留、比较和转发；
  capability 不得绕过对应操作本身的类型检查和安全入口。
- **REQ-TYPE-015**：opaque member descriptor 必须允许字段整体读取、写入、方法传参和作为 struct/variant 构造输入，
  且每次操作均要求 `TypeId` 完全匹配；不得导航其内部结构，也不得通过 opaque member descriptor 直接动态构造
  该类型的独立根值。
- **REQ-TYPE-016**：capability 必须使用带命名空间的稳定 `CapabilityId`；ID 采用由点号分隔的非空标识符段，
  `qubit.reflect.*` 命名空间由本库保留，第三方必须使用其自身稳定命名空间。ID 不得依赖进程内动态分配的 bit 位，
  也不得因链接顺序改变。
- **REQ-TYPE-017**：内建类型的 core capability 由库登记；用户类型必须通过 derive 辅助属性或显式注册声明其
  capability。对 `Send`、`Sync`、`Clone`、`Default` 等对应 Rust trait 的声明，生成代码必须用 trait bound 在
  编译期验证；虚假声明必须编译失败，不得降级为运行时错误。
- **REQ-TYPE-018**：每个 capability descriptor 可以携带可选的安全动态操作适配器。core `Clone` 必须提供动态
  clone，core `Default` 必须提供动态默认构造；扩展 capability 可以定义自己的受检操作，但必须通过其
  `CapabilityId` 和公开契约取得，不得依赖未检查的类型转换。
- **REQ-TYPE-019**：`TypeCapabilities` 必须是不可变、确定性迭代的 capability descriptor 集合；重复 ID、同一 ID
  绑定不兼容契约或保留命名空间冲突必须产生编译期诊断，只有跨链接单元无法提前判断的冲突才可在汇聚初始化时失败。
- **REQ-TYPE-020**：`kind()` 必须返回稳定的分层类别；每个具有专属结构的 kind 必须通过 `as_struct()`、
  `as_sequence()`、`as_map()`、`as_function()` 等方法进入只包含适用导航的 typed view。不得把所有
  `element_type()`、`key_type()`、`pointee_type()` 等不相干方法平铺到根 `TypeDescriptor`。
- **REQ-TYPE-021**：registry 只包含参与最终链接的已反射类型和内建类型；`TypeDescriptor::of::<T>()` 不依赖全局
  名称查询且在结构 descriptor 本身合法时始终成功。涉及分散 impl 的聚合查询必须显式返回 `RegistryError`，不得因
  registry 冲突让纯字段或 variant 查询 panic。
- **REQ-TYPE-022**：`Reflect` 和 `TypeDescriptor::of<T>()` 必须支持 `T: ?Sized + 'static`，使 `str`、`[T]` 和
  dyn-compatible `dyn Trait` 可拥有 descriptor。除 `str` 的内建安全借用适配器外，受 `Any` 安全转换的 `Sized`
  限制，unsized 类型只提供描述和导航，不能直接进入动态值包装、动态构造或调用实参；框架不得用裸指针绕过该限制。
- **REQ-TYPE-023**：`PrimitiveKind` 必须完整区分 `bool`、`char`、所有固定宽度有符号/无符号整数、`isize`、
  `usize`、`f32` 和 `f64`；`String` 与 `str` 必须归入独立 `Text(TextKind)`，分别区分 owned UTF-8 string 与
  borrowed UTF-8 string slice，不得把 `str` 当作元素为 `char` 的普通 slice。
- **REQ-TYPE-024**：container/smart-pointer typed view 必须记录准确的标准库 family 和全部类型参数：map 的 key/value、
  set/sequence/optional/slice 的 element、array 的 element/length、tuple 的有序元素、`Box/Rc/Arc` 的 pointee；引用和
  裸指针还必须记录 mutability，函数指针记录 ABI、safety、variadic、参数和返回表达式。
- **REQ-TYPE-025**：`ReflectRegistry` 必须分别提供 `find_by_type_name` 与 `find_by_query_name`，两者都返回候选集合；
  跨 crate 的 query-name 冲突允许存在但必须由调用者限定或消歧，单个类型内部的成员 query-name 冲突仍在编译期失败。
- **REQ-TYPE-026**：capability 集合描述“已登记且具有适配器的能力”，不是稳定 Rust 上对任意 trait 实现的负向
  反射；查询缺失表示 `NotRegistered`，不得解释为类型一定未实现该 Rust trait。动态包装构造仍以调用点泛型 bound
  为准，不能仅因 descriptor 未登记 `Send`/`Sync` 就拒绝一个编译期已证明安全的值。
- **REQ-TYPE-027**：内部 `DescriptorInterner` 与公开 `ReflectRegistry` 必须分离：前者可以按需缓存
  `TypeDescriptor::of::<T>()` 产生的 generic/composite concrete descriptor；后者只枚举静态提交的类型定义、
  内建定义和显式 concrete registration。按需查询不得在初始化后改变公开 registry 枚举结果。
- **REQ-TYPE-028**：字段类型引用必须使用 `TypeRef`：可完整解析且实现 `Reflect` 的 concrete 字段使用
  `Resolved`，显式 `#[reflect(opaque)]` 字段使用 `Opaque`，泛型定义级尚未单态化的字段使用 `Symbolic`。
  `OpaqueTypeDescriptor` 只保存 `TypeId`、诊断性 `type_name` 与受检整体操作适配器，是所属
  member 的不透明类型视图，不是第二个根 `TypeDescriptor`；即使该 concrete 类型另处实现了 `Reflect`，显式 opaque
  member 仍保持 opaque，且不得破坏该类型根 descriptor 的唯一性。
- **REQ-TYPE-029**：类型作者可以在 derive 类型上显式使用 `#[reflect(opaque)]`，生成该类型唯一的、kind 为
  `TypeKind::Opaque` 的根 descriptor；它不公开字段、variant 或内部导航，但仍可承载声明过且经编译期验证的
  capability。除 capability 自身提供的 adapter 外，它不支持逐字段/variant 构造。类型级 opaque 与字段级 opaque
  都必须是源码显式策略，禁止自动推断或降级。

## 4. 字段反射

### 4.1 功能和场景

字段反射用于 schema 生成、对象检查、数据绑定、测试构造和上层 Model Property 推导。显式 derive 即表示类型作者
允许反射层为该类型生成安全访问适配器，包括私有字段；它不等于改变字段在普通 Rust 代码中的可见性。

```rust
#[derive(Reflect)]
struct User {
    pub id: u64,
    username: String,
}
```

目标基础接口：

```rust,ignore
impl FieldDescriptor {
    pub fn declaring_type(&self) -> &'static TypeDescriptor;
    pub fn index(&self) -> usize;
    pub fn rust_name(&self) -> Option<&'static str>;
    pub fn query_name(&self) -> Option<&'static str>;
    pub fn field_type(&self) -> &TypeRef;
    pub fn visibility(&self) -> FieldVisibility;

    pub fn get<'a>(&self, target: ReflectedRef<'a>)
        -> Result<ReflectedRef<'a>, FieldAccessError>;
    pub fn get_mut<'a>(&self, target: ReflectedMut<'a>)
        -> Result<ReflectedMut<'a>, FieldAccessError>;
    pub fn set(&self, target: ReflectedMut<'_>, value: ReflectedOwned)
        -> Result<(), FieldAccessError>;
}
```

`ReflectedRef`、`ReflectedMut` 和 `ReflectedOwned` 是字段、构造等同步就地操作的规范本地 mode。线程安全 mode 的
`SendReflectedRef`、`SendReflectedMut` 和 `SendReflectedOwned` 用于跨线程传递，并可安全、不可逆地降级为相应本地
包装后调用这些 API；只有显式登记的线程安全方法 adapter 才在调用链中保持 `ThreadSafe` mode。不要求用户再实现或
引用第二个公共 `ReflectValue` trait；这些包装类型必须通过受检泛型入口从 Rust 值构造。

```rust,ignore
let mut user = User { id: 7, username: "alice".into() };
let username = TypeDescriptor::of::<User>().field("username").unwrap();

assert_eq!(
    username.get(ReflectedRef::new(&user))?.downcast_ref::<String>(),
    Some(&"alice".into()),
);
username.set(
    ReflectedMut::new(&mut user),
    ReflectedOwned::new(String::from("bob")),
)?;
assert_eq!(
    username.get(ReflectedRef::new(&user))?.downcast_ref::<String>(),
    Some(&"bob".into()),
);
```

- **REQ-FLD-001**：每个字段必须记录 declaring type、源码索引、可选名称、字段类型和源码可见性。
- **REQ-FLD-002**：具名字段按 query name 与索引查询；tuple/newtype 字段仅按索引查询，`rust_name()` 和
  `query_name()` 都返回 `None`。
- **REQ-FLD-003**：私有字段必须可被 descriptor 查询；普通 Rust 字段访问规则不因此改变。
- **REQ-FLD-004**：字段读取返回的借用不得超过 target 借用期；可变读取期间不得再制造别名可变借用。
- **REQ-FLD-005**：字段写入必须检查 target 类型与 value 类型，完全匹配后才可修改目标。
- **REQ-FLD-006**：任何检查失败都不得部分修改目标。
- **REQ-FLD-007**：读取或写入错误必须包含字段身份、期望类型和实际类型（可获得时）。
- **REQ-FLD-008**：字段描述不得自动把 getter/setter 方法合并为 Field；上层可以另行推导 Property。
- **REQ-FLD-009**：enum variant 字段在 Rust 源码中没有独立可见性修饰符；其 descriptor 必须明确记录“继承
  enum/variant 的可访问边界”或等价事实，不得把它伪装成源码显式 `pub` 字段。
- **REQ-FLD-010**：未标记 `#[reflect(opaque)]` 的 concrete 字段必须以 `TypeRef::Resolved` 导航到唯一根
  `TypeDescriptor`；字段类型未实现 `Reflect` 时 derive 必须给出定位到该字段的编译错误。
- **REQ-FLD-011**：标记 `#[reflect(opaque)]` 的字段必须以 `TypeRef::Opaque` 描述，无论同一类型是否也实现
  `Reflect` 都不得自动升级为 resolved；其读写、传参与外层构造仍按准确 `TypeId` 受检。

## 5. enum 与 variant 反射

```rust
#[derive(Reflect)]
enum Event {
    Started,
    Progress(u8),
    Failed { code: u32, message: String },
}
```

```rust,ignore
impl VariantDescriptor {
    pub fn declaring_type(&self) -> &'static TypeDescriptor;
    pub fn index(&self) -> usize;
    pub fn rust_name(&self) -> &'static str;
    pub fn query_name(&self) -> &'static str;
    pub fn kind(&self) -> VariantKind;
    pub fn fields(&self) -> &[FieldDescriptor];
    pub fn field(&self, name: &str) -> Option<&FieldDescriptor>;
    pub fn field_at(&self, index: usize) -> Option<&FieldDescriptor>;
    pub fn is_active(&self, value: ReflectedRef<'_>) -> Result<bool, TypeMismatch>;
}

pub enum VariantKind {
    Unit,
    Tuple,
    Struct,
}
```

- **REQ-VAR-001**：非类型级 opaque 的 enum descriptor 必须按源码顺序描述全部 variant；类型级
  `#[reflect(opaque)]` enum 的根 kind 为 opaque，不得泄露 variant。
- **REQ-VAR-002**：variant 必须区分 unit、tuple 和 struct 三种 shape。
- **REQ-VAR-003**：variant 内字段遵循与 struct 字段相同的索引、名称、类型、可见性和访问安全规则。
- **REQ-VAR-004**：必须可以受检判断一个 enum 值当前激活的 variant。
- **REQ-VAR-005**：尝试用非激活 variant 的字段 descriptor 读取值必须返回明确错误，不得 panic 或读取无效内存。
- **REQ-VAR-006**：每个 variant 必须记录 discriminant 是源码显式声明还是由 Rust 隐式分配；enum descriptor 必须
  记录与布局和 discriminant 相关的合法 `repr(...)` 声明，但不得把诊断文本当作数值协议。
- **REQ-VAR-007**：variant 字段 descriptor 的身份必须同时包含 enum 根类型、variant 与 variant 内字段位置；
  不同 variant 中同名或同索引字段不得被视为同一字段。
- **REQ-VAR-008**：仅 fieldless 且使用整数 `repr` 的 enum 必须提供准确的数值 discriminant 查询和按数值反查
  variant；隐式 discriminant 也必须返回编译器规则确定的最终数值。其他 enum 不得伪造统一整数 discriminant。
- **REQ-VAR-009**：按数值反查没有结果时必须返回 `None` 或结构化缺失结果；若声明本身导致重复或越界，必须沿用
  Rust 编译器诊断，不得在运行时任意选择 variant。

## 6. 方法与参数反射

### 6.1 方法范围

方法 descriptor 不只描述 Property getter/setter，而是描述每个 `#[reflect_impl]` 中可纳入反射范围的函数：

```rust
#[reflect_impl]
impl User {
    pub fn new(id: Id, username: String) -> Self { /* ... */ }
    pub fn username(&self) -> &str { &self.username }
    pub fn rename(&mut self, value: String) { self.username = value; }
    pub fn into_username(self) -> String { self.username }
}
```

目标基础接口：

```rust,ignore
impl MethodDescriptor {
    pub fn rust_name(&self) -> &'static str;
    pub fn query_name(&self) -> &'static str;
    pub fn declaring_impl(&self) -> &'static ImplDescriptor;
    pub fn visibility(&self) -> MethodVisibility;
    pub fn receiver(&self) -> Option<&ReceiverDescriptor>;
    pub fn parameters(&self) -> &[ParameterDescriptor];
    pub fn parameter(&self, name: &str) -> Option<&ParameterDescriptor>;
    pub fn parameter_at(&self, index: usize) -> Option<&ParameterDescriptor>;
    pub fn return_value(&self) -> &ReturnDescriptor;
    pub fn qualifiers(&self) -> MethodQualifiers;
    pub fn invoke(&self, invocation: Invocation<'_>)
        -> Result<InvocationOutput<'_>, InvocationError>;
}

impl ParameterDescriptor {
    pub fn index(&self) -> usize;
    pub fn name(&self) -> Option<&'static str>;
    pub fn pattern(&self) -> &ParameterPatternDescriptor;
    pub fn passing_mode(&self) -> ParameterPassingMode;
    pub fn signature_type(&self) -> &TypeExpression;
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor>;
}
```

- **REQ-MTH-001**：方法必须记录名称、所属 impl、源码可见性、receiver、非 receiver 参数、返回类型和限定符。
- **REQ-MTH-002**：receiver 必须至少区分无 receiver、`self`、`&self` 与 `&mut self`。
- **REQ-MTH-003**：参数索引不包含 receiver；参数名称用于诊断和绑定，不得视为跨源码修改的稳定 ABI。
- **REQ-MTH-004**：方法必须区分 inherent 来源与 trait 来源；trait 方法还必须能导航到对应 trait descriptor。
- **REQ-MTH-005**：同名 inherent 方法、不同 trait 的同名方法不得相互覆盖。
- **REQ-MTH-006**：必须记录 `async`、`unsafe`、`const`、`extern`、泛型等影响可调用性的事实，不能调用时仍可描述。
- **REQ-MTH-007**：关联函数必须作为没有 receiver 的方法描述；它与类型动态构造 API 是两种独立能力。
- **REQ-MTH-008**：`#[reflect_impl]` 默认描述 impl block 中的全部方法，包括 `pub(crate)` 和私有 inherent 方法；
  这是类型作者在定义位置授予的反射能力，不改变普通 Rust 可见性。动态调用还必须满足签名安全和辅助属性策略。
- **REQ-MTH-009**：descriptor 必须明确区分“可描述”和“可动态调用”。每个方法都必须能查询其调用能力；不可调用时
  必须给出结构化原因，例如不支持的 receiver、参数 pattern、泛型、ABI、unsafe 或返回借用关系。
- **REQ-MTH-010**：参数名称必须返回 `Option`；简单 identifier 为 `Some`，`_` 和解构 pattern 为 `None`，同时通过
  `ParameterPatternDescriptor` 保留 identifier/wildcard/destructure 类别和不依赖 `syn` 的诊断表示。位置调用支持
  所有可安全适配的 pattern；按名称绑定只适用于名称唯一的简单 identifier 参数。
- **REQ-MTH-011**：参数必须以 `TypeExpression` 描述声明签名，并在可解析为 concrete Rust 类型时导航到
  `TypeDescriptor`；passing mode 必须区分 owned、shared borrow 与 mutable borrow。返回值必须通过
  `ReturnDescriptor` 区分 unit、never、concrete type、reference 与 opaque `impl Trait`，不得用虚假的普通类型
  descriptor 表示 `!` 或 `impl Trait`。
- **REQ-MTH-012**：泛型方法声明必须完整描述类型、lifetime、const 参数、bounds 和 where predicates；动态调用只
  面向使用 `#[reflect(specialize(...))]` 登记的有限 concrete `MethodInstanceDescriptor`，不得在运行时猜测或枚举
  无限单态化实例。
- **REQ-MTH-013**：签名包含 `&[T]`、`&dyn Trait` 等 unsized 借用目标时必须完整描述；在不使用 unsafe 且
  基础动态值包装无法安全擦除/恢复该 unsized 值时，方法标记为不可调用，除非该 kind 登记了专用安全
  `UnsizedValueAdapter` capability。`&str`/`&mut str` 使用反射层内建的 text 借用适配器。不得用错误的 sized owner
  类型冒充精确参数类型。

## 7. trait 与 impl 反射

```rust,ignore
impl TraitDescriptor {
    pub fn trait_id(&self) -> TraitId;
    pub fn rust_name(&self) -> &'static str;
    pub fn rust_path(&self) -> &'static str;
    pub fn query_name(&self) -> &'static str;
    pub fn completeness(&self) -> TraitCompleteness;
    pub fn direct_supertraits(&self) -> &[TraitDescriptorRef];
    pub fn all_supertraits(&self) -> SupertraitClosure<'_>;
    pub fn methods(&self) -> &[MethodDescriptor];
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor>;
    pub fn associated_types(&self) -> &[AssociatedTypeDescriptor];
    pub fn associated_consts(&self) -> &[AssociatedConstDescriptor];
}

impl ImplDescriptor {
    pub fn target_type(&self) -> &'static TypeDescriptor;
    pub fn kind(&self) -> ImplKind;
    pub fn implemented_trait(&self) -> Option<&'static TraitDescriptor>;
    pub fn methods(&self) -> &[MethodDescriptor];
}

pub enum ImplKind {
    Inherent,
    Trait,
}

pub enum TraitCompleteness {
    Complete,
    ExternalIncomplete,
}

pub enum TraitId {
    Reflected(TypeId),
    External(ExternalTraitId),
}
```

```rust,ignore
let user = TypeDescriptor::of::<User>();
let identity_impl = user.impls()?.iter()
    .find(|item| item.implemented_trait().is_some())
    .unwrap();
assert_eq!(identity_impl.target_type().type_id(), TypeId::of::<User>());
```

- **REQ-TRT-001**：`TraitDescriptor` 必须独立于任何具体实现描述 trait 声明。
- **REQ-TRT-002**：`ImplDescriptor` 必须指向目标类型，并区分 inherent impl 与 trait impl。
- **REQ-TRT-003**：trait impl descriptor 必须同时关联 trait descriptor 与目标 type descriptor。
- **REQ-TRT-004**：impl 中方法必须关联到其具体实现；不得只返回 trait 声明中的抽象签名。
- **REQ-TRT-005**：目标类型的有效方法视图必须包含未覆盖的 trait 默认方法并标记为 `Defaulted`；宏必须为具体目标
  类型生成调用适配器，签名受支持时可以动态调用。显式覆盖标记为 `Overridden` 并导航到具体 impl 方法。
- **REQ-TRT-006**：trait descriptor 必须分别提供直接 supertrait 和确定、去重、防递归的传递闭包；已反射
  supertrait 导航到完整 descriptor，未反射外部 supertrait 导航到 incomplete descriptor。
- **REQ-TRT-007**：blanket impl、泛型 impl 和带约束 impl 必须具有定义级 descriptor，完整记录泛型参数与
  predicates；系统不得运行时枚举无限适用实例。只有显式登记的 concrete impl instance 才进入具体类型的有效 impl
  视图并参与动态调用。
- **REQ-TRT-008**：每个使用 `#[reflect]` 的 trait 必须生成隐藏 marker type，并以 marker 的 `TypeId` 构成当前
  进程内的 `TraitId`；trait 路径仅用于诊断。该身份机制不得要求 trait 为 dyn-compatible。
- **REQ-TRT-009**：未使用 `#[reflect]` 的 trait impl 必须关联 `ExternalIncomplete` descriptor，至少记录 impl
  源码中使用的 trait 路径、目标类型和当前 impl 中可见的方法；不得虚构默认方法、完整关联项、supertrait 或完整
  trait 声明。此类方法在签名可安全适配时仍可动态调用。
- **REQ-TRT-010**：`TraitDescriptor` 始终描述 trait 声明；dyn-compatible trait 的 `dyn Trait` 还必须具有独立的
  `TypeDescriptor` 并能导航回对应 `TraitDescriptor`。非 dyn-compatible trait 仍具有完整 `TraitDescriptor`，但
  不得伪造不存在的 `dyn Trait` 类型 descriptor。attribute 宏无法观察跨文件或跨 crate 的 supertrait 声明时默认
  不生成 dyn descriptor；作者须用 `dyn_compatible` 显式证明，继承关联类型则以
  `dyn_compatible(Supertrait::AssociatedType, ...)` 声明。宏生成真实 `dyn Trait` 类型，由 rustc 最终验证该证明。
- **REQ-TRT-011**：每个未反射 external trait 必须由使用方提供带命名空间的稳定 `ExternalTraitId`；不同 impl 按
  此 ID 汇聚，源码路径只作为可保留的诊断别名。同一 trait 经不同 `use` 别名书写时允许路径不同；显式 ID 是调用者
  对身份相同的声明。若同一目标类型以同一 ID 注册多个不兼容 impl，或汇聚事实发生不可合并冲突，必须确定性失败。
- **REQ-TRT-012**：关联类型 descriptor 必须记录名称、bounds、默认 `TypeExpression` 和声明顺序；impl descriptor
  必须记录 concrete 绑定或仍未解析的符号表达式。只有解析到 concrete `TypeDescriptor` 的绑定才能参与动态调用。
- **REQ-TRT-013**：关联常量 descriptor 必须记录名称、声明类型、声明顺序和是否有默认值；impl descriptor 必须
  标记实际值来自默认声明还是显式覆盖，并在值可进入动态值边界时提供返回 `ReflectedOwned` 的安全读取适配器。
- **REQ-TRT-014**：`TraitId` 必须区分 reflected marker `TypeId` 与稳定 `ExternalTraitId`，并在当前进程内实现一致的
  `Eq`/`Hash`；两种来源不得因诊断名称相同而相等。external ID 可跨编译稳定，但不自动升级为模型层领域 ID。
- **REQ-TRT-015**：泛型 trait 必须区分 `TraitDefinitionDescriptor` 与应用了 concrete 类型/const 实参的
  `TraitDescriptor`；reflected marker `TypeId` 和 external applied identity 都必须包含 concrete 实参。supertrait、
  关联项和方法签名在 applied view 中完成实参代换，未应用的定义不得伪造可调用运行时身份。

## 8. 安全动态值边界

动态 API 需要在“类型已擦除”和“仍遵循 Rust 静态安全”之间建立边界。概念上至少需要三类值：

```rust,ignore
ReflectedRef<'a>   // 共享借用
ReflectedMut<'a>   // 独占可变借用
ReflectedOwned     // 拥有值
```

它们应通过 `new<T: 'static>(...)` 一类泛型入口构造，并提供受检的 `is::<T>()`、`downcast_ref::<T>()`、
`downcast_mut::<T>()` 或 `downcast::<T>()` 能力，但不得暴露可以伪造类型或借用期的安全 API。参与动态字段操作、
方法调用或构造的 opaque 值不需要另行实现公共动态值 trait。

- **REQ-VAL-001**：动态值必须携带足以进行准确类型检查的身份。
- **REQ-VAL-002**：借用值必须在类型系统中保留原值生命周期，不得通过 `'static` 擦除借用期。
- **REQ-VAL-003**：拥有值的 downcast 失败不得丢失原值；错误必须允许调用方取回该值，或在 API 契约中明确消费时机。
- **REQ-VAL-004**：动态 API 不得使用 JSON、字符串或 `Debug` 文本作为通用中间值。
- **REQ-VAL-005**：Send/Sync 能力必须由实际值类型决定，不得因类型擦除而无条件增加。
- **REQ-VAL-006**：公开安全 API 不得依赖调用者维护未检查的裸指针不变量。
- **REQ-VAL-007**：动态值包装与 `std::any::Any` 的公共关系必须遵循 `REQ-VAL-011` 和 `REQ-VAL-012`，不得直接以
  无生命周期或无线程约束的裸 `dyn Any` 代替包装类型。
- **REQ-VAL-008**：`Reflect` 是用户需要实现或由宏生成的唯一公共反射 trait；动态值包装必须能直接从满足其
  生命周期要求的 Rust 值构造，不得要求调用者为了参与动态操作再实现第二个公共 value trait。
- **REQ-VAL-009**：默认动态值包装是本地包装，不得无条件实现 `Send` 或 `Sync`；必须另行提供受类型约束的
  `ThreadSafe` mode，只允许实际满足相应 `Send`/`Sync` bound 的值进入。两类 mode 之间的转换必须受检，
  不得仅根据 descriptor 名称或运行时标志增加 auto trait。
- **REQ-VAL-010**：本地 owned 包装的构造入口必须接受 `T: 'static`，线程安全 owned 包装的构造入口必须接受
  `T: 'static + Send + Sync`。`downcast::<T>(self)` 成功时返回 `T`，失败时必须返回未丢失原值的原包装；共享与
  可变借用包装 downcast 失败返回 `None`，且不得改变原借用状态。
- **REQ-VAL-011**：本地与线程安全包装必须共享一套以 mode 参数区分的底层类型，并以公共类型别名提供简洁名称；
  `Local` 与 `ThreadSafe` mode 必须在编译期决定内部擦除边界和 auto trait，不得使用运行时 enum 模拟条件
  `Send`/`Sync`。公共调用者通常只需使用别名，不应被迫手写 mode 参数。
- **REQ-VAL-012**：动态值包装必须以 `is/downcast` 作为主要受检 API，同时为 Any-compatible 值提供安全的
  `as_any()`、`as_any_mut()` 和 `into_any()` 互操作入口；对专用 unsized variant，这些入口返回 `None`/原包装，
  并使用其 typed 专用访问器。本地模式的 Any 入口返回对应 `dyn Any` 边界；线程安全模式保留 `Send + Sync` bound。
  任何转换都不得丢失借用期或静默降低线程安全契约。
- **REQ-VAL-013**：本地 `DynamicRef/Mut::new` 要求 `T: Sized + 'static`；线程安全共享借用还要求 `T: Sync`，
  线程安全可变借用还要求 `T: Send + Sync`。API 必须用编译期 bound 表达这些差异，不得依赖运行时 capability flag
  或未检查裸指针接受 unsized 值。
- **REQ-VAL-014**：`DynamicRef/Mut` 必须内建安全的 `str` variant，并提供 `new_str`/`new_str_mut` 与
  `as_str`/`as_str_mut`；它保留原借用期和线程 mode，可用于精确调用 `&str`/`&mut str` 参数及承载借用返回值，
  但不得进入 owned 包装或伪装成 `dyn Any`。
- **REQ-VAL-015**：每个 `ThreadSafe` 动态包装必须提供消费自身的 `into_local` 降级，保持原值、准确类型身份和借用期；
  降级不得失败。框架不得提供仅凭运行时 capability 把 `Local` 包装升级为 `ThreadSafe` 的入口；需要线程安全包装时
  必须从满足编译期 `Send`/`Sync` bound 的 Rust 值重新构造。

## 9. 字段读取与写入

动态字段访问必须像 Java `Field::get/set` 一样由 descriptor 发起，但必须比 Java 更严格地表达借用和所有权。

```rust,ignore
let descriptor = TypeDescriptor::of::<User>();
let field = descriptor.field("username").unwrap();

let value = field.get(ReflectedRef::new(&user))?;
let username: &String = value.downcast_ref().ok_or(/* ... */)?;

field.set(
    ReflectedMut::new(&mut user),
    ReflectedOwned::new(String::from("new-name")),
)?;
```

- **REQ-ACC-001**：`get` 必须校验 target 的实际类型等于 declaring type。
- **REQ-ACC-002**：`get_mut` 和 `set` 必须要求目标的独占可变借用。
- **REQ-ACC-003**：`set` 必须要求输入值与字段声明类型匹配；不得做数值拓宽、字符串解析或 `Into` 推导。
- **REQ-ACC-004**：私有字段可通过宏生成的类型内部适配器访问，但不得向未反射类型提供同等能力。
- **REQ-ACC-005**：字段访问不受源码 `pub` 限制，因为作者已通过 derive 显式授权反射；可见性仍必须保留供策略层判断。
- **REQ-ACC-006**：`#[reflect(read_only)]` 必须保留字段描述和共享读取，但禁用 `get_mut`/`set`；
  `#[reflect(skip)]` 必须保留结构 descriptor 并标记为 skipped，同时禁用该成员的动态读写/调用/构造能力。辅助属性
  不得删除 Rust 结构事实或让字段索引重新编号。
- **REQ-ACC-007**：`#[reflect(rename = "...")]` 只改变 query name，`rust_name()` 必须始终返回源码名称；同一
  查询作用域内 rename 冲突、与未重命名成员冲突或空名称必须在编译期失败。所有错误仍必须包含 Rust 名称。
- **REQ-ACC-008**：方法使用 `#[reflect(no_invoke)]` 保留完整描述但禁用调用；字段/variant 使用
  `#[reflect(no_construct)]` 禁止对应动态构造路径。对不适用目标使用辅助属性必须编译失败；本库不提供改变普通 Rust
  可见性的 `expose` 后门。

## 10. 动态方法调用

```rust,ignore
let rename = TypeDescriptor::of::<User>()
    .methods_named("rename")?
    .exact_inherent()?;

rename.invoke(Invocation::borrowed_mut(
    &mut user,
    [ReflectedOwned::new(String::from("alice"))],
))?;
```

调用分为校验与执行两个阶段。实现必须在执行方法前完成 receiver、参数数量、顺序和值类型检查。

```rust,ignore
pub enum InvocationArg<'a, Mode> {
    Owned(DynamicOwned<Mode>),
    Ref(DynamicRef<'a, Mode>),
    Mut(DynamicMut<'a, Mode>),
}

pub enum BorrowOrigin {
    Receiver,
    Parameter(usize),
}

pub enum InvocationOutput<'call, Mode> {
    Unit,
    Owned(DynamicOwned<Mode>),
    Ref {
        value: DynamicRef<'call, Mode>,
        origins: &'static [BorrowOrigin],
    },
    Mut {
        value: DynamicMut<'call, Mode>,
        origin: BorrowOrigin,
    },
    Future(ReflectedFuture<'call, Mode>),
}
```

- **REQ-INV-001**：无 receiver 的关联函数、`&self`、`&mut self` 和 `self` 方法必须采用不同的调用输入约束。
- **REQ-INV-002**：参数数量或类型不匹配时必须在进入用户方法前失败。
- **REQ-INV-003**：`&mut self` 调用必须要求独占可变借用；`self` 调用必须消费拥有值。
- **REQ-INV-004**：返回值可以是拥有值或绑定到 receiver/参数生命周期的借用；API 必须在类型上保留该关系。
- **REQ-INV-005**：普通 `invoke` 必须原样传播方法内部 panic，不得伪装成参数错误。另行提供的
  `invoke_catching` 只为显式标记 `#[reflect(catch_unwind)]` 的方法生成；宏必须以 trait bound 在编译期验证 receiver、
  参数和适配器满足 `UnwindSafe`/`RefUnwindSafe`。在 `panic = "unwind"` 下 panic payload 包装为独立
  `InvocationPanic`；`panic = "abort"` 下 descriptor 必须明确报告 catching 能力不可用。框架不得依赖 specialization
  自动猜测 unwind safety。
- **REQ-INV-006**：`unsafe fn` 只能描述，不能通过本库动态调用；不得为其生成安全或 unsafe 调用适配器。这一限制
  保持第一方与宏生成代码兼容 `#![forbid(unsafe_code)]`。
- **REQ-INV-007**：`const fn` 可以按普通运行时函数调用；descriptor 记录 const 事实但不承诺动态常量求值。
- **REQ-INV-008**：`async fn` 调用必须返回保留调用借用期和 mode 的 `ReflectedFuture<'call, Mode>`，其完成值为
  对应动态输出；反射层不得内置或选择 executor。本地 mode 接受非 `Send` future，线程安全 mode 只接受实际
  `Send` 的 future，panic 在 poll 时按普通 Future 语义传播。线程安全 future 适配器只由
  `#[reflect(thread_safe)]` 显式请求并经编译期 bound 验证，不得在运行时探测。
- **REQ-INV-009**：泛型方法只有显式登记的有限 concrete specialization 可动态调用；每个 specialization 必须记录
  concrete 类型/const 实参并有独立调用适配器。未登记的泛型声明仍可完整描述，但查询调用能力时返回结构化原因。
- **REQ-INV-010**：除普通 receiver 外，必须支持 `Box<Self>`、`Rc<Self>`、`Arc<Self>`、`Pin<&Self>`、
  `Pin<&mut Self>` 和 `Pin<Box<Self>>` 等能由安全 Rust 构造适配器的 receiver。其他合法 arbitrary self type 可通过
  已登记的安全 `ReceiverAdapter` capability 扩展；调用者必须提供准确 receiver 容器，框架不得把未 pin 的值隐式
  转换为 `Pin`。没有安全适配器时只描述、不可调用。
- **REQ-INV-011**：方法查找必须支持按 impl/trait 限定，不能仅凭裸方法名解决歧义。
- **REQ-INV-012**：所有参与一次调用的借用必须安全重借用到共同的 `'call` 生命周期；借用输出使用该保守生命周期，
  同时必须记录 `BorrowOrigin::Receiver` 或参数索引集合。不得仅给出来源不明的单一生命周期；HRTB 在能以 `'call`
  实例化时生成适配器，否则只描述。
- **REQ-INV-013**：进入用户代码前的任何校验失败都必须返回 `InvocationRecovery`：拥有 receiver 和参数按原顺序
  返还，借用包装保持其原借用关系。进入用户代码后即遵循原签名的消费语义，不再承诺恢复已消费输入。
- **REQ-INV-014**：非 receiver 参数必须由 `InvocationArg<'call, Mode>` 明确区分 owned、shared borrow 和
  mutable borrow。默认要求 passing mode 精确匹配；只允许把 mutable borrow 安全重借用为 shared borrow，不得把
  owned 值隐式借出或复制。位置参数是规范入口；名称绑定只接受全部名称唯一的简单 identifier 参数。
- **REQ-INV-015**：`InvocationOutput` 必须区分 unit、owned、shared borrow、mutable borrow 与 future；never-return
  方法若正常返回属于不可达框架不变量。除语言层 `async fn` 的专用 future 适配外，返回 opaque `impl Trait` 的方法
  默认只能描述、不能动态调用；调用能力必须报告 `OpaqueReturnType` 或等价稳定原因。需要动态调用时，类型作者必须
  改为可命名返回类型，框架不得泄露或依赖编译器隐藏类型名。
- **REQ-INV-016**：方法调用适配器默认只提供 `Local` mode。只有显式标记 `#[reflect(thread_safe)]` 时才生成
  `ThreadSafe` adapter，且宏必须在编译期验证 receiver、全部参数、拥有输出及 future 满足相应 `Send`/`Sync` 与借用
  约束；验证失败必须定位到方法并编译失败。descriptor 中缺少线程安全 adapter 只表示未登记，不得推断该方法在
  Rust 类型系统中必然不具备线程安全性。

## 11. 动态构造

动态构造不是调用某个用户 `new` 方法，而是根据类型或 variant 的字段 shape 创建值。它用于通用反序列化、
fixture 生成、对象映射和框架绑定。

### 11.1 struct 构造

```rust,ignore
let user = TypeDescriptor::of::<User>().construct_struct([
    ("id", ReflectedOwned::new(Id::from(7))),
    ("username", ReflectedOwned::new(String::from("alice"))),
])?;

let point = TypeDescriptor::of::<Point>().construct_tuple([
    ReflectedOwned::new(10_i32),
    ReflectedOwned::new(20_i32),
])?;

let marker = TypeDescriptor::of::<Marker>().construct_unit()?;
```

- **REQ-CON-001**：具名 struct 必须支持按字段名称构造，tuple/newtype struct 必须支持按索引顺序构造，unit struct
  必须支持无参数构造。
- **REQ-CON-002**：默认情况下每个字段必须且只能提供一次；缺失、重复和未知字段必须分别报告错误。
- **REQ-CON-003**：所有字段值必须在开始构造前完成类型校验；失败时不得留下可观察的半初始化对象。
- **REQ-CON-004**：构造私有字段是 derive 明确授权的类型内部操作，不得依赖调用点可见性。
- **REQ-CON-005**：默认构造入口要求每个可构造字段恰好提供一次；`Option<T>` 不得自动缺省为 `None`。只有显式
  `#[reflect(default)]` 或 `#[reflect(default = path)]` 的字段可以省略，前者必须在编译期验证字段类型实现
  `Default`，后者必须验证零参数 provider 返回准确字段类型。

### 11.2 enum variant 构造

```rust,ignore
let failed = TypeDescriptor::of::<Event>()
    .variant("Failed").unwrap()
    .construct_struct([
        ("code", ReflectedOwned::new(500_u32)),
        ("message", ReflectedOwned::new(String::from("timeout"))),
    ])?;

let started = TypeDescriptor::of::<Event>()
    .variant("Started").unwrap()
    .construct_unit()?;
```

- **REQ-CON-006**：unit、tuple 和 struct variant 必须分别提供与其 shape 一致的动态构造能力。
- **REQ-CON-007**：对错误 shape 调用构造入口必须返回 `WrongShape` 类错误。
- **REQ-CON-008**：variant 构造必须返回拥有的 enum 根类型值，而不是未封装 payload。
- **REQ-CON-009**：variant 字段的完整性、重复项和类型检查规则与 struct 构造一致。
- **REQ-CON-010**：构造校验失败不得泄漏、重复析构或遗失已接收的字段值；错误发生在用户可观察的值构造前，
  输入返还规则与 `REQ-INV-013` 一致。
- **REQ-CON-011**：`#[reflect(skip)]`/`#[reflect(no_construct)]` 字段必须同时具有显式 default provider，否则整个
  struct/variant 的从零动态构造能力标记为不可用并给出结构化原因；不得用未初始化内存绕过该字段。
- **REQ-CON-012**：必须提供独立的 struct-update 操作：消费一个类型匹配的 owned 基值和零到多个 override，先完整
  校验所有名称、重复项与类型，再构造更新后的 owned 值；未覆盖字段从基值保留。失败时必须返还基值和全部 override。
- **REQ-CON-013**：类型具有已验证的 `Default` capability 时可以通过 capability 适配器动态默认构造；这与逐字段
  construct 的缺省策略相互独立，不得因为根类型实现 `Default` 就静默补齐普通 construct 的缺失字段。
- **REQ-CON-014**：辅助属性的 default provider、字段来源和 construct/update 能力必须可从 descriptor 查询，供
  schema、绑定和生成器在调用前判断，不得只能通过尝试构造获知。

## 12. 分散 impl 的汇聚与发现

Rust 没有稳定原生机制让一个类型在运行时枚举所有方法，过程宏也不能扫描整个 crate。因此每个
`#[reflect_impl]` 生成独立描述片段，再由反射运行时形成统一视图。

```rust
// file_a.rs
#[reflect_impl]
impl User {
    pub fn username(&self) -> &str { /* ... */ }
}

// file_b.rs
#[reflect_impl]
impl User {
    pub fn disable(&mut self) { /* ... */ }
}

// 两个方法都应出现在 TypeDescriptor::of::<User>() 的方法视图中。
```

- **REQ-AGG-001**：同一类型的所有已标注 impl 片段必须汇聚到该类型的统一查询入口。
- **REQ-AGG-002**：汇聚必须支持 impl 与类型声明位于不同文件或模块。
- **REQ-AGG-003**：系统不得通过扫描源码目录、解析其他文件或使用 rustc 私有接口寻找 impl。
- **REQ-AGG-004**：未标注 impl 不存在于反射世界中；框架不得声称结果是 Rust 编译器所知方法的完全集合。
- **REQ-AGG-005**：重复注册同一反射片段必须产生确定性诊断或初始化错误，不得重复枚举成员。
- **REQ-AGG-006**：全局汇聚完成后的查询视图必须不可变并可安全并发读取。
- **REQ-AGG-007**：每个类型、trait 和 impl 宏必须提交不可变静态注册片段；运行时使用 `OnceLock` 或等价安全原语
  惰性建立统一 `ReflectRegistry`，并提供 `initialize()` 供应用启动时主动暴露全部汇聚错误。不得要求可变热注册。
- **REQ-AGG-008**：动态库装卸和运行时热注册不在目标范围内。
- **REQ-AGG-009**：确定性排序只针对“参与链接且成功注册的反射片段集合相同”的情况；链接进不同片段的两个程序
  可以得到不同成员集合，但各自不得受链接器偶然顺序或首次查询线程影响。
- **REQ-AGG-010**：静态分布式注册必须覆盖同 crate 多模块、依赖 crate 以及最终二进制中合法声明的下游 trait
  impl。目标平台不支持该注册机制时必须编译失败或显式关闭 impl 汇聚相关 API，不得返回貌似成功的不完整结果。
- **REQ-AGG-011**：汇聚实现可以依赖内部使用 unsafe 的 linker-section 库，但 `qubit-reflect` 第一方和宏生成代码
  只能调用其安全封装；依赖必须在仓库 CI 的每个保证平台验证注册完整性、重复检测和并发初始化。
- **REQ-AGG-012**：impl 排序键必须由 inherent/trait 类别、完整 trait 诊断路径或 `ExternalTraitId`、声明 crate、
  模块路径和源码位置组成；inherent impl 先于 trait impl，同一 impl 内方法保持源码顺序。内容相同但链接顺序不同的
  程序必须产生相同迭代顺序。
- **REQ-AGG-013**：`ReflectRegistry::initialize()` 返回 `Result<&'static ReflectRegistry, RegistryError>`；初始化结果
  包括错误必须被缓存并可安全并发查询。纯类型结构查询不依赖成功汇聚，`impls/methods` 等聚合查询必须传播缓存的
  `RegistryError`。
- **REQ-AGG-014**：generic/blanket impl 的定义片段可以全局登记，但不得自动展开无限 concrete 实例；只有宏输入中
  显式登记的 concrete specialization 才附加到目标 `TypeDescriptor`。
- **REQ-AGG-015**：descriptor interning 是内部缓存，不属于运行时热注册；其并发增长不得改变已初始化 registry 的
  成员集合、排序或冲突结果。需要被 `ReflectRegistry::get(TypeId)` 动态发现的 generic concrete 类型必须通过
  specialization/registration 静态提交。

## 13. 泛型、生命周期与复杂 Rust 类型

```rust
#[derive(Reflect)]
struct Page<T> {
    items: Vec<T>,
    total: usize,
}
```

- **REQ-GEN-001**：泛型类型定义与 concrete 单态化类型必须明确区分；`Page<User>` 与 `Page<Order>` 具有不同
  `TypeId`。
- **REQ-GEN-002**：任何作为 owned 动态值、`TypeId` 身份或动态构造结果的 concrete 类型必须为 `'static`；方法参数
  和返回值中的借用通过 wrapper 的调用生命周期表达，不要求被借用目标具有 `'static` 借用期。
- **REQ-GEN-003**：生成的 trait bound 必须只要求形成相应 descriptor/操作实际所需的能力，不得无条件增加无关 bound。
  对出现在 resolved 字段结构中的泛型参数，derive 必须生成形成该字段 `TypeDescriptor` 所需的 `Reflect` bound；只出现
  在 `#[reflect(opaque)]` 字段中的泛型参数不得被附加 `Reflect` bound，但必须满足形成准确动态身份所需的 `'static`。
- **REQ-GEN-004**：泛型定义必须通过独立 `GenericDefinitionDescriptor` 描述 lifetime、类型和 const 参数、bounds、
  defaults 与 where predicates；每个 concrete `TypeDescriptor` 必须导航到定义 descriptor 及按声明顺序排列的
  concrete 实参。
- **REQ-GEN-005**：lifetime 参数在定义级完整描述，但不得伪造运行时 `TypeId` 或声称区分仅 lifetime 不同的
  concrete 类型。包含引用字段的 `'static` 单态化类型可以完整参与动态操作；持有非 `'static` 引用的根值只能通过
  `TypeExpression`/定义 descriptor 描述，不能进入基于 `Any` 的 owned 动态值边界。
- **REQ-GEN-006**：数组、tuple、`REQ-TYPE-007` 确认的常见容器、slice、引用、裸指针、函数指针和 trait object
  必须具有与其 kind 对应的子类型导航。裸指针不得通过安全动态 API 解引用；无法安全动态操作的签名仍必须可描述。
- **REQ-GEN-007**：类型参数 concrete 实参导航到 `TypeDescriptor`；const 实参必须记录其声明类型、规范化诊断表达，
  并在可进入动态值边界时提供 reflected-owned 值。不得把 const 表达式源码文本当作跨编译稳定身份。
- **REQ-GEN-008**：generic method/impl/blanket impl 的定义 descriptor 与 concrete instance descriptor 必须分离；
  concrete instance 记录全部类型和 const 实参以及适用 predicates 的编译期验证结果，不得运行时重新求解 trait bound。
- **REQ-GEN-009**：HRTB 和具名 lifetime 关系必须以结构化 `LifetimeExpression` 表示；调用适配器仅在这些关系能安全
  实例化到统一 `'call` 重借用期时生成。不能安全表达的声明仍可查询完整签名，但调用能力返回明确原因。
- **REQ-GEN-010**：generic concrete 类型和内建组合类型的 descriptor 必须按 `TypeId` 并发安全 intern；同一 concrete
  类型始终返回同一 `&'static TypeDescriptor`。实现可以为进程生命周期有界泄漏已查询 descriptor，但不得错误依赖
  “泛型函数内 static 会按单态化复制”。递归关系必须通过惰性 descriptor handle 解析，初始化不得死锁或暴露半成品。
- **REQ-GEN-011**：opaque 与 resolved 的选择必须由源码属性确定，不得因单态化实参后来实现了 `Reflect` 而改变
  generic 字段的 shape；同一 generic 定义的不同 concrete 实例必须保持相同的字段透明性策略。
- **REQ-GEN-012**：带 lifetime 参数的 derive 类型只为满足 `Self: 'static` 的 concrete 实例实现 `Reflect`；其完整
  lifetime-generic 声明由 `GenericDefinitionDescriptor` 保存，并可从任一合法 `'static` concrete descriptor 导航。
  框架不得为非 `'static` 根值伪造 `TypeId`，也不得为访问定义信息引入与 `Reflect` 同义的第二套公共类型 trait。

## 14. 与 `rs-model-derive` 的集成边界

`qubit-reflect` 负责“Rust 结构是什么、有哪些字段和方法、怎样安全操作”；模型层负责“该结构在领域中意味着什么”。

```text
qubit-reflect                         qubit-model-metadata
──────────────────────────────────    ──────────────────────────────────
TypeDescriptor                        TypeMetadata / role metadata
FieldDescriptor                       FieldMetadata + constraints
MethodDescriptor                      Property getter/setter inference
VariantDescriptor                     EnumVariantMetadata + domain names
safe get/set/invoke/construct          validation / codec / relation policy
```

```rust,ignore
let reflected = TypeDescriptor::of::<User>();
let model = TypeMetadata::of::<User>();

assert_eq!(model.descriptor().type_id(), reflected.type_id());
assert_eq!(model.field("username").unwrap().descriptor().index(), 1);
```

- **REQ-INT-001**：模型层的 `TypeDescriptor`/字段结构事实必须来自 `qubit-reflect`，不得长期维护第二套相互独立实现。
- **REQ-INT-002**：`FieldMetadata` 可以包装或关联 `FieldDescriptor`，并增加 identifier、unique、reference、constraint、
  validator、codec 与 redact 等领域事实。
- **REQ-INT-003**：Property 不属于基础 Rust 反射概念；模型层可以根据 FieldDescriptor 与 MethodDescriptor 推导
  field-backed、computed 或 virtual Property。
- **REQ-INT-004**：`Entity`、`Projection`、`Model`、`Enum`、`Value` 等角色宏应生成等价的 Reflect 能力，避免要求用户
  再重复书写 `#[derive(Reflect)]`；角色过程宏必须直接生成与 `#[derive(Reflect)]` 相同契约的 `Reflect` impl 和
  静态注册片段，不通过要求用户叠加第二个宏实现。
- **REQ-INT-005**：反射层不得依赖或认识 ModelId、identifier、relation、validation、codec、redact 等上层概念。
- **REQ-INT-006**：模型层按名称查询字段或方法时，必须沿用反射层的身份和歧义规则。
- **REQ-INT-007**：依赖方向以 `qubit-reflect` 为准：`qubit-model-metadata` 和 `qubit-model-derive` 必须复用
  `Reflect` 与基础 descriptor，不得要求 `qubit-reflect` 依赖模型层，也不得以 `HasTypeDescriptor` 等同义 trait
  建立第二套类型描述体系。
- **REQ-INT-008**：结构生成器只依赖 `TypeDescriptor` 时，只能承诺 Rust 类型、字段完整性与 shape 正确；要解释
  `text`、`decimal`、`sequence`、validation 等静态约束，必须依赖 `TypeMetadata`；唯一性、reference 和外部
  validator 等上下文约束仍需 repository/validator/context 等上层能力。
- **REQ-INT-009**：`#[text]`、`#[decimal]`、identifier、reference、unique、validation、codec、redact 等属性
  不得下沉为反射层内建语义。角色宏可以一次解析声明并同时生成 Reflect 与模型 metadata，使普通模型用户无需重复
  标注两层宏。
- **REQ-INT-010**：由于依赖方向是模型层指向反射层，`qubit-reflect::TypeDescriptor` 不得直接返回
  `qubit-model-metadata::TypeMetadata`；从 descriptor 反查模型 metadata 的便利能力必须由模型层包装、扩展 trait
  或 registry 提供，避免形成循环依赖。
- **REQ-INT-011**：`qubit-model-derive` 和 `qubit-model-metadata` 必须依赖 `qubit-reflect` 的公共契约实现结构反射；
  模型角色 attribute macro 应在展开结果中委托给由模型 facade 重导出的 `Reflect` derive，不得复制一套字段、variant、
  泛型 bound、opaque 或注册代码生成逻辑。模型 facade 必须隐藏地重导出所需运行时与 derive 路径，使终端用户无需
  为宏展开细节额外声明直接依赖；同一声明重复叠加角色宏与显式 `Reflect` derive 必须给出编译期诊断。
- **REQ-INT-012**：涉及反射结构、动态操作或两层依赖方向时，本文是 `rs-reflect`、`rs-model-derive` 与
  `rs-model-metadata` 后续对齐的权威需求源；模型层文档若与本文冲突，必须修改模型层文档和实现，不得在
  `qubit-reflect` 中加入领域概念或兼容性分叉。

## 15. 错误与诊断

### 15.1 编译期诊断

以下问题必须尽可能在宏展开时报告：宏目标种类错误、语法错误、同一声明中的重复反射名称、无法生成安全适配器的
签名、声明中已知不受支持的 receiver 或泛型形态。

- **REQ-ERR-001**：编译错误必须指向相关声明、字段、variant、impl 或方法，而不是只指向宏名。
- **REQ-ERR-002**：错误消息必须说明违反的规则，并在可行时给出受支持写法。
- **REQ-ERR-003**：宏不得因普通无效输入而 panic；应产生 `compile_error!`/`syn::Error` 等正常诊断。

### 15.2 运行时错误

错误分类至少要能表达：目标类型不匹配、值类型不匹配、错误 shape、字段缺失、字段重复、未知字段、variant 不匹配、
receiver 不匹配、参数数量错误、参数类型错误、名称歧义、成员不可调用和 descriptor 汇聚错误。

- **REQ-ERR-004**：公开动态操作必须返回结构化错误，不得只返回字符串。
- **REQ-ERR-005**：错误必须保留操作类型、descriptor 身份、参数/字段路径、期望类型和实际类型等可获得信息。
- **REQ-ERR-006**：错误类型必须实现 `Debug`、`Display` 和 `std::error::Error`。
- **REQ-ERR-007**：错误显示文本不构成稳定机器协议；消费者必须按公开分类匹配。
- **REQ-ERR-008**：动态输入错误不得 panic；只有文档明确声明的内部不变量破坏才可视为框架 bug。
- **REQ-ERR-009**：字段写入、调用、构造和 update 的预执行错误必须携带结构化 recovery payload，返还仍由框架持有
  的 owned 输入；错误类型必须提供按原名称/索引取回输入的非 panic API。
- **REQ-ERR-010**：`InvocationPanic` 必须与 `InvocationError` 分离，保留方法身份和可安全保留的 panic payload；
  payload 的 `Display` 文本不是稳定协议，且不得把 panic 误报为用户输入错误。
- **REQ-ERR-011**：`RegistryError` 必须区分重复片段、身份内容冲突、external trait ID 冲突、capability 冲突和不支持
  平台；缓存后的错误必须可重复观察，不得因查询顺序变化。
- **REQ-ERR-012**：成员不可调用/构造错误必须包含稳定分类和全部阻塞原因，而不是只返回第一个字符串原因，使工具可以
  在执行前完整解释 unsafe、泛型未实例化、借用无法表达、属性禁止等限制。

## 16. 非目标与明确限制

- **REQ-OUT-001**：本系统不提供无标注 Rust 类型的通用运行时反射。
- **REQ-OUT-002**：本系统不解析整个工作区来发现 impl，也不依赖 rustc 私有 API。
- **REQ-OUT-003**：本系统不提供绕过类型安全、借用规则或未选择加入类型之模块隐私的通用后门；类型作者在声明位置
  使用反射宏后生成的私有成员安全适配器属于显式授权。
- **REQ-OUT-004**：`TypeId` 与 `type_name` 都不是跨版本持久化 schema ID。
- **REQ-OUT-005**：本系统不要求反射宏、编译器插件或动态库在程序运行中热加载。
- **REQ-OUT-006**：本系统不保证动态调用无法通过安全 Rust 表达的任意函数签名；不支持项仍应可描述并报告“不可调用”。
- **REQ-OUT-007**：基础反射层不定义序列化格式、数据库映射、validation、领域角色或访问控制策略。
- **REQ-OUT-008**：基础反射层不定义 BeanRandom/fixture 的随机分布，也不保证构造值满足模型约束或当前业务上下文；
  结构随机、约束感知随机与上下文 fixture 属于不同上层保证等级。
- **REQ-OUT-009**：`no_std`/`alloc` 属于非目标；任何外部移植不得削弱 descriptor 身份、动态错误和安全操作契约。
- **REQ-OUT-010**：核心构造 API 面向 derive struct/enum 及 variant，不根据 container kind 自动承诺 `Option`、集合、
  tuple 或 smart pointer 的通用构造；具体 concrete 组合类型可以通过已验证的扩展 capability 提供
  `ConstructFromElements` 等安全适配器。

## 17. 验收要求

- **REQ-ACCPT-001**：必须有 compile-pass/compile-fail 测试覆盖三种宏的合法目标、非法目标和关键签名限制。
- **REQ-ACCPT-002**：必须测试具名、tuple、newtype、unit struct 以及 unit/tuple/struct enum variant 的描述与构造。
- **REQ-ACCPT-003**：必须测试 public/private 字段的读取、可变读取、写入、错误类型和借用边界。
- **REQ-ACCPT-004**：必须测试无 receiver、`&self`、`&mut self`、`self` 及 trait impl 方法的描述和受支持调用。
- **REQ-ACCPT-005**：必须测试多个文件中的多个 `#[reflect_impl]` 被汇聚，未标注 impl 不出现。
- **REQ-ACCPT-006**：必须测试 inherent 与多个 trait 的同名方法不会覆盖，并能通过限定查询消歧。
- **REQ-ACCPT-007**：必须用编译失败测试证明动态 API 不能延长借用期、制造别名可变借用或错误增加 Send/Sync。
- **REQ-ACCPT-008**：必须测试所有动态输入错误在执行用户代码或修改目标前返回。
- **REQ-ACCPT-009**：必须测试递归类型 descriptor 不会在初始化、遍历或 Debug 时无限递归。
- **REQ-ACCPT-010**：必须提供至少一个模型层集成测试，证明 Model Field/Property 可以复用反射 descriptor。
- **REQ-ACCPT-011**：必须用集成测试证明 `String` 以及已确认的 `Option<T>`、`Vec<T>` 等组合类型可通过
  `Reflect`/`TypeDescriptor::of::<T>()` 查询，并可作为字段、参数或返回类型导航。
- **REQ-ACCPT-012**：必须测试不可调用但可描述的方法能够报告稳定的结构化原因，并覆盖参数 pattern、泛型、unsafe
  或不支持 receiver 中至少三类边界。
- **REQ-ACCPT-013**：必须测试动态调用和构造在校验失败时的输入消费/返还、析构次数及“未进入用户代码”保证。
- **REQ-ACCPT-014**：必须提供模型层边界测试，证明模型角色复用同一 `Reflect` descriptor，同时领域约束不会出现在
  `qubit-reflect` 的内建 descriptor 语义中。
- **REQ-ACCPT-015**：必须覆盖 `HashMap`、`BTreeMap`、`HashSet`、`BTreeSet` 的 kind 与子类型导航，并证明这些
  descriptor 的存在不等同于承诺通用动态构造。
- **REQ-ACCPT-016**：必须测试显式 `#[reflect(opaque)]` 字段可以按准确类型整体读写、传参和参与外层构造，但不能
  导航内部结构或由 opaque member descriptor 直接构造根值。
- **REQ-ACCPT-017**：必须用编译失败测试证明本地动态值包装不会被错误跨线程发送，并测试线程安全包装只接受满足
  对应 `Send`/`Sync` bound 的值。
- **REQ-ACCPT-018**：必须测试 `()` 为 arity 0 tuple，并覆盖引用、slice、裸指针、函数指针与 trait object 的 kind
  和子类型导航；安全动态 API 必须拒绝解引用裸指针。
- **REQ-ACCPT-019**：必须测试 owned downcast 失败返还原包装、借用 downcast 失败不改变借用状态，以及线程安全
  owned 构造入口拒绝非 `Send` 或非 `Sync` 类型。
- **REQ-ACCPT-020**：必须测试 fieldless integer-repr enum 的显式和隐式 discriminant 数值及反向查询，并证明
  data-carrying 或无整数 `repr` 的 enum 不会暴露伪造的数值 API。
- **REQ-ACCPT-021**：必须用 compile-pass/compile-fail 测试验证 core capability 的真实 trait bound、保留命名空间
  和重复 ID 冲突，并测试未知扩展 capability 可被保留、比较和确定性遍历。
- **REQ-ACCPT-022**：必须测试 mode 泛型及其公共别名具有预期的 `Send`/`Sync` 编译期性质，并覆盖 core `Clone`、
  `Default` 与至少一个扩展 capability 的安全动态操作适配器。
- **REQ-ACCPT-023**：必须逐类测试 `TypeKind` 的 typed view：匹配 kind 返回正确 view，错误 kind 返回 `None`，
  且根 descriptor 不暴露只对其他 kind 有意义的平铺导航。
- **REQ-ACCPT-024**：必须同时覆盖 dyn-compatible 与非 dyn-compatible trait 的 marker `TraitId`，并测试
  `dyn Trait` 的 `TypeDescriptor` 只在合法时存在且可导航回 trait 声明。
- **REQ-ACCPT-025**：必须测试未反射外部 trait 的 impl 被标记为 incomplete，只包含可证明事实；其受支持方法仍可
  调用，但不得出现未在 impl 中声明的默认方法或关联项。
- **REQ-ACCPT-026**：必须测试 direct/transitive supertrait 导航、关联类型的声明与 concrete 绑定、关联常量默认值与
  覆盖读取，以及默认方法的 `Defaulted`/`Overridden` 来源和动态调用。
- **REQ-ACCPT-027**：必须覆盖 generic type、method、impl、blanket impl、const generic、lifetime 参数和 HRTB 的
  定义 descriptor，并证明只有显式 concrete specialization 进入有效调用视图。
- **REQ-ACCPT-028**：必须测试 marked impl 默认描述私有方法，以及 `rename`、`skip`、`read_only`、`no_invoke`、
  `no_construct`、`default`、capability 和 specialization 属性的合法目标、冲突与编译期诊断。
- **REQ-ACCPT-029**：必须跨至少两个依赖 crate 测试静态分布式注册、确定性排序、重复/冲突初始化错误、并发首次查询
  和显式 `ReflectRegistry::initialize()`；链接顺序变化不得改变结果。
- **REQ-ACCPT-030**：必须测试完整字段构造、显式 `Default`/provider 补缺、`Option` 不自动缺省、不可构造字段诊断、
  struct update 及所有失败路径的基值和输入返还。
- **REQ-ACCPT-031**：必须分别测试普通 panic 传播、显式 `catch_unwind` 在 unwind 模式下的 catching、非
  unwind-safe 标记的编译失败，以及 `panic = "abort"` 下 catching 能力不可用的配置行为。
- **REQ-ACCPT-032**：必须测试借用 receiver/参数的本地 async 方法、显式 `thread_safe` 的 `Send` future 线程安全
  调用、非 `Send` future 标记的编译失败，并证明反射层没有隐式执行 future。
- **REQ-ACCPT-033**：必须覆盖 `Box`、`Rc`、`Arc`、`Pin<&Self>`、`Pin<&mut Self>`、`Pin<Box<Self>>` receiver，
  至少一个扩展 `ReceiverAdapter`，以及没有安全适配器时只描述不可调用。
- **REQ-ACCPT-034**：必须用 compile-pass/compile-fail 测试覆盖共同 `'call` 重借用期、receiver/参数借用来源、
  mutable-to-shared 重借用、禁止 owned 隐式借出、非 `'static` 根值限制和可安全实例化的 HRTB。
- **REQ-ACCPT-035**：必须覆盖 identifier、wildcard、destructure 参数 pattern，unit、never、reference、opaque
  `impl Trait` 返回值和位置/名称绑定边界，并验证非 async opaque return 只描述且报告稳定的不可调用原因。
- **REQ-ACCPT-036**：必须测试同一 external trait 通过不同源码别名但相同 `ExternalTraitId` 汇聚，并测试同一目标的
  重复 impl 身份或不可合并事实产生确定性错误。
- **REQ-ACCPT-037**：必须测试 `qubit-reflect-derive` 的默认 re-export、直接依赖 derive crate 的高级用法，以及
  Cargo 依赖重命名后的生成代码路径解析。
- **REQ-ACCPT-038**：必须测试按 `TypeId`、重名 `type_name` 候选和全量枚举查询，并证明任意非 reflect attribute
  不会泄漏为底层 descriptor 或引入 `syn` 公共依赖。
- **REQ-ACCPT-039**：必须测试 `str`、slice 和 `dyn Trait` 等 unsized descriptor、内建 `str` 借用包装与动态调用，
  并用编译失败测试证明 slice/dyn Trait 在没有专用适配器时不能直接进入动态值包装或构造；线程安全借用构造必须
  严格执行对应 `Sync`/`Send + Sync` bound。
- **REQ-ACCPT-040**：必须逐项覆盖 primitive、`String`/`str` text、container/smart-pointer family、引用/裸指针
  mutability 和函数指针 ABI/safety/variadic 导航，并测试 `TypeExpression`/`LifetimeExpression` 不依赖 `syn` 公共类型。
- **REQ-ACCPT-041**：必须并发查询多个 generic concrete/组合类型，证明同一 `TypeId` descriptor 指针唯一、不同
  concrete 实参互不混淆，并覆盖直接递归、间接递归初始化无死锁且不产生半初始化视图。
- **REQ-ACCPT-042**：必须覆盖 reflected/external 泛型 trait 的定义与多个 concrete applied descriptor，验证
  `TraitId`、supertrait 和关联项实参代换正确且不同应用不会错误汇聚。
- **REQ-ACCPT-043**：必须证明初始化 registry 后按需查询新的 generic/composite descriptor 不会改变 registry
  枚举和名称查询结果；显式 concrete registration 则必须可按 `TypeId` 发现，并且 interner 并发增长不影响排序。
- **REQ-ACCPT-044**：必须用 compile-pass/compile-fail 测试证明普通字段缺少 `Reflect` 时 derive 失败、添加
  `#[reflect(opaque)]` 后成功；还必须覆盖 resolved/opaque 泛型字段的最小 bound、显式 opaque 不自动升级，以及
  `TypeRef` 导航不产生第二个根 descriptor。类型级 opaque 还必须拥有唯一根 descriptor 且不暴露任何内部成员。
- **REQ-ACCPT-045**：必须测试 `catch_unwind` 和 `thread_safe` 属性的合法目标、编译期 bound 验证与未标记时的能力
  缺失语义，并覆盖外部 supertrait/bound 的 `external_trait(path, id = "...")` 映射、缺失映射和冲突诊断。
- **REQ-ACCPT-046**：必须测试带 lifetime 参数的 derive 类型只在 `'static` concrete 实例上取得根 descriptor，且可
  从该实例导航到包含完整 lifetime 参数和字段符号类型的定义 descriptor；非 `'static` 根值进入动态边界必须编译失败。
- **REQ-ACCPT-047**：必须提供跨 crate 集成测试，证明模型角色宏通过模型 facade 委托同一 `Reflect` derive、终端用户
  不需直接添加 `qubit-reflect` 依赖、模型 descriptor 与反射根 descriptor 身份一致，并验证重复显式 derive 的诊断。
- **REQ-ACCPT-048**：必须测试三种线程安全包装到本地包装的无损 `into_local` 降级，并用编译失败测试证明本地包装
  不能凭 descriptor capability 或运行时检查升级为线程安全包装；降级后的值必须可用于字段访问和动态构造。

## 18. 审核结论

本规范的审核占位符已经全部关闭。实现计划不得重新把已确认能力降级为“暂不实现”；若稳定 Rust、安全 Rust 或目标
平台无法满足某项动态操作，必须保留完整描述并按本文规定返回结构化的不可用原因。任何新的架构级歧义都必须先修订
本规范并新增带编码需求，而不是在实现中隐式选择。

## 19. 需求索引

| 需求组 | 内容 | 数量 |
| --- | --- | ---: |
| `REQ-SYS-*` | 系统目标与边界 | 13 |
| `REQ-MAC-*` | 三种反射宏 | 21 |
| `REQ-DESC-*` / `REQ-TYPE-*` | descriptor 关系与类型 API | 40 |
| `REQ-FLD-*` / `REQ-VAR-*` | 字段与 enum variant | 20 |
| `REQ-MTH-*` / `REQ-TRT-*` | 方法、参数、trait 与 impl | 28 |
| `REQ-VAL-*` / `REQ-ACC-*` | 动态值和字段访问 | 23 |
| `REQ-INV-*` | 动态方法调用 | 16 |
| `REQ-CON-*` | 动态构造 | 14 |
| `REQ-AGG-*` / `REQ-GEN-*` | 汇聚、泛型与复杂类型 | 27 |
| `REQ-INT-*` | 模型层集成 | 12 |
| `REQ-ERR-*` / `REQ-OUT-*` | 错误与非目标 | 22 |
| `REQ-ACCPT-*` | 验收标准 | 48 |

当前最终需求规范共定义 284 条带编码需求，不再保留待确认占位符。
