# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

`qubit-reflect` 是一个需求先行的 Rust 宏反射 crate，目标是为上层框架统一提供类型、字段、方法、trait、impl、枚举变体、安全访问、动态调用和动态构造信息。

仓库目前只是初始骨架，尚未实现或稳定任何公共反射 API。

## 面向读者

本项目面向需要在稳定 Rust 上检查并访问类型结构的框架和基础库作者，避免依赖源码扫描或 rustc 私有 API。

## 安装

crate 尚未发布，也不应作为可用依赖接入。首版公共 API 实现后再补充安装方法。

## 当前状态

仓库目前只包含 CI 基础设施、crate 元数据和中文版初步需求规范，尚未提供规划中的 derive 或属性宏。

## 规划范围

目标设计包括结构 descriptor、安全字段访问、方法调用、trait/impl metadata，以及 struct 和 enum variant 的动态构造。准确 API 仍需经过需求审核。

## 延伸阅读

- [中文版需求规范](doc/2026-08-28-qubit-reflect-requirements.zh_CN.md)
- [English README](README.md)

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
