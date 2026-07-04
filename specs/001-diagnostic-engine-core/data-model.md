# Data Model: Doctor 诊断引擎核心（MVP）

**Date**: 2026-07-04

## Entity Overview

```
DiagnosticReport (1) ─── (*) Issue
       │                       │
       │                       ├── (*) Evidence
       │                       │
       └── SystemModel          └── Severity (enum)

Plugin (1) ─── Scanner
       │─── ModelBuilder
       │─── EvidenceCollector
       └─── RuleProvider ─── (*) Rule ─── (*) Issue
```

## Core Entities

### DiagnosticReport

一次诊断执行的完整结果。

| Field | Type | Description |
|-------|------|-------------|
| `project_name` | `String` | 项目名称（从 pom.xml/build.gradle 提取） |
| `timestamp` | `DateTime<Utc>` | 诊断执行时间 |
| `duration_ms` | `u64` | 诊断耗时（毫秒） |
| `health_score` | `u8` | 健康评分 0-100，100 满分，ERROR -10，WARNING -3，INFO -1 |
| `system_overview` | `SystemOverview` | 系统技术栈概览 |
| `issues` | `Vec<Issue>` | 诊断问题列表（按严重程度降序排列） |
| `summary` | `DiagnosisSummary` | 诊断执行摘要 |

**Validation rules**:
- `health_score` MUST be in range [0, 100]
- 如果 `issues` 为空，`health_score` MUST be 100
- `issues` MUST be sorted by severity (ERROR > WARNING > INFO), then by category

### SystemOverview

技术栈快照。

| Field | Type | Description |
|-------|------|-------------|
| `build_tool` | `BuildTool` | 构建工具（Maven/Gradle） |
| `spring_boot_version` | `Option<String>` | Spring Boot 版本号 |
| `java_version` | `Option<String>` | Java 版本 |
| `starters` | `Vec<String>` | 检测到的 Spring Boot Starter 列表 |
| `module_count` | `usize` | Maven 模块数（多模块项目） |

### Issue

单个诊断发现的问题。

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | 唯一标识，格式：`{CATEGORY}-{NNN}` |
| `title` | `String` | 简短问题标题 |
| `severity` | `Severity` | 严重等级 |
| `category` | `Category` | 问题分类 |
| `description` | `String` | 详细问题描述 |
| `evidence` | `Vec<Evidence>` | 支撑证据列表（至少 1 条） |
| `fix_suggestion` | `String` | 修复建议（可为空字符串，表示无具体建议） |
| `confidence` | `Confidence` | 置信度 |

**Validation rules**:
- `evidence` MUST NOT be empty (SC-003: 每个问题至少 1 条可追溯证据)
- `title` SHOULD be ≤120 characters
- `description` MUST include file/class references when applicable

### Evidence

支撑诊断结论的单个事实。

| Field | Type | Description |
|-------|------|-------------|
| `evidence_type` | `EvidenceType` | 证据类型 |
| `source` | `String` | 来源标识（`文件路径:行号` 或 `endpoint URL`） |
| `summary` | `String` | 证据内容摘要 |
| `reliability` | `Reliability` | 可信度等级 |

**Validation rules**:
- `source` MUST be parseable as a file location or URL
- `summary` MUST NOT contain full source code or config values (only descriptive summary)

### SystemModel

系统结构化描述（内部使用，不直接输出到报告）。

| Field | Type | Description |
|-------|------|-------------|
| `bean_graph` | `BeanGraph` | Bean 依赖图 |
| `auto_config_model` | `AutoConfigModel` | 自动配置模型 |
| `config_model` | `ConfigModel` | 配置属性模型 |

### Plugin

插件描述符。

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | 插件唯一名称 |
| `version` | `String` | 插件版本 |
| `description` | `String` | 插件功能描述 |
| `enabled` | `bool` | 是否启用 |
| `source_path` | `PathBuf` | 插件目录路径 |

## Sub-Models

### BeanGraph

```
BeanGraph
├── beans: Vec<BeanDef>
│   ├── name: String
│   ├── class_name: String
│   ├── scope: BeanScope (Singleton/Prototype/Request/Session)
│   ├── declared_in: String (声明该 Bean 的配置类或 XML 文件)
│   └── dependencies: Vec<String> (依赖的其他 Bean 名称)
└── edges: Vec<BeanDep>
    ├── from: String (Bean 名称)
    ├── to: String (Bean 名称)
    └── injection_type: InjectionType (Field/Constructor/Setter)
```

### AutoConfigModel

```
AutoConfigModel
├── enabled: Vec<AutoConfigClass>
│   ├── class_name: String
│   ├── condition_results: Vec<ConditionResult>
│   └── registered_beans: Vec<String>
├── disabled: Vec<DisabledAutoConfig>
│   ├── class_name: String
│   ├── failed_condition: String
│   └── reason: String
└── excluded: Vec<String> (用户显式排除的自动配置类)
```

### ConfigModel

```
ConfigModel
├── properties: Vec<ConfigProperty>
│   ├── key: String
│   ├── value_summary: String (值的摘要，不含敏感信息)
│   ├── sources: Vec<ConfigSource>
│   │   ├── source_type: ConfigSourceType
│   │   │   (ApplicationYml/ApplicationProperties/EnvVar/SystemProperty/CommandLine)
│   │   ├── location: String
│   │   └── priority: u8 (Spring 配置优先级: 1=highest)
│   └── conflicts: Vec<ConfigConflict> (同一 key 的多个来源值不一致时)
│       ├── source_a: ConfigSource
│       ├── value_a: String
│       ├── source_b: ConfigSource
│       └── value_b: String
```

## Enums

### Severity
```rust
enum Severity {
    Error,    // 扣 10 分 — 阻断性问题
    Warning,  // 扣 3 分  — 潜在风险
    Info,     // 扣 1 分  — 建议信息
}
```

### Category
```rust
enum Category {
    Bean,
    Config,
    Transaction,
    AutoConfig,
    Startup,
}
```

### EvidenceType
```rust
enum EvidenceType {
    SourceCode,    // 源码分析结果
    ConfigFile,    // 配置文件内容
    Runtime,       // Actuator endpoint 数据
}
```

### Reliability
```rust
enum Reliability {
    Confirmed,   // 已确认：来自可直接验证的来源
    Inferred,    // 推断：基于间接证据推理
    Unverified,  // 未验证：证据来源可靠性不确定
}
```

### Confidence
```rust
enum Confidence {
    High,     // >90% 确信
    Medium,   // 50-90%
    Low,      // <50%
}
```

## DiagnosisSummary

| Field | Type | Description |
|-------|------|-------------|
| `total_rules_executed` | `usize` | 执行的规则总数 |
| `total_evidence_collected` | `usize` | 收集的证据总数 |
| `issues_by_severity` | `HashMap<Severity, usize>` | 按严重等级统计的问题数 |
| `runtime_sources_available` | `bool` | Actuator endpoint 是否可达 |

## DTO: StructuredSummary (LLM Input)

发送给 LLM 的结构化摘要（FR-030）。

| Field | Type | Description |
|-------|------|-------------|
| `project_context` | `ProjectContext` | 项目基本信息 |
| `issues` | `Vec<IssueSummary>` | 问题摘要列表（最多 20 条，按严重程度优先） |

### ProjectContext
| Field | Type |
|-------|------|
| `spring_boot_version` | `Option<String>` |
| `build_tool` | `BuildTool` |
| `bean_count` | `usize` |
| `auto_config_count` | `usize` |

### IssueSummary
| Field | Type |
|-------|------|
| `issue_id` | `String` |
| `severity` | `Severity` |
| `category` | `Category` |
| `title` | `String` |
| `evidence_count` | `usize` |
| `key_classes` | `Vec<String>` (涉及的类名/Bean 名) |

**Privacy constraint**: MUST NOT contain file paths, source code snippets, or config values.
