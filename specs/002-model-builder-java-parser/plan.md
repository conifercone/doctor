# Implementation Plan: Java 源码模型构建器

**Branch**: `002-model-builder-java-parser` | **Date**: 2026-07-04 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/002-model-builder-java-parser/spec.md`

## Summary

实现 Model Builder：Bean 图构建器（正则解析 Java 源码 → BeanGraph）、自动配置模型构建器（解析 AutoConfiguration.imports + @Conditional → AutoConfigModel）、配置模型构建器（解析 YAML/Properties → ConfigModel）。替换 `doctor diagnose` pipeline 中的 `Default::default()` stub，使诊断规则产出真实结果。

## Technical Context

**Language/Version**: Rust 1.85+ (stable, edition 2024)

**Primary Dependencies**: 零新增。复用 `regex`、`serde_yaml`、`walkdir`。

**Testing**: `cargo test` — builder 单元测试 + mumu 端到端验证。

**Target Platform**: macOS/Linux CLI。

**Project Type**: 现有 CLI 增量 — 修改 `src/model/`（4 文件）+ `src/cli/diagnose.rs`（1 文件）。

**Performance Goals**: SC-M01: 100 Bean ≤10s。SC-M02: 检出率 ≥95%。

**Constraints**: 纯静态分析，不编译项目，不依赖 Maven/Gradle 运行。

## Constitution Check

*GATE: Must pass before implementation.*

| # | Principle | Status |
|---|-----------|--------|
| I | 安全优先 | ✅ safe Rust |
| II | 保持简单 | ✅ 正则匹配，不引入 Java parser |
| III | 遵循最佳实践 | ✅ 复用现有 deps |
| IV | 正确性优先 | ✅ ≥95% 检出率目标 |
| V | 测试驱动质量 | ✅ 每 builder 子模块单元测试 |
| VI | 正确处理错误 | ✅ 跳过格式错误文件，不崩溃 |
| VII | 精简依赖 | ✅ 零新增 |
| VIII | 文档即设计 | ✅ pub 函数文档注释 |
| IX | 保持一致性 | ✅ 遵循现有模块结构 |
| X | 持续改进 | ✅ 替换 stub → 真实实现 |

**Gate Result**: ALL PASS.

## Project Structure

### 修改的文件

```text
src/model/
├── bean_graph.rs        # ADD: build_bean_graph(project_path) → BeanGraph
├── auto_config.rs       # ADD: build_auto_config_model(project_path) → AutoConfigModel
├── config.rs            # ADD: build_config_model(project_path) → ConfigModel
└── system_model.rs      # ADD: build_system_model(project_path) → SystemModel (facade)

src/cli/
└── diagnose.rs          # MODIFY: replace Default::default() with system_model::build_system_model()
```

### 实现策略

**Bean 图构建** (`bean_graph.rs`):
1. `walkdir` 遍历所有子模块 `src/main/java/` 下的 `.java` 文件
2. 逐行正则匹配识别: stereotype 注解（@Component/@Service/...）、@Bean 方法、@Autowired 注入、@Qualifier、Lombok @RequiredArgsConstructor
3. import 解析推断完整类名
4. 构建 `BeanGraph { beans: Vec<BeanDef>, edges: Vec<BeanDep> }`

**自动配置模型** (`auto_config.rs`):
1. 解析 `AutoConfiguration.imports` 文件获取候选列表
2. 扫描对应 Java 源文件中的 @ConditionalOnXxx 注解
3. 判断条件满足/不满足状态
4. 构建 `AutoConfigModel`

**配置模型** (`config.rs`):
1. `walkdir` 找到所有 `application*.yml/yaml/properties`
2. 解析 YAML（多文档支持）和 Properties 格式
3. 同一 key 多源 → 标记冲突
4. 构建 `ConfigModel`

**Pipeline 集成** (`diagnose.rs`):
1. 替换 `SystemModel::new(Default::default(), ...)` 为 `system_model::build_system_model(&canonical_path)`
2. 处理构建失败（error 而非 panic）

## Complexity Tracking

> No violations.
