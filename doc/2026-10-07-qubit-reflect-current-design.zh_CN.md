# 当前设计说明（2026-10-07）

本文记录反射引用设计复核后的 `qubit-reflect` 0.2.0 当前设计。原有的
[2026-09-03 设计文档](2026-09-03-qubit-reflect-design.zh_CN.md) 描述的是 0.1
设计且保持不变；本文作为当前设计增补。

## 类型引用与身份

<!-- reflect-contract: typeref.constructor=owned-resolved -->
`TypeRef::of<T: Reflect + ?Sized>()` 通过 `TypeDescriptor::of::<T>()` 创建拥有值
`TypeRef::Resolved`。构造这个枚举值本身不分配内存。初始化 `T` 的根描述符可能
分配内存；描述符还会检查手写 `Reflect` 实现报告的 Rust `TypeId` 是否与 `T` 一致。
身份不一致时会 panic，与直接调用 `TypeDescriptor::of` 的行为相同。该方法不承诺
`TypeRef` 值在进程内全局驻留；身份由根描述符承载。

<!-- reflect-contract: lazy.retry=panic-retry -->
`LazyTypeRef` 首次解析也使用此构造器。每个 slot 的 `OnceLock` 会向并发读取者发布
一次成功解析的结果。如果 resolver panic，slot 仍未初始化，后续调用会重试。

## 模型元数据边界

模型元数据需要 `&'static TypeRef`，而反射 API 返回拥有值。模型 v7 适配器在这个
静态元数据边界创建并泄漏一个 `TypeRef`。每次调用 helper 都可能分配；生成的元数据
provider 会缓存其结果，因此这是初始化开销，而不是每次读取属性的开销。该 helper
不承诺全局驻留。

## 字段名查找

<!-- reflect-contract: field.lookup=linear -->
`TypeDescriptor::field` 当前按名称线性扫描描述符的字段。下面的 Criterion 微基准使用
一个包含 16 个 `u8` 字段的派生类型；取得描述符后只测量 `field(name)`。Criterion
应用字段基准过滤条件前，自定义 `main` 仍会执行现有的 20 个进程冷启动测量。

命令：`cargo bench -p qubit-reflect --bench descriptor_lookup -- 'field_lookup/'`

环境：rustc 1.94.0、x86_64 Intel Core i5-9600K 3.70 GHz、6 核；Cargo 优化 bench
profile。Criterion 每项采集 100 个样本，预热 3 秒，测量约 5 秒。

| 查找位置 | 估算值 | Criterion 区间 | 离群样本 |
| --- | ---: | ---: | ---: |
| 首字段（`f00`） | 5.2645 ns | 5.0341–5.5676 ns | 17/100（17%） |
| 尾字段（`f15`） | 42.574 ns | 40.660–44.756 ns | 16/100（16%） |
| 缺失字段（`absent`） | 12.718 ns | 12.490–12.938 ns | 28/100（28%） |

这些数据只描述单机上合成的 16 字段查找，不能代表端到端反射或模型元数据吞吐量。
离群样本比例也较高，尤其是缺失字段，因此不应把细微差异解读为稳定的产品性能差异。
当前保留简单线性扫描：这次测量没有证明代表性应用能从索引中获益，尚不足以抵消
索引的存储和构建成本。若要重新评估，应使用真实下游模型结构及查询频率，并同时
比较延迟与描述符内存占用。

## 延伸阅读

- [历史 0.1 设计](2026-09-03-qubit-reflect-design.zh_CN.md)
- [用户手册](user_guide.zh_CN.md)
- [derive 契约矩阵](derive-contract-matrix.zh_CN.md)
