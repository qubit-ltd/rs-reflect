# rs-reflect 下游复评结果

## 实施范围

- 显式快照新增 `capability_only_type_targets`，按 capability ID 返回未加入类型成员集合的目标和来源，并按来源身份排序。
- `definition_capabilities` 现在区分未知定义（`None`）与已注册但无能力（`Some(empty)`）；便捷查询保持原来的缺失语义。
- `FragmentIdentity` 的字符串字段改为共享的不可变 `Arc<str>`，避免注册构建期间克隆来源身份时重复分配字符串。
- 平台 benchmark 输出协议升为 v2，拆分冷反射初始化与冷模型投影。测量脚本严格校验协议版本、全部五项指标和样本数量。
- 更新中英文 README 与用户指南，补充可运行的快照成员/能力专用示例及泛型定义三态说明。

## 环境与验证

- Rust 工具链：`rustc 1.94.0 (4a4ef493e 2026-03-02)`，host `x86_64-unknown-linux-gnu`。
- 基线仓库提交：`rs-reflect fe922970be053a5146465de92a3c862393c8fe8a`、`rs-model-metadata a2b22ae280ea860466d26045a7ac2b4872c82eda`、`rs-platform 5cea766c5db57bcfc8a66b491e48146b40d7ccf5`。
- `cargo +1.94.0 test --locked --workspace --all-features --lib --tests`（`rs-reflect`）：退出码 0。
- `cargo +1.94.0 test --locked --no-default-features --lib --tests`（`rs-reflect`）：退出码 0。
- `./scripts/check-downstream.sh`：退出码 0；覆盖 `rs-model-metadata` 全功能测试、四组无默认功能检查、`rs-platform` 工作区检查及 testkit 测试。
- `python3 -m unittest discover -s scripts/tests -p 'measure_reflect_downstream_tests.py'`：退出码 0，8 项通过。
- `./scripts/check-markdown-examples.sh`：退出码 0，22 个隔离 Rust 示例通过。
- `rs-reflect/align-ci.sh`、`rs-reflect/ci-check.sh`：退出码均为 0；CI 检查包含功能矩阵、Markdown 示例、打包验证与覆盖率门槛。
- `rs-platform/align-ci.sh`、`rs-platform/ci-check.sh`：退出码均为 0；工作区测试、文档和依赖检查完成。
- 平台 benchmark 编译及 v2 十样本测量：退出码均为 0。

## 下游测量

每个数字是 10 个新进程的中位数；方括号内为最小值至最大值。模型数在全部样本中均为 133，五项指标均存在，样本成功率为 10/10。

| v2 指标 | 耗时 ns/op | 分配次数/op | 请求字节/op |
| --- | ---: | ---: | ---: |
| `cold_reflection_init` | 864,688 [827,654–1,307,130] | 4,452 [4,452–4,452] | 1,114,815 [1,114,815–1,114,815] |
| `cold_model_projection` | 1,051,920 [1,023,020–1,123,130] | 6,932 [6,932–6,932] | 1,481,427 [1,481,427–1,481,427] |
| `type_metadata_try_of_representatives` | 4,291 [4,182–6,394] | 34 [34–34] | 1,984 [1,984–1,984] |
| `warm_model_projection` | 431,430 [428,454–451,939] | 1,916 [1,916–1,916] | 225,116 [225,116–225,116] |
| `relationship_validation` | 422,776 [414,655–435,270] | 3,761 [3,761–3,761] | 399,344 [399,344–399,344] |

冷反射初始化阶段占冷反射初始化与首次模型投影两段合计分配次数约 39.1%，请求字节约 43.0%，达到计划中的 profile 门槛。`perf` 因容器的 `perf_event_paranoid=4` 无法采样；在临时源码副本用 DHAT 单独运行一次反射初始化后，分配栈指向 `RegistryBuilder::push_type`、`push_capability_with_origin` 以及冻结来源索引时对 `FragmentIdentity` 的深复制。把私有字符串字段改为共享 `Arc<str>` 后，DHAT 的初始化分配从 6,109 次降到 4,453 次（约减少 27.1%），请求字节从 1,156,921 降到 1,115,839（约减少 3.6%）。冷初始化指标在同一 v2 基准位置复测，中位耗时从 1,103,525 ns 降到 825,086 ns，分配次数从 6,108 降到 4,452。首次模型投影现在在 `link_all_models()` 前测量，计入首次投影所需的缓存和索引；更早一次 v2 运行曾提前初始化全局模型注册表，其冷投影数据未用于本报告。未发现下游 warm 指标的分配变化。

旧协议只测量 `ModelRegistry::global()` 的单一 `cold_link_and_projection` 项，中位数为 1,989,520 ns、13,040 次分配和 2,637,324 请求字节；新协议分别测量反射初始化和模型投影，测试入口也改为显式传入已初始化的反射注册表，因此旧单项与新两项不能直接比较。新协议测量器修复了临时源码副本中 validator 路径重复的问题；基线与复测的模型数均为 133。分配器统计的是累计请求次数和请求字节，不代表进程结束时的存活内存。

## 证据

- 基线十次采样：[baseline-layoutfix2/measurements.json](/tmp/superpowers-4_mn2csx/baseline-layoutfix2/measurements.json)
- 协议 v2 优化前十次采样：[after-v2/measurements.json](/tmp/superpowers-4_mn2csx/after-v2/measurements.json)
- 协议 v2 最终十次采样：[after-v2-cold-projection/measurements.json](/tmp/superpowers-4_mn2csx/after-v2-cold-projection/measurements.json)
- 共享身份优化前 v2 十次采样：[after-v2/measurements.json](/tmp/superpowers-4_mn2csx/after-v2/measurements.json)
- DHAT 临时 profile：[reflection-heap.json](/tmp/superpowers-4_mn2csx/reflection-heap.json)
- 实施计划：[2026-09-27-rs-reflect-reassessment-plan.md](/tmp/superpowers-4_mn2csx/2026-09-27-rs-reflect-reassessment-plan.md)
