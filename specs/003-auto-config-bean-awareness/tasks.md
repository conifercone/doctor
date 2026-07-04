# Tasks: 自动配置 Bean 感知

**Input**: Design documents from `specs/003-auto-config-bean-awareness/`

**Prerequisites**: plan.md (required), spec.md (required)

**Tests**: Tests included per Constitution V — class 常量池解析和 jar 扫描必须具备测试。

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)

---

## Phase 1: Setup

- [x] T001 Create classpath module directory: mkdir src/classpath/ and create src/classpath/mod.rs with module declarations

---

## Phase 2: Foundational — Class 文件常量池解析

**Purpose**: 纯标准库解析 .class 二进制格式，无需外部依赖。

- [x] T002 Implement class file reader: read_u16/read_u32 helpers, skip magic+version, parse constant_pool_count, read constant pool entries (tag dispatch: UTF8=1, Class=7, String=8, Utf8=1) in src/classpath/class_parser.rs
- [x] T003 Implement UTF8 constant resolver: build string table from CONSTANT_Utf8 entries; resolve class_index, name_and_type_index through CONSTANT_Class and CONSTANT_NameAndType in src/classpath/class_parser.rs
- [x] T004 Implement method parsing: parse methods_count, for each method: access_flags, name_index, descriptor_index, attributes_count → find RuntimeVisibleAnnotations attribute → parse annotations → detect @Bean descriptor in src/classpath/class_parser.rs
- [x] T005 Implement class-level annotation parsing: parse class access_flags → attributes → RuntimeVisibleAnnotations → detect 6 Component-descriptor annotations (Component, Service, Repository, Controller, RestController, Configuration) + @Import (extract value array) + @ConfigurationProperties in src/classpath/class_parser.rs
- [x] T006 Implement @Bean name resolution: extract @Bean annotation's name/value attribute from annotation element_value pairs; fallback to method name in src/classpath/class_parser.rs
- [x] T007 Implement type descriptor parser: convert JVM descriptors to readable class names (e.g., `()Ljavax/sql/DataSource;` → DataSource, `(Ljava/lang/String;)V` → void for return type) in src/classpath/class_parser.rs

---

## Phase 3: US1 — 自动配置 Bean 假阳性削减 (P1) 🎯 MVP

**Goal**: classpath jar 扫描 → 发现自动配置 Bean → 注册到 Bean 图 → 缺失检测跳过它们。

**Independent Test**: 对 mumu 项目运行诊断，ERROR 从 72 → ≤30，`OAuth2AuthorizationService`/`objectProvider`/`jsonMapper` 不再出现。

### Jar 扫描 + 缓存

- [x] T008 [P] [US1] Implement jar scanner: walk Gradle cache path (`~/.gradle/caches/modules-2/files-2.1/`); for each .jar: open as ZipArchive, check for META-INF/spring/AutoConfiguration.imports (and spring.factories fallback); extract class name list in src/classpath/jar_scanner.rs
- [x] T009 [P] [US1] Implement cache engine: SHA256(build.gradle.kts + settings.gradle.kts + libs.versions.toml) → `~/.doctor/cache/auto-config-beans-{hash}.json`; if cache hit → deserialize JSON; if miss → trigger full scan → serialize results in src/classpath/cache.rs
- [x] T010 [US1] Implement classpath scan orchestration: jar_scanner → for each discovered auto-config class → locate .class in jar → class_parser → extract all Beans (4 discovery methods) → dedup by class_name → return Vec<AutoConfigBean> in src/classpath/mod.rs

### Bean 图集成

- [x] T011 [US1] Add BeanSource enum (UserDefined / AutoConfig) to BeanDef struct; also scan user project for @ConfigurationProperties-annotated classes and @EnableConfigurationProperties references (extract referenced classes) in src/model/bean_graph.rs
- [x] T012 [US1] Integrate classpath scanner into build_bean_graph(): after user beans are built, call classpath scanner → register discovered AutoConfig beans with source=AutoConfig into BeanGraph in src/model/bean_graph.rs

### 规则修改

- [x] T013 [US1] Modify bean missing detection: skip dependency check for targets that exist in BeanGraph as source=AutoConfig (they are provided by framework, not user's responsibility) in src/rule_engine/bean_rules.rs

### Pipeline 集成

- [x] T014 [US1] Wire classpath scan step into diagnose pipeline (before or after user bean graph build); emit cache status (hit/miss) in verbose mode in src/cli/diagnose.rs

### 验证

- [x] T015 [US1] Write unit tests: class_parser with fixture .class files (simple @Configuration + @Bean method); assert extracted beans have correct type and name in src/classpath/class_parser.rs
- [x] T016 [US1] Write unit tests: jar_scanner with test jar containing AutoConfiguration.imports; assert class name list extracted correctly in src/classpath/jar_scanner.rs
- [x] T017 [US1] Run doctor diagnose on mumu and verify: (a) ERROR count ≤30, (b) OAuth2AuthorizationService/ObjectMapper/DataSource NOT in issues, (c) these beans marked as AutoConfig source in BeanGraph (SC-A03)

**Checkpoint**: mumu ERROR count drops from 72 to ≤30.

---

## Phase 4: US2 — 多模块 Bean 去重 (P2)

**Goal**: 不同子模块中的同名 Bean 不再报告为冲突。

**Independent Test**: mumu WARNING 从 20 → ≤8，JacksonConfiguration x4 / SwaggerConfiguration x4 不再出现。

- [x] T018 [US2] Modify bean conflict detection: group beans by type; if all beans of same type originate from different src/main/java root paths → skip conflict report; same-module duplicates still flagged in src/rule_engine/bean_rules.rs
- [x] T019 [US2] Write unit test: fixture with 2 modules each having JacksonConfiguration → assert 0 conflicts; same module with 2 beans of same type → assert 1 conflict in src/rule_engine/bean_rules.rs
- [x] T020 [US2] Run doctor diagnose on mumu and verify: WARNING count ≤8, JacksonConfiguration/SwaggerConfiguration NOT in Bean conflicts

**Checkpoint**: mumu WARNING count drops from 20 to ≤8.

---

## Phase 5: Polish

- [x] T021 Run cargo fmt and cargo clippy; fix all issues
- [x] T022 Run cargo test and verify all tests pass
- [x] T023 Run cargo build --release; full mumu end-to-end: ERROR ≤30, WARNING ≤8, duration ≤1.2s

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No deps
- **Phase 2 (Foundational)**: Depends on Phase 1 — class_parser is prerequisite for everything
- **Phase 3 (US1)**: Depends on Phase 2 (class_parser ready) — can start T008/T009 in parallel
- **Phase 4 (US2)**: Depends on Phase 3 bean graph changes (T011-T012), but rule changes are independent of classpath
- **Phase 5 (Polish)**: Depends on Phase 3+4

### Within Phase 3 (US1)

- T008 (jar_scanner) ∥ T009 (cache) — parallel, different concerns
- T010 (orchestration) depends on T008 + T009 + Phase 2
- T011 (BeanSource enum) ∥ T010 — T011 can start immediately
- T012 (integrate) depends on T010 + T011
- T013 (rule modify) depends on T012
- T014 (pipeline) depends on T012
- T015 ∥ T016 (tests) — parallel after respective implementations
- T017 (mumu validation) depends on T014

### Within Phase 4 (US2)

- T018 depends on T013 (needs updated bean_rules.rs context)
- T019 depends on T018
- T020 depends on T018

### Parallel Opportunities

- **Phase 2**: T002-T007 all in same file — sequential by necessity (build up parser incrementally)
- **Phase 3 T008 ∥ T009**: jar scanner and cache are independent files
- **T011 ∥ T010**: BeanSource enum is simple struct change, separate from orchestration logic

---

## Implementation Strategy

### MVP First (US1 Only)

1. Phase 1 + Phase 2 → class_parser ready
2. Phase 3 (T008-T017) → auto-config bean detection works
3. **STOP**: Validate mumu ERROR ≤30
4. **Ship**: This alone is a major UX improvement

### Full Feature

1. US1 → auto-config beans recognized
2. US2 → multi-module dedup
3. mumu: 72 ERR → ≤30 ERR + 20 WARN → ≤8 WARN

---

## Notes

- Class 常量池解析使用标准库 `std::io::Cursor<&[u8]>` + `read_u16::<BigEndian>()` 等方法，零外部依赖
- Gradle 缓存路径在 macOS 上为 `~/.gradle/caches/modules-2/files-2.1/`，Linux 相同
- Maven 本地仓库路径为 `~/.m2/repository/`，支持作为备选扫描源
- `@Import` 递归最大深度 2（自动配置类 → Imported 类），防止 `ImportSelector` 无限循环
- 不支持 `ImportSelector`、`ImportBeanDefinitionRegistrar`、`BeanDefinitionRegistryPostProcessor`、`FactoryBean` — 记录为局限性
