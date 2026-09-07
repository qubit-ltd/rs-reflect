# `qubit-reflect` 演进历史

- 日期：2026-09-07
- 相关设计：[中文设计](2026-09-03-qubit-reflect-design.zh_CN.md) · [English design](2026-09-03-qubit-reflect-design.md)

本页保留按日期记录的实现与评审说明。当前契约统一维护在详细设计中；
这里说明九月变更如何形成该契约。

## 2026-09-05 评审整改

- 类型相关 capability 按具体 `TypeId` 缓存；factory 在缓存表锁外执行，
  回归覆盖多个 struct/enum 单态化和自定义模型 provider，包括并发调用。
- `ImplDefinitionDescriptor::implemented_trait()` 只暴露声明已知的链接。
  符号链接通过 `implemented_trait_in(&registry)` 或
  `ReflectRegistry::impl_definition_trait` 解析；链接属于 snapshot 索引，
  构建失败和其他 snapshot 不会修改声明。
- facade 宏使用 `#[reflect(definition_provider_v2 = identifier)]` 选择由
  facade 所有的 provider 名称。v2 provider 是无参数函数，返回
  `&'static TypeDefinitionDescriptor`，不要求具体单态化，也不推断 reflect
  的默认名称。
- `scripts/check-downstream.sh` 校验真实 `rs-model-metadata` workspace
  （含 `derive/`）和 `rs-platform`。本地 CI 与独立 GitHub Actions job 都执行
  该门禁；缺少相邻仓库会显式失败。baseline 使用清单记录的精确 SHA，head
  使用各依赖仓库的 `main`，两条通道都记录 feature 选择；私有仓库可使用
  `DEPENDENCY_TOKEN`。
- descriptor 首次初始化在排除进程启动时间的全新子进程中测量；热查询与
  1/4/8 线程查询分开测量。平台 benchmark 覆盖实际链接模型、投影、关系校验、
  allocation 请求和 requested bytes。

## 2026-09-07：显式调用快照与 `codegen_v3`

所有普通、catching、thread-safe 和 pinned 入口现在都显式接受
`&ReflectRegistry`。调用方使用 `methods_named_in(registry, ...)` 查找，
再把同一个 registry 传给 `invoke_*(registry, invocation)`。函数指针采用
独立的 `for<'registry, 'call>` 生命周期；输出、future 与 recovery 不保留
registry 借用，但仍受输入生命周期、Local/ThreadSafe 和 Pin 约束。调用不会
初始化全局 registry，也不会回退到全局 capability。

可安全生成的特殊 receiver 总有静态 adapter；入口可用性不再由 inventory
探测决定。所选 snapshot 缺少、类型不匹配或只声明 fact 的 receiver capability
时，返回 `ReceiverAdapterUnavailable` 并按调用者顺序保留全部输入。adapter
拒绝与 intrinsic 冲突仍是不同的结构化原因；静态不支持的签名仍无入口。

`TypeDescriptor` 的 Debug 只输出结构事实，不查询 capability 或执行 provider。
冻结的成员、名称和方法索引不执行 provider；未注册具体类型的 capability 查询
仍可能惰性初始化 intrinsic facts。provider 只能依赖静态类型事实，禁止重入
capability 或 registry 初始化。下游自定义 capability/provider 可以承载模型元数据；
reflect 不定义或解释领域语义。

本次破坏性变更移除 `codegen_v2`，facade 必须精确暴露 `codegen_v3`。
模型 `__private::v4` 与 `definition_provider_v2` 保持各自独立的契约；包版本仍为
0.1.0、未发布。

本阶段建立的验收与覆盖率门禁已写入设计的验证矩阵，并继续作为当前契约的一部分。
