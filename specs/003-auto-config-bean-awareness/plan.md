# Implementation Plan: 自动配置 Bean 感知

**Branch**: `003-auto-config-bean-awareness` | **Date**: 2026-07-04 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-auto-config-bean-awareness/spec.md`

## Summary

减少自动配置 Bean 假阳性 + 多模块 Bean 去重。核心改动：新增 `classpath_scanner` 模块（jar 扫描 + class 常量池解析 + 缓存），修改 `bean_graph.rs`（BeanDef 来源标记 + 依赖检查跳过 AutoConfig Bean），修改 `rule_engine/bean_rules.rs`（冲突去重）。

## Technical Context

**Language/Version**: Rust 1.85+ (edition 2024)

**Primary Dependencies**: 零新增。Java class 常量池解析用标准库二进制读取（`std::io::Read` + `byteorder`-style 手动解析，或直接用 `std` 的 `read_u16`/`read_u32` 等）。

**Storage**: `~/.doctor/cache/auto-config-beans.json` — SHA256 key 缓存文件。

**Testing**: `cargo test` — class 常量池解析单元测试 + jar 扫描集成测试 + mumu 端到端验证。

**Target Platform**: macOS/Linux。

**Performance Goals**: SC-A04 — 缓存命中 ≤1.0s 总耗时，首次扫描 ≤5s。

## Constitution Check

| # | Principle | Status |
|---|-----------|--------|
| I | 安全优先 | ✅ safe Rust，二进制读取 |
| II | 保持简单 | ✅ 手动解析 class 常量池，不引入 ASM |
| III | 遵循最佳实践 | ✅ 标准库 + 已有 deps |
| IV | 正确性优先 | ✅ 首次全量扫描不遗漏任何 jar |
| V | 测试驱动质量 | ✅ 常量池解析 UT + jar 扫描 IT |
| VI | 正确处理错误 | ✅ jar/class 失败降级 |
| VII | 精简依赖 | ✅ 零新增 |
| VIII | 文档即设计 | ✅ pub 函数文档注释 |
| IX | 保持一致性 | ✅ 遵循现有模块结构 |
| X | 持续改进 | ✅ 直接提升诊断可用性 |

**Gate Result**: ALL PASS.

## Project Structure

### 新增文件

```text
src/classpath/
├── mod.rs             # classpath 扫描入口 + 缓存逻辑
├── jar_scanner.rs     # walk Gradle/Maven 缓存，查找含 AutoConfiguration.imports 的 jar
├── class_parser.rs    # class 文件常量池解析，提取 @Bean 方法
└── cache.rs           # SHA256 hash 生成，缓存读写

src/output/
└── mask.rs            # (已存在，无需修改)
```

### 修改文件

```text
src/model/
├── bean_graph.rs      # BeanDef 增加 source 字段 (UserDefined/AutoConfig)
│                      # build_bean_graph 调用 classpath scanner 注册自动配置 Bean
└── issue.rs           # (无需修改)

src/rule_engine/
└── bean_rules.rs      # 缺失检测跳过 AutoConfig Bean (FR-A08)
                       # 冲突检测按模块路径去重 (FR-A06)

src/cli/
└── diagnose.rs        # pipeline 中加入 classpath scan 步骤
```

## 实现策略

### 1. Class 文件常量池解析 (最核心)

```text
class 文件结构:
  magic (4 bytes) + version (4 bytes)
  constant_pool_count (2 bytes)
  constant_pool[]: 每个 entry 以 tag (1 byte) 开头
    tag=1 → CONSTANT_Utf8 { length(2), bytes[length] }
  ...
  methods_count (2 bytes)
  methods[]:
    access_flags (2 bytes) — 包含注解标记
    name_index (2 bytes) → 方法名
    descriptor_index (2 bytes) → 返回类型描述符
    attributes[] → RuntimeVisibleAnnotations → @Bean 检测
```

手动解析逻辑（无需外部 crate）：
1. 读 magic + version → 跳过
2. 读 constant_pool_count → 遍历 constant_pool 收集 UTF8 常量
3. 读 access_flags + this_class + super_class → 跳过
4. 读 interfaces + fields → 跳过
5. 读 class-level RuntimeVisibleAnnotations → 检测 4 类 Bean 定义:
   a) @Component 派生: `Lorg/springframework/stereotype/{Component,Service,Repository,Controller};` + `L.../RestController;` + `L.../Configuration;` → 类自身是 Bean
   b) @Import: `L.../Import;` → 提取 value 数组中的类名 → 递归一级 (深度 2)
   c) @ConfigurationProperties: `L.../ConfigurationProperties;` → 标记为 AutoConfig Bean
6. 读 methods: 对每个 method, 检查 RuntimeVisibleAnnotations → 匹配 `L.../Bean;` → 提取 name_index、descriptor_index、及 @Bean 的 name/value 属性 → 得到方法名、显式 Bean 名、返回类型

### 2. Jar 扫描

```text
~/.gradle/caches/modules-2/files-2.1/
→ walkdir 遍历所有 .jar 文件
→ 对每个 jar: zip::ZipArchive::new(jar_file)
  → 查找 META-INF/spring/AutoConfiguration.imports
  → 找到 → 读出类名列表
  → 对每个类: 在 jar 中定位 .class 文件 → 常量池解析 → 提取 @Bean/@Component/@Import/@ConfigurationProperties Bean
  → 汇总为 Vec<AutoConfigBean>
```

### 3. 缓存

```text
hash = SHA256(build.gradle.kts + settings.gradle.kts + libs.versions.toml)
cache_path = ~/.doctor/cache/auto-config-beans-{hash}.json
if exists → 直接加载
else → 全量扫描 → 序列化到 cache_path
```

## Complexity Tracking

> No violations.
