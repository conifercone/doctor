# Implementation Plan: 接口-实现类匹配

**Branch**: `004-interface-impl-matching` | **Date**: 2026-07-04 | **Spec**: [spec.md](./spec.md)

## Summary

修改 bean_graph.rs 的源码扫描阶段：解析 `implements` 声明 → 构建 `interface → [impl_classes]` 映射 → 存入 BeanGraph。修改 bean_rules.rs 的缺失检测：除 name/class_name 外，还按接口映射匹配。

## Technical Context

**修改文件**: `src/model/bean_graph.rs` + `src/rule_engine/bean_rules.rs`（2 文件）
**新增依赖**: 0
**性能**: FR-I06 — 额外耗时 ≤0.1s（在现有源码扫描同一个文件读取中完成）

## Constitution Check: 10/10 PASS

## 实现策略

### bean_graph.rs — 接口解析

在 `process_java_file()` 中，新增正则匹配 `implements\s+([\w\s,.<>]+)`，提取接口名并存入 `BeanGraph`:
```rust
// 新增字段
pub interface_impls: HashMap<String, Vec<String>>, // interface_name → [impl_class_names]
```

### bean_rules.rs — 接口匹配

在 `detect_missing_beans` 中，构建 `interface_set: HashSet<(interface, impl_class)>`，检查依赖时额外匹配接口。
