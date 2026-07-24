# Qubit Collections

[![Rust CI](https://github.com/qubit-ltd/rs-collections/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-collections/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-collections/coverage-badge.json)](https://qubit-ltd.github.io/rs-collections/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-collections.svg?color=blue)](https://crates.io/crates/qubit-collections)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

为标准库没有直接表达的查询模式提供聚焦的 Rust 集合类型。

## 概述

Qubit Collections 的第一个类型是 `OrderedIndexMap<K, O, V>`：它在主键
Map 之外维护一个可独立管理的有序次级索引。它支持按 `K` 快速查找、按 `O`
取得最早对象或有界前缀、重复有序键的稳定插入顺序，以及退出有序索引后仍保留
在主 Map 中的对象。

本 crate 没有运行时依赖，也不提供内部同步。跨线程共享时，应由所有者使用符合
其并发模型的同步原语包装。

## 复杂度

| 操作 | 复杂度 |
|---|---|
| 按主键查询或修改值 | 平均 `O(1)` |
| 查看最小的已索引有序键 | `O(log n)` |
| 插入、替换、删除或解除一个索引项 | `O(log n)` |
| 解除包含 `k` 个对象的有界前缀 | `O(k log n)` |
| 空间 | `O(n)` |

## 安装

```toml
[dependencies]
qubit-collections = "0.1"
```

## 快速开始

```rust
use qubit_collections::OrderedIndexMap;

let mut deadlines = OrderedIndexMap::new();
deadlines.insert("later", 20, "second");
deadlines.insert("earlier", 10, "first");

assert_eq!(Some((&"earlier", &10, &"first")), deadlines.first());
assert_eq!(vec!["earlier"], deadlines.unindex_through(&10));

// 解除索引只影响有序视图。
assert_eq!(Some(&"first"), deadlines.get("earlier"));
assert_eq!(Some((&"later", &20, &"second")), deadlines.first());
```

## 索引生命周期

每次插入都会进入有序索引。`pop_first` 和 `remove` 删除完整记录；
`unindex`、`unindex_first` 和 `unindex_through` 只删除有序索引项。主记录
会保留原始有序键，并继续通过 `get`、`get_mut`、`order_key` 和 `remove`
访问。

`get_mut` 不允许修改有序键；在不重建次级索引的情况下修改它会破坏 Map
不变量。

如果用户定义的键 trait 在跨索引修改过程中 panic，Map 会进入中毒状态并拒绝
后续操作。此时应丢弃该实例，避免读取只完成了一半的索引更新。

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

仓库地址：[https://github.com/qubit-ltd/rs-collections](https://github.com/qubit-ltd/rs-collections)
