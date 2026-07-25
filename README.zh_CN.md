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
取得最早对象或有界区间、重复有序键的稳定挂载顺序、显式的挂载/分离状态，以及
退出有序索引后仍保留在主 Map 中的对象。主键只存储一次，并且不要求实现
`Clone`。

本 crate 使用 `hashbrown` 的底层哈希表抽象维护私有主键索引。它不使用
`slotmap`：由于槽位标识永远不会离开 Map，私有的 `Vec<Option<_>>` arena
已经足够。本 crate 不提供内部同步；当所有组成类型都满足条件时，Map 会自动
实现 `Send` 和 `Sync`。需要共享可变访问时，应由所有者使用符合其并发模型的
同步原语包装。

## 复杂度

| 操作 | 复杂度 |
|---|---|
| 按主键查询或修改值 | 平均 `O(1)` |
| 查看最小的已挂载记录 | `O(log n)` |
| 插入、替换、删除、挂载、分离或重新排序记录 | `O(log n)` |
| 分离或提取包含 `k` 个对象的区间 | `O(k log n)` |
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

let first = deadlines.first().expect("存在已挂载的截止时间");
assert_eq!(&"earlier", first.key());

// 分离只影响有序视图，并直接提供值访问。
let detached = deadlines.detach("earlier").expect("记录已挂载");
assert_eq!(&"first", detached.value());
drop(detached);
assert_eq!(Some(&"first"), deadlines.get("earlier"));
assert_eq!(
    vec![&"second"],
    deadlines.values_ordered().collect::<Vec<_>>(),
);
```

## 索引生命周期

每次插入都会挂载到有序索引。`pop_first`、`remove` 和 `extract_range`
删除完整记录。`detach` 和 `detach_range` 保留主记录，并且无需再次进行
哈希查询即可访问值；`attach` 恢复有序可见性。`set_order` 在保留挂载状态的
同时修改有序键。`insert` 会替换已占用的主键；`try_insert` 则拒绝重复项，
并原样返回未插入的主键、有序键和值。

`get_entry`、`iter`、`iter_ordered`、`range` 和 `first` 会公开记录的
主键、有序键、值与 `IndexState`。下游只需要值时可直接使用
`values_ordered`。主键和有序键不能直接修改，因为不重建相应索引的修改会破坏
集合不变量；通过内部可变性间接修改也受同一限制。

当范围起点大于终点，或相等的两个端点都为排除边界时，范围操作会 panic。
提前丢弃 `detach_range` 或 `extract_range` 只影响已经产出的记录。挂载顺序使用
`u64` 序列；序列耗尽时会 panic，而 `clear` 会重置序列分配器。

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
