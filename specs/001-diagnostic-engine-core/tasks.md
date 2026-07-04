# Tasks: Doctor 诊断引擎核心（MVP）

**Input**: Design documents from `specs/001-diagnostic-engine-core/`

**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Tests are included per Constitution V (测试驱动质量) — core business logic and public interfaces require tests.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single Rust crate**: `src/`, `tests/` at repository root
- Modules: `src/scanner/`, `src/model/`, `src/evidence/`, `src/rule_engine/`, `src/ai/`, `src/cli/`, `src/output/`, `src/plugin/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependency configuration, CI setup

- [x] T001 Update Cargo.toml with all required dependencies: clap (derive), serde (derive), serde_json, serde_yaml, thiserror, tokio (rt-multi-thread), reqwest (native-tls), quick-xml, walkdir, colored, regex, chrono (serde), anyhow in Cargo.toml
- [x] T002 [P] Create rustfmt.toml with project formatting rules (max_width=100, edition=2024) in rustfmt.toml
- [x] T003 [P] Create clippy.toml with allowed lints configuration in clippy.toml
- [x] T004 [P] Create .github/workflows/ci.yml with CI pipeline: cargo fmt --check, cargo clippy -- -D warnings, cargo test, cargo tarpaulin in .github/workflows/ci.yml
- [x] T005 Create module declarations: scaffold all `mod.rs` files with module structure per plan.md in src/main.rs, src/scanner/mod.rs, src/model/mod.rs, src/evidence/mod.rs, src/rule_engine/mod.rs, src/ai/mod.rs, src/cli/mod.rs, src/output/mod.rs, src/plugin/mod.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data types, error infrastructure, and plugin traits — MUST be complete before ANY user story

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Error Handling

- [x] T006 [P] Define DoctorError enum with all error variants (ProjectNotFound, ParseError, NetworkError, RuleExecutionError, etc.) using thiserror in src/error.rs
- [x] T007 [P] Define DoctorResult<T> type alias in src/error.rs

### Core Data Types & Enums

- [x] T008 [P] Define Severity enum (Error/Warning/Info) with Display impl and health score deduction values (10/3/1) in src/model/severity.rs
- [x] T009 [P] Define Category enum (Bean/Config/Transaction/AutoConfig/Startup) in src/model/category.rs
- [x] T010 [P] Define EvidenceType enum (SourceCode/ConfigFile/Runtime) and Reliability enum (Confirmed/Inferred/Unverified) in src/evidence/types.rs
- [x] T011 [P] Define Confidence enum (High/Medium/Low) in src/model/confidence.rs

### Evidence & Issue Data Structures

- [x] T012 [P] Define Evidence struct (evidence_type, source, summary, reliability) with serde support in src/evidence/types.rs
- [x] T013 [P] Define Issue struct (id, title, severity, category, description, evidence, fix_suggestion, confidence) with serde support; validate evidence not empty in src/model/issue.rs

### System Model Structures

- [x] T014 [P] Define BeanDef, BeanDep, InjectionType, BeanScope, BeanGraph structs in src/model/bean_graph.rs
- [x] T015 [P] Define AutoConfigClass, ConditionResult, DisabledAutoConfig, AutoConfigModel structs in src/model/auto_config.rs
- [x] T016 [P] Define ConfigProperty, ConfigSource, ConfigSourceType, ConfigConflict, ConfigModel structs in src/model/config.rs
- [x] T017 Define SystemModel struct aggregating BeanGraph, AutoConfigModel, ConfigModel in src/model/system_model.rs

### Diagnostic Report Structures

- [x] T018 [P] Define SystemOverview struct (build_tool, spring_boot_version, java_version, starters, module_count) with BuildTool enum in src/model/system_overview.rs
- [x] T019 [P] Define DiagnosisSummary struct (total_rules_executed, total_evidence_collected, issues_by_severity, runtime_sources_available) in src/model/summary.rs
- [x] T020 Define DiagnosticReport struct (project_name, timestamp, duration_ms, health_score, system_overview, issues, summary) with health score computation logic in src/model/report.rs

### Plugin System Foundation

- [x] T021 [P] Define Scanner trait (name, detect, scan) in src/plugin/traits.rs
- [x] T022 [P] Define ModelBuilder trait (name, build_bean_graph, build_auto_config_model, build_config_model) in src/plugin/traits.rs
- [x] T023 [P] Define EvidenceCollector trait (name, collect_source_evidence, collect_config_evidence, collect_runtime_evidence) in src/plugin/traits.rs
- [x] T024 [P] Define DiagnosticRule trait (id, name, category, diagnose) and RuleProvider trait (name, rules) in src/plugin/traits.rs
- [x] T025 [P] Define PluginDescriptor struct (name, version, description, source_path) with serde support in src/plugin/registry.rs
- [x] T026 Implement PluginRegistry: scan directory for plugins, load descriptors (TOML), filter enabled plugins in src/plugin/registry.rs

**Checkpoint**: Foundation ready — user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Spring Boot 项目健康诊断 (Priority: P1) 🎯 MVP

**Goal**: 实现 `doctor diagnose` 命令，自动扫描 Spring Boot 项目、建立系统模型、收集证据、执行诊断规则、输出包含健康评分的诊断报告。

**Independent Test**: 在一个有已知配置问题的 Spring Boot 项目中运行 `doctor diagnose`，验证终端输出包含系统概览、健康评分（0-100）、至少 1 个诊断问题（带证据追溯）。

### Scanner Implementation

- [x] T027 [P] [US1] Implement Maven project detection: parse pom.xml with quick-xml, extract groupId/artifactId/version, dependencies, starters, Spring Boot version in src/scanner/maven.rs
- [x] T028 [P] [US1] Implement Gradle project detection: parse build.gradle/build.gradle.kts, extract dependencies and Spring Boot plugin version in src/scanner/gradle.rs
- [x] T029 [US1] Implement Scanner facade: detect build tool type, dispatch to Maven or Gradle parser, return SystemOverview in src/scanner/mod.rs

### Model Builder Implementation

- [x] T030 [P] [US1] Implement Bean graph builder: scan Java source files for @Bean, @Component, @Service, @Repository, @Controller annotations; extract class names, dependencies (@Autowired, constructor params) in src/model/bean_graph.rs
- [x] T031 [P] [US1] Implement Auto-config model builder: parse spring.factories / org.springframework.boot.autoconfigure.AutoConfiguration.imports; analyze @ConditionalOnClass, @ConditionalOnMissingBean, @ConditionalOnProperty conditions in src/model/auto_config.rs
- [x] T032 [P] [US1] Implement Config model builder: parse application.yml, application.properties (all profiles); collect environment variables and system properties; track source priority in src/model/config.rs
- [x] T033 [US1] Implement ModelBuilder facade: orchestrate BeanGraph + AutoConfigModel + ConfigModel construction; produce SystemModel in src/model/system_model.rs

### Evidence Engine Implementation

- [x] T034 [P] [US1] Implement source code evidence collector: scan Java files for annotations, class declarations, method signatures, field injections; produce Evidence records with file:line sources in src/evidence/source.rs
- [x] T035 [P] [US1] Implement config file evidence collector: parse YAML and Properties files; extract all key-value pairs with source location in src/evidence/config_file.rs
- [x] T036 [US1] Implement Actuator runtime evidence collector: async HTTP client to query /actuator/health, /actuator/env, /actuator/beans, /actuator/conditions, /actuator/configprops; handle connection timeout and 404 gracefully in src/evidence/actuator.rs
- [x] T037 [US1] Implement EvidenceEngine facade: orchestrate all evidence collectors; merge results with deduplication; tag reliability levels in src/evidence/mod.rs

### Rule Engine Implementation

- [x] T038 [P] [US1] Implement Bean missing detection rule: for each @Autowired dependency, verify target Bean exists in BeanGraph; produce Issue with BEAN-XXX ID in src/rule_engine/bean_rules.rs
- [x] T039 [P] [US1] Implement Bean conflict detection rule: detect multiple beans of same type without @Qualifier at injection site in src/rule_engine/bean_rules.rs
- [x] T040 [P] [US1] Implement Circular dependency detection rule: DFS on BeanGraph to find cycles; report the full dependency chain in src/rule_engine/bean_rules.rs
- [x] T041 [P] [US1] Implement Auto-config failure detection rule: analyze disabled auto-config classes; check if expected starters have missing auto-config in src/rule_engine/auto_config_rules.rs
- [x] T042 [P] [US1] Implement Config conflict detection rule: detect same property key with different values across multiple ConfigSources in src/rule_engine/config_rules.rs
- [x] T043 [P] [US1] Implement Transaction invalidation detection rule: check @Transactional on non-public methods, self-invocation patterns, unchecked exception handling in src/rule_engine/transaction_rules.rs
- [x] T044 [P] [US1] Implement Conditional assembly failure detection rule: check @ConditionalOnXxx conditions that caused expected beans to not be created in src/rule_engine/auto_config_rules.rs
- [x] T045 [P] [US1] Implement Startup analysis rule: identify beans with longest initialization paths; flag auto-config classes with slow conditions in src/rule_engine/startup_rules.rs
- [x] T046 [US1] Implement RuleEngine facade: register all rules from built-in Spring Boot RuleProvider; execute all rules sequentially; collect and sort Issues by severity; compute health score (100 - 10*ERRORs - 3*WARNINGs - 1*INFOs, min 0) in src/rule_engine/mod.rs

### CLI Integration

- [x] T047 [US1] Implement `doctor diagnose` subcommand: clap derive struct with --output, --plugin, --no-ai, --offline, --timeout, --verbose flags; wire to diagnose pipeline in src/cli/diagnose.rs
- [x] T048 [US1] Implement main CLI entry point: clap command enum with diagnose and explain subcommands; dispatch to handler functions in src/main.rs

### Terminal Output

- [x] T049 [US1] Implement terminal formatter: colored output with health score, system overview box, issue list sorted by severity (red ERROR, yellow WARNING, blue INFO), evidence references in src/output/terminal.rs

### US1 Integration & Wire-up

- [x] T050 [US1] Implement DiagnosePipeline: orchestrate Scanner → ModelBuilder → EvidenceEngine → RuleEngine → OutputFormatter flow; handle offline mode flag; measure and log duration in src/cli/diagnose.rs
- [x] T051 [US1] Write integration test: end-to-end diagnose on a fixture Spring Boot project with known Bean missing and config conflict issues; verify report structure and evidence traceability in tests/integration/diagnose_test.rs

**Checkpoint**: At this point, `doctor diagnose` should be fully functional — scan → model → evidence → rules → terminal report

---

## Phase 4: User Story 2 - AI 解释诊断结果 (Priority: P2)

**Goal**: 实现 `doctor explain` 命令，将诊断结果的结构化摘要发送给外部 LLM，生成自然语言问题解释。

**Independent Test**: 对一份 JSON 诊断报告运行 `doctor explain`，验证 AI 输出包含每个问题的自然语言解释、根因分析和修复建议。

### Structured Summary DTO

- [x] T052 [P] [US2] Define ProjectContext and IssueSummary DTOs for LLM input (per data-model.md StructuredSummary) with serde in src/ai/summary.rs
- [x] T053 [P] [US2] Implement summary builder: convert DiagnosticReport + Issues into StructuredSummary; filter to top 20 issues by severity; strip source code and config values per FR-030 in src/ai/summary.rs

### LLM API Client

- [x] T054 [US2] Implement LLM client: async reqwest POST to OpenAI-compatible /v1/chat/completions; configurable api_url, api_key (from env DOCTOR_LLM_KEY), model; system prompt with diagnostic context; handle timeout and API errors gracefully in src/ai/explain.rs
- [x] T055 [US2] Implement AI explanation formatter: parse LLM response; structure output with problem description, root cause analysis, impact scope, fix suggestions per issue; handle non-JSON LLM responses gracefully in src/ai/explain.rs

### CLI Integration

- [x] T056 [US2] Implement `doctor explain` subcommand: clap derive struct with REPORT arg, --api-url, --api-key, --model, --locale flags; load report JSON, build summary, call LLM, display formatted explanation in src/cli/explain.rs
- [x] T057 [US2] Integrate `doctor explain` into main CLI command enum in src/main.rs

### US2 Integration

- [x] T058 [US2] Write integration test: provide fixture diagnostic report JSON (10 known-root-cause Spring Boot issues); verify LLM client constructs correct structured summary (no source code in payload); verify response contains required sections (description/root cause/impact/fix); verify graceful degradation on network failure in tests/integration/explain_test.rs. Note: SC-005 (80% fix accuracy) is a post-launch evaluation metric via user feedback, not a build-time assertion.

**Checkpoint**: At this point, `doctor diagnose` + `doctor explain` both work independently

---

## Phase 5: User Story 3 - 诊断报告输出 (Priority: P3)

**Goal**: 支持多种输出格式（JSON、Markdown、HTML、SARIF），敏感信息自动脱敏。

**Independent Test**: 对同一项目运行 `--output json|markdown|html|sarif`，验证每种格式的结构正确性和内容完整性。

### Output Formatters

- [x] T059 [P] [US3] Implement JSON formatter: serde serialize DiagnosticReport; validate JSON structure matches contract schema in src/output/json.rs
- [x] T060 [P] [US3] Implement Markdown formatter: generate structured markdown with headings, tables for issues, code blocks for evidence, health score badge in src/output/markdown.rs
- [x] T061 [P] [US3] Implement HTML formatter: generate standalone HTML report with inline CSS; health score gauge, issue cards, collapsible evidence sections in src/output/html.rs
- [x] T062 [P] [US3] Implement SARIF formatter: map DiagnosticReport to SARIF v2.1.0 schema; map Severity to SARIF levels (error/warning/note); map Evidence.source to physicalLocation in src/output/sarif.rs

### Sensitive Info Masking

- [x] T063 [P] [US3] Implement sensitive value detector: regex patterns for passwords, API keys, tokens, connection strings, private keys in src/output/mask.rs
- [x] T064 [US3] Integrate masking into all output formatters: mask sensitive values in terminal, JSON, Markdown, HTML, SARIF outputs; preserve structure but replace values with `***` in src/output/mod.rs

### CLI Integration

- [x] T065 [US3] Wire `--output` flag: implement OutputFormat enum (Terminal, Json, Markdown, Html, Sarif) with Display; dispatch to correct formatter in diagnose pipeline; handle file vs stdout output in src/cli/diagnose.rs and src/output/mod.rs

### US3 Integration

- [x] T066 [US3] Write integration test: run diagnose --output json, --output sarif on fixture project; validate JSON schema, SARIF structure, markdown completeness; verify sensitive values are masked in all formats in tests/integration/output_test.rs

**Checkpoint**: All three user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Configuration, documentation, and project health

- [x] T067 [P] Implement .doctor.toml config file loading: parse plugins.enabled, ai.api_url/api_key_env/model, output.default_format/color, diagnosis.timeout_seconds/max_issues; merge with CLI args (CLI takes precedence) in src/config.rs
- [x] T068 [P] Add module-level documentation (//! comments) explaining design intent for each module: scanner, model, evidence, rule_engine, ai, cli, output, plugin in src/scanner/mod.rs, src/model/mod.rs, src/evidence/mod.rs, src/rule_engine/mod.rs, src/ai/mod.rs, src/cli/mod.rs, src/output/mod.rs, src/plugin/mod.rs
- [x] T069 [P] Add pub API documentation (/// comments) for all public types, traits, and functions across all modules
- [x] T070 Run cargo fmt and cargo clippy -- -D warnings; fix all issues
- [x] T071 Run cargo test --lib and verify all unit tests pass
- [x] T072 Run cargo test and verify all integration tests pass
- [x] T073 [P] Add performance benchmark: time full diagnose pipeline on 50-Bean fixture project (assert ≤30s per SC-001) in tests/bench/diagnose_bench.rs
- [x] T074 [P] Add CI timing assertion: verify diagnose completes within 60s on CI runner per SC-008 in specs/001-diagnostic-engine-core/quickstart.md
- [x] T075 Validate quickstart.md scenarios: run all 8 verification scenarios manually; fix any issues found in docs/specs/001-diagnostic-engine-core/quickstart.md
- [x] T076 Update README.md with project overview, installation, usage examples, and link to docs

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup (T001, T005) — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational phase completion
- **User Story 2 (Phase 4)**: Depends on Foundational phase completion; depends on US1 for DiagnosticReport struct
- **User Story 3 (Phase 5)**: Depends on Foundational phase completion; depends on US1 for DiagnosticReport data
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) — No dependencies on other stories. Foundation for US2 and US3.
- **User Story 2 (P2)**: Can start after Foundational (Phase 2). Needs DiagnosticReport from Phase 2 (T020), not from US1 implementation. Can develop in parallel with US1.
- **User Story 3 (P3)**: Can start after Foundational (Phase 2). Needs DiagnosticReport struct from Phase 2. Can develop in parallel with US1 and US2.

### Within User Story 1

- Scanner (T027-T029) and Model Builder (T030-T033) can run in parallel
- Evidence Engine (T034-T037) depends on Model Builder structs
- Rule Engine (T038-T046) depends on Model Builder structs + Evidence types
- CLI (T047-T048) and Output (T049) depend on Rule Engine facade
- Integration test (T050-T051) depends on all US1 implementation

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel (T002, T003, T004)
- All Foundational tasks within same subgroup can run in parallel
- US1 ModelBuilder tasks (T030, T031, T032) can run in parallel
- US1 Rule Engine tasks (T038-T045) can ALL run in parallel (different files, rule trait)
- US2 summary DTOs (T052, T053) can run in parallel
- US3 output formatters (T059, T060, T061, T062) can ALL run in parallel
- Once Foundational completes, US1 + US2 + US3 can develop in parallel (if multiple developers)

---

## Parallel Example: User Story 1 Rule Engine

```bash
# Launch all 8 rule implementations in parallel:
Task: "Implement Bean missing detection rule in src/rule_engine/bean_rules.rs"
Task: "Implement Bean conflict detection rule in src/rule_engine/bean_rules.rs"
Task: "Implement Circular dependency detection rule in src/rule_engine/bean_rules.rs"
Task: "Implement Auto-config failure detection rule in src/rule_engine/auto_config_rules.rs"
Task: "Implement Config conflict detection rule in src/rule_engine/config_rules.rs"
Task: "Implement Transaction invalidation detection rule in src/rule_engine/transaction_rules.rs"
Task: "Implement Conditional assembly failure detection rule in src/rule_engine/auto_config_rules.rs"
Task: "Implement Startup analysis rule in src/rule_engine/startup_rules.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: `cargo run -- diagnose /path/to/spring-boot-project`
5. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → `doctor diagnose` works → Demo (MVP!)
3. Add User Story 2 → `doctor explain` works → Demo
4. Add User Story 3 → JSON/Markdown/HTML/SARIF output → Demo
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (scanner + model + evidence + rules + terminal)
   - Developer B: User Story 2 (AI explain pipeline)
   - Developer C: User Story 3 (output formatters + masking)
3. Stories integrate through shared structs from Phase 2

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group with Conventional Commits format
- Stop at any checkpoint to validate story independently
- Run `cargo fmt` and `cargo clippy` after each task group
- All `pub` items MUST have `///` documentation (Constitution VIII)
- All fallible functions MUST return DoctorResult<T> (Constitution VI)
- No `unwrap()` or `expect()` in production code (Constitution VI)
