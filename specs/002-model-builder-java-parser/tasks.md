# Tasks: Java 源码模型构建器

**Input**: Design documents from `specs/002-model-builder-java-parser/`

**Prerequisites**: plan.md (required), spec.md (required)

**Tests**: Tests are included per Constitution V — core business logic requires tests.

**Organization**: Tasks are grouped by user story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Bean 图构建 (US1 — P1) 🎯 MVP

**Goal**: 实现 `build_bean_graph()`，解析所有 Java 源码构建 Bean 依赖图。

**Independent Test**: 对包含 Controller → Service → Repository 依赖链的项目运行诊断，验证 Bean 图包含全部 3 个 Bean 及依赖边。

### Implementation

- [x] T001 [US1] Implement Java source scanner: walkdir traverse all `src/main/java/` directories across root + submodules; collect `.java` file paths in src/model/bean_graph.rs
- [x] T002 [US1] Implement stereotype annotation detection: regex match @Component, @Service, @Repository, @Controller, @RestController, @Configuration; extract class name and annotation type; build BeanDef entries in src/model/bean_graph.rs
- [x] T003 [US1] Implement @Bean method detection: in @Configuration classes, find @Bean-annotated methods; use method name as Bean name, return type as class_name in src/model/bean_graph.rs
- [x] T004 [US1] Implement @Autowired injection detection: regex match @Autowired on fields, constructors; extract target type; create BeanDep edges with InjectionType in src/model/bean_graph.rs
- [x] T005 [US1] Implement @Qualifier handling: extract qualifier value from @Qualifier("name") annotations at injection points; record in BeanDep metadata in src/model/bean_graph.rs
- [x] T006 [US1] Implement Lombok constructor injection: detect @RequiredArgsConstructor or @AllArgsConstructor; treat all private final fields as constructor-injected dependencies in src/model/bean_graph.rs
- [x] T007 [US1] Implement import resolution: parse import statements to resolve simple class names → fully qualified names; fallback to simple name when unresolvable in src/model/bean_graph.rs
- [x] T008 [US1] Implement multi-module support: parse settings.gradle.kts include() statements; for each submodule, scan its src/main/java/ directory; aggregate all beans into single BeanGraph in src/model/bean_graph.rs

### Verification

- [x] T009 [US1] Write unit tests: prepare fixture directory with 20 known beans (@Service, @Repository, @Configuration+@Bean, @Autowired field/constructor); assert BeanGraph detects ≥19 beans (95%) and ≥10 edges; test @Qualifier edge metadata in src/model/bean_graph.rs
- [x] T010 [US1] Validate on real project: run doctor diagnose on mumu and verify BeanGraph beans.len() > 0 and edges.len() > 0

**Checkpoint**: `build_bean_graph()` returns populated BeanGraph from real Spring Boot projects.

---

## Phase 2: 配置 & 自动配置模型 (US2 — P2)

**Goal**: 实现 `build_config_model()` 和 `build_auto_config_model()`。

**Independent Test**: 对包含多 profile YAML 的项目运行，验证 ConfigModel 包含属性及来源；验证 AutoConfigModel 包含候选自动配置类列表。

### Config Model

- [x] T011 [P] [US2] Implement YAML config parser: walkdir find all application*.yml/yaml files; parse with serde_yaml; flatten nested keys to dot-notation; extract key-value pairs with source file path in src/model/config.rs
- [x] T012 [P] [US2] Implement Properties config parser: parse application*.properties files; extract key=value lines (skip comments); record source in src/model/config.rs
- [x] T013 [US2] Implement config conflict detection: for same key defined in multiple sources with different values, create ConfigConflict entries; track source priority in src/model/config.rs

### Auto-Config Model

- [x] T014 [US2] Implement AutoConfiguration.imports parser: locate and parse META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports in classpath jars or project dependencies; extract candidate class names in src/model/auto_config.rs
- [x] T015 [US2] Implement @Conditional annotation detection: scan auto-config class source files for @ConditionalOnClass, @ConditionalOnMissingBean, @ConditionalOnProperty; record condition results in src/model/auto_config.rs
- [x] T016 [US2] Implement enabled/disabled classification: mark auto-config classes with all conditions met as enabled, any failed condition → disabled with reason in src/model/auto_config.rs

### Verification

- [x] T017 [US2] Write unit tests: YAML parsing with multi-doc, multi-profile; properties parsing; config conflict detection in src/model/config.rs
- [x] T018 [US2] Write unit tests: AutoConfiguration.imports parsing, @Conditional detection in src/model/auto_config.rs

**Checkpoint**: `build_config_model()` and `build_auto_config_model()` produce populated models.

---

## Phase 3: Pipeline 集成 (US3 — P3)

**Goal**: 整合 Model Builder 到 diagnose pipeline，替换 Default::default() stub。

**Independent Test**: 对包含 Bean 缺失问题的项目运行 `doctor diagnose`，验证输出至少 1 个 ERROR 级别问题。

### Integration

- [x] T019 [US3] Implement build_system_model() facade: orchestrate build_bean_graph() + build_auto_config_model() + build_config_model(); return SystemModel in src/model/system_model.rs
- [x] T020 [US3] Replace stub in diagnose.rs: replace SystemModel::new(Default::default(), ...) with build_system_model(&canonical_path); handle build errors gracefully in src/cli/diagnose.rs
- [x] T021 [US3] Add error resilience: skip Java files with parse errors (log warning, continue); skip modules without src/main/java (no crash); handle missing config files gracefully in src/model/system_model.rs

### End-to-End Verification

- [x] T022 [US3] Run doctor diagnose on mumu project and verify: health_score < 100 (issues found) OR if 100 (all clean), verify Bean graph is populated with actual beans
- [x] T023 [US3] Run doctor diagnose --output json on mumu; verify JSON report contains non-empty bean_graph in system_model

---

## Phase 4: Polish

**Purpose**: Tests pass, formatting, final validation.

- [x] T024 Run cargo fmt and cargo clippy; fix all warnings
- [x] T025 Run cargo test and verify all tests pass (existing 15 + new builder tests)
- [x] T026 Run cargo build --release; verify doctor diagnose on mumu produces meaningful output
- [x] T027 [P] Performance check: run doctor diagnose on 100+ bean project; assert duration_ms < 10000 per SC-M01 in specs/002-model-builder-java-parser/quickstart.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (US1)**: No deps — can start immediately. BLOCKS Phase 3.
- **Phase 2 (US2)**: No deps on US1 — can run in parallel with US1. BLOCKS Phase 3.
- **Phase 3 (US3)**: Depends on Phase 1 AND Phase 2 completion.
- **Phase 4 (Polish)**: Depends on Phase 3.

### Within Phase 1 (US1)

- T001 → T002 → T003 (sequential: scanner → stereotypes → @Bean methods)
- T004, T005, T006 can run in parallel after T002 (all injection detection, different regex patterns)
- T007 depends on T002 (needs class names to resolve)
- T008 can run in parallel with T004-T007
- T009 depends on T001-T008
- T010 depends on T009 + T020 (needs pipeline integration)

### Within Phase 2 (US2)

- T011, T012 are [P] parallel (different file formats)
- T013 depends on T011 + T012
- T014, T015 are [P] parallel
- T016 depends on T015
- T017, T018 are [P] parallel after respective implementations

### Parallel Opportunities

- **US1 + US2 can run entirely in parallel** (different files: bean_graph.rs vs config.rs/auto_config.rs)
- Within US1: T004, T005, T006, T008 can run in parallel
- Within US2: T011||T012, T017||T018

---

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1 (T001-T010) → Bean graph works
2. Verify: `doctor diagnose` on mumu shows beans detected

### Full Feature

1. US1 (Bean graph) + US2 (Config + AutoConfig) in parallel
2. US3 (Pipeline integration) after US1+US2
3. Polish

---

## Notes

- Zero new dependencies — all parsing uses existing `regex`, `serde_yaml`, `walkdir`
- For multi-module projects, module list comes from `settings.gradle.kts` (already parsed in Gradle scanner)
- Java regex patterns MUST handle both `@Annotation` and `@Annotation(value = "x")` forms
- Error handling: skip malformed files with warning, never crash
