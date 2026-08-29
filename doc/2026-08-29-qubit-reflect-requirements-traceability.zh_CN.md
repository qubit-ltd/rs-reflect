# `qubit-reflect` 需求追踪矩阵

本矩阵的规范 ID 集合以
[最终需求规范](2026-08-28-qubit-reflect-requirements.zh_CN.md) 为唯一来源。实施计划的
“逐项需求映射”章节（从 `REQ-SYS-001` 开始）保留每个 ID 独立的一行映射；该章节是本矩阵的
机器可核验明细，避免在两份文档中复制 284 行而产生漂移。

## 验证方法

```bash
rg -o 'REQ-[A-Z]+-[0-9]+' doc/2026-08-28-qubit-reflect-requirements.zh_CN.md | sort -u > /tmp/reflect-requirements
rg -o 'REQ-[A-Z]+-[0-9]+' doc/2026-08-29-qubit-reflect-implementation-plan.zh_CN.md | sort -u > /tmp/reflect-plan
diff -u /tmp/reflect-requirements /tmp/reflect-plan
```

## 实现与测试入口

| 需求范围 | 实现入口 | 验证入口 |
| --- | --- | --- |
| `REQ-DESC-*`、`REQ-TYPE-*`、`REQ-GEN-*` | `src/descriptor/`、`src/builtin/`、`derive/src/expand/` | `tests/descriptor/` |
| `REQ-VAL-*`、`REQ-ACC-*` | `src/value/`、`src/access/` | `tests/value/`、`tests/access/` |
| `REQ-INV-*`、`REQ-MTH-*` | `src/invoke/`、`derive/src/expand/impls.rs` | `tests/invoke/`、`tests/descriptor/reflect_impl_tests.rs` |
| `REQ-CON-*`、`REQ-VAR-*` | `src/construct/`、`derive/src/expand/{structs,enums}.rs` | `tests/construct/`、`tests/descriptor/derive_*_tests.rs` |
| `REQ-AGG-*`、`REQ-TRT-*` | `src/registry/`、`src/descriptor/` | `tests/registry/`、`test-crates/registry-*` |
| `REQ-MAC-*`、`REQ-ERR-*` | `derive/src/{parse,ir,validate,expand}/` | `derive/tests/`、`tests/ui/` |
| `REQ-INT-*`、`REQ-OUT-*`、`REQ-ACCPT-*` | `src/lib.rs`、`README*`、`doc/` | workspace、doctest、CI 对等检查 |

每个具体 requirement 的任务映射位于
[实施计划的逐项需求映射](2026-08-29-qubit-reflect-implementation-plan.zh_CN.md#逐项需求映射)，并以该
需求范围中的实现和验证入口为最终代码证据。
