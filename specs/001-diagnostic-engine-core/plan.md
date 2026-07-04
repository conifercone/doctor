# Implementation Plan: Doctor 诊断引擎核心（MVP）

**Branch**: `001-diagnostic-engine-core` | **Date**: 2026-07-04 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-diagnostic-engine-core/spec.md`

## Summary

构建 Doctor 诊断引擎核心（MVP），一个面向 Spring Boot 项目的 CLI-first 智能诊断工具。系统通过静态分析（源码 + 配置文件）结合可选的运行时信息（Actuator endpoints），自动执行诊断规则（Bean 缺失/冲突/循环依赖、自动配置失败、配置冲突、事务失效、启动分析），输出带健康评分和证据追溯的诊断报告，并支持 AI 自然语言解释。

技术方案：Rust (edition 2024) 实现，基于 trait 的插件架构。核心诊断引擎完全离线运行，AI 解释通过外部 LLM API 实现。项目采用 workspace 单 crate 结构，分为 scanner、model、evidence、rule_engine、ai、cli、output、plugin 八个核心模块。

## Technical Context

**Language/Version**: Rust 1.85+ (stable, edition 2024)

**Primary Dependencies**:
- `clap` (derive) — CLI argument parsing
- `serde` / `serde_json` / `serde_yaml` — serialization
- `thiserror` — structured error types
- `tokio` (rt-multi-thread) — async runtime
- `reqwest` (native-tls) — HTTP client for Actuator + LLM API
- `quick-xml` — Maven POM XML parsing
- `walkdir` — project directory traversal
- `colored` — terminal output formatting

**Storage**: File-based — diagnostic reports as JSON/YAML on local filesystem. SARIF/Markdown/HTML output files. Plugin directory `~/.doctor/plugins/`. Optional config file `.doctor.toml`.

**Testing**: `cargo test` (unit tests + integration tests + doctests). `cargo-tarpaulin` for coverage.

**Target Platform**: macOS (primary), Linux (CI/CD). Single binary distribution.

**Project Type**: CLI application (single crate, modular architecture)

**Performance Goals**: ≤30s for 50-Bean Spring Boot project diagnosis (SC-001); ≤60s CI/CD full pipeline including diagnosis (SC-008)

**Constraints**: Core diagnostic engine MUST run fully offline. AI explain degrades gracefully without network. No unsafe Rust without `// SAFETY:` justification (Constitution I). All fallible operations use Result-based error handling (Constitution VI).

**Scale/Scope**: MVP: Spring Boot 2.x/3.x + Maven/Gradle. Single project at a time. ~500 Beans max tested.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | Notes |
|---|-----------|--------|-------|
| I | 安全优先 | ✅ PASS | Pure safe Rust; no `unsafe` required for diagnostic engine |
| II | 保持简单 | ✅ PASS | Single crate, trait-based plugin system (not dynamic loading), no unnecessary abstractions in MVP |
| III | 遵循最佳实践 | ✅ PASS | clap/serde/thiserror — Rust ecosystem standards; cargo fmt + clippy |
| IV | 正确性优先 | ✅ PASS | Diagnostic rules operate on structured models; evidence traces every conclusion; no premature optimization |
| V | 测试驱动质量 | ✅ PASS | Tests planned: doctests for pub APIs, unit tests for rule engine, integration tests for end-to-end diagnose flow |
| VI | 正确处理错误 | ✅ PASS | `thiserror` enum errors; no `unwrap()` in production code; all errors user-facing with context |
| VII | 精简依赖 | ✅ PASS | 8 direct deps — all justified: clap (CLI), serde* (serialization), thiserror (errors), tokio (async), reqwest (HTTP), quick-xml (POM parsing), walkdir (file traversal), colored (terminal) |
| VIII | 文档即设计 | ✅ PASS | All `pub` items will have `///` docs; module-level `//!` docs for design intent |
| IX | 保持一致性 | ⏳ PLAN | cargo fmt + clippy in CI; Conventional Commits |
| X | 持续改进 | ✅ PASS | Modular design enables incremental plugin additions |

**Gate Result**: ALL PASS — Proceed to Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/001-diagnostic-engine-core/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output — CLI command schemas
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point: CLI arg parsing, command dispatch
├── scanner/             # Project type detection, dependency analysis
│   ├── mod.rs
│   ├── maven.rs         # pom.xml parser
│   └── gradle.rs        # build.gradle parser
├── model/               # System model data structures
│   ├── mod.rs
│   ├── bean_graph.rs    # Bean dependency graph
│   ├── auto_config.rs   # Auto-configuration model
│   └── config.rs        # Configuration property model
├── evidence/            # Evidence collection
│   ├── mod.rs
│   ├── source.rs        # Java source code analysis
│   ├── config_file.rs   # YAML/properties file parsing
│   └── actuator.rs      # Spring Boot Actuator client
├── rule_engine/         # Diagnostic rule execution
│   ├── mod.rs
│   ├── bean_rules.rs    # Bean missing/conflict/circular dependency
│   ├── config_rules.rs  # Configuration conflict detection
│   ├── auto_config_rules.rs  # Auto-configuration analysis
│   ├── transaction_rules.rs  # @Transactional validation
│   └── startup_rules.rs # Startup performance analysis
├── ai/                  # AI explanation
│   ├── mod.rs
│   └── explain.rs       # LLM prompt builder + API client
├── cli/                 # CLI command definitions
│   ├── mod.rs
│   ├── diagnose.rs      # `doctor diagnose` subcommand
│   └── explain.rs       # `doctor explain` subcommand
├── output/              # Report formatting
│   ├── mod.rs
│   ├── terminal.rs      # Colored terminal output
│   ├── json.rs          # JSON format
│   ├── markdown.rs      # Markdown format
│   ├── html.rs          # HTML format
│   └── sarif.rs         # SARIF format
├── config.rs            # .doctor.toml configuration file loading
└── plugin/              # Plugin system
    ├── mod.rs
    ├── registry.rs      # Plugin discovery + loading
    └── traits.rs        # Scanner/ModelBuilder/EvidenceCollector/RuleProvider/DiagnosticRule traits

tests/
├── integration/
│   └── diagnose_test.rs # End-to-end diagnose on sample projects
└── unit/
    ├── rule_engine/     # Per-rule unit tests
    ├── scanner/         # Maven/Gradle parser tests
    └── evidence/        # Source analysis tests
```

**Structure Decision**: Single crate (not workspace) for MVP. Modules are logically separated into directories but compiled as one binary. This avoids unnecessary workspace complexity (Constitution II) while maintaining clear code boundaries. Plugin system uses Rust trait objects — no dynamic library loading in MVP.

## Complexity Tracking

> No constitution violations. Table intentionally empty.
