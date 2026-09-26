# rs-reflect 设计与实现复评

- 日期：2026-09-26
- 范围：`rs-reflect`、`rs-model-metadata`、`rs-platform` 当前工作区
- 工具链：Rust 1.94.0
- 结论：保持反射运行时、注册表快照和 `codegen_v3` 设计；新增真实下游重复测量工具与本次复评记录。当前证据未触发运行时或生成协议破坏性变更。

## 验证范围

基线记录在三个仓库都干净、且各自位于 `dev-starfish` 分支时采集。下游门禁包含以下七条命令，全部退出码为 0：

1. `rs-model-metadata` workspace 的 `--all-features --lib --tests`。
2. `qubit-model-metadata` 的无默认 feature、`generic`、`codec` 和 `validation` 四种检查。
3. `rs-platform` workspace 检查。
4. `qubit-platform-testkit` 测试及 doctest。

实际下游共 133 个模型。`rs-platform` benchmark 的单次基线为：冷链接与投影 1.956 ms、13,040 次分配请求和 2,637,324 请求字节；热模型投影 433.733 μs、1,916 次和 225,116 字节；关系校验 422.615 μs、3,761 次和 399,344 字节。另有代表类型元数据查询 4.316 μs、34 次和 1,984 字节。

在 10 个全新进程中重复同一 benchmark，冷链接耗时中位数为 4.106 ms（2.126–6.276 ms），热模型投影为 852.950 μs（478.900–1,313.619 μs），关系校验为 793.637 μs（486.015–1,497.008 μs）。代表类型元数据查询中位数为 7.914 μs（4.430–9.753 μs）。四项的每次分配数和请求字节数在 10 次样本中均保持一致。

两组耗时跨度明显，采样期间机器负载不受控，不能把两组中位数差解释为代码回归或改进。可确定的证据是分配计数稳定，真实 workload 的模型数为 133；当前没有定义的产品延迟预算，也没有缓存导致预算违约的证据。10 次完整原始输出、仓库提交号与工具链信息保存在临时目录的 `baseline/measurements/measurements.json`。

## 内存与缓存审计

descriptor interner 按具体 `TypeId` 保存唯一 `OnceLock<TypeDescriptor>`；生成 capability cache 也按具体 `TypeId` 保存一次初始化 cell。应用 trait payload 使用 `(TypeId, AppliedTraitId)` 键。工厂在 interner/map 锁外运行，已存在的并发初始化和 descriptor 唯一性测试覆盖这些契约。

这些对象因返回 `'static` descriptor 而保留至进程结束，符合 `REQ-GEN-010` 对按需具体类型缓存的既定要求。单个二进制可实例化的具体 Rust 类型和生成 trait 应用集合有限；本次测得的真实平台注册链在重复样本中的分配计数与请求字节数稳定。当前未观察到重复 cell、错误的缓存键或快照枚举被按需缓存修改。benchmark 的累计请求字节不是常驻堆大小，因此本次不把它当成 RSS 结论；没有证据要求增加可回收缓存或改变生命周期。

## 生成协议与下游边界

`rs-model-metadata` 的私有生成 ABI 通过精确的 `codegen_v3` facade 使用反射 runtime；模型 crate 自己维护独立的 `v7` ABI。反射 workspace 的跨 crate facade fixtures 与本次完整的模型、平台测试覆盖了结构 derive、trait/impl 生成、generic 元数据、能力注册和平台真实模型装配。真实下游命令均通过，没有发现协议升级后下游仍静默通过的缺口。

本次没有改动 `codegen_v3`、模型 `v7` 或下游 facade。维护时继续以 `scripts/check-downstream.sh` 作为真实依赖链门禁，并在涉及 generated ABI 的变更中连同反射 workspace fixtures 一起运行。

## 产物与后续门槛

- `scripts/measure-reflect-downstream.py` 在临时目录复制当前 rs-reflect worktree、模型 metadata 和平台源码，使用独立 Cargo target 运行 benchmark，避免从原平台 checkout 误测另一个 reflect 版本。
- `scripts/tests/measure_reflect_downstream_tests.py` 覆盖 benchmark 输出解析、重复样本统计、模型数漂移、仓库名稳定性、临时输出路径安全和临时源码布局。
- 当前没有运行时改动。只有后续实测证明确有可复现的重复缓存、并发错误、快照语义变化，或真实 workload 超出经确认的延迟/内存预算时，才重新评估对应缓存实现。
