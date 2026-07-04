# Tasks: Rust PSI 引擎（tree-sitter）

**Input**: Design documents from `specs/005-rust-psi-engine/`

**Prerequisites**: plan.md (required), spec.md (required)

**Tests**: Included — 每个模块必须具备单元测试（Constitution V）。

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

- [x] T001 Add dependencies: tree-sitter = "0.26", tree-sitter-java (grammar), sled = "0.34" to Cargo.toml
- [x] T002 Create src/psi/ directory and src/psi/mod.rs with module declarations for cst, ast, index, bean_collector
- [x] T003 Register psi module in src/lib.rs and src/main.rs

---

## Phase 2: Foundational — CST 解析器

- [x] T004 Implement parse_java_file: tree-sitter Parser + tree-sitter-java LANGUAGE → parse source to Tree in src/psi/cst.rs
- [x] T005 Implement CST node traversal helper: recursive walk with node kind filter (class_declaration, field_declaration, method_declaration, annotation, import_declaration, package_declaration) in src/psi/cst.rs
- [x] T006 Write unit test: parse a simple @Service class with tree-sitter, verify CST contains expected node kinds in src/psi/cst.rs

---

## Phase 3: US1 — AST 层 + Bean 收集 (P1) 🎯 MVP

**Goal**: tree-sitter 提取 Bean 定义，替代正则。精确识别跨行注解、不误匹配注释中的伪注解。

**Independent Test**: 对包含跨行 @Autowired 和 // @Component 注释的文件扫描，验证跨行注入被识别、注释不误判。

### AST 层

- [x] T007 [US1] Implement package + import extraction: from CST tree, extract package name and build HashMap<simple_name, FQCN> in src/psi/ast.rs
- [x] T008 [US1] Implement Class extraction: class_declaration → PsiClass { name, fqcn, interfaces, annotations, fields, methods } in src/psi/ast.rs
- [x] T009 [US1] Implement Field extraction: field_declaration → PsiField { name, type_name, annotations } with @Autowired/@Qualifier detection in src/psi/ast.rs
- [x] T010 [US1] Implement Method extraction: method_declaration → PsiMethod { name, return_type, parameters, annotations } with @Bean detection in src/psi/ast.rs
- [x] T011 [US1] Implement Annotation parsing: extract annotation FQCN (via import resolution), attribute key-value pairs (e.g., @Service("name") → name="name") in src/psi/ast.rs

### Bean 收集

- [x] T012 [US1] Implement BeanCollector: traverse all PsiClass in project, identify 4 bean types (stereotype, @Bean, @Import, auto-config scan), assign unique beanId + beanName in src/psi/bean_collector.rs
- [x] T013 [US1] Implement annotation inheritance: @Service/@Repository/@Controller/@RestController/@Configuration all recognized as @Component-derived bean annotations in src/psi/bean_collector.rs
- [x] T014 [US1] Implement dependency edge building: from @Autowired fields + constructor params + @Bean method params → query PsiClass index → build BeanDep edges in src/psi/bean_collector.rs

### 替换正则

- [x] T015 [US1] Modify build_bean_graph() in src/model/bean_graph.rs: replace regex-based parsing with call to psi::bean_collector::collect_beans() in src/model/bean_graph.rs
- [x] T016 [US1] Verify regression: run doctor diagnose on mumu, confirm Bean count >= current (287 user beans) and ERROR count <= current (12) in specs/005-rust-psi-engine/quickstart.md

---

## Phase 4: US2 — 全局 PSI 索引 (P2)

**Goal**: sled KV 数据库持久化全局符号索引，支持 FQCN→PsiClass 查询和 import 跨文件解析。

**Independent Test**: 对 mumu 项目建立索引，验证按 FQCN 查询返回正确的 PsiClass 节点。

### 索引构建

- [x] T017 [US2] Implement sled index store: open/create sled DB at ~/.doctor/cache/psi-index/, define key schemas (CLASS:{fqcn}, IMPORT:{simple_name}) in src/psi/index.rs
- [x] T018 [US2] Implement index writer: after AST parsing, write PsiClass entries to sled, populate CLASS:* and IMPORT:* keys in src/psi/index.rs
- [x] T019 [US2] Implement index reader: query by FQCN returns PsiClass, query by simple_name returns Vec<FQCN> (for import resolution) in src/psi/index.rs
- [x] T020 [US2] Implement cross-file reference resolution: field type_name → import resolution → FQCN → sled lookup → target PsiClass in src/psi/index.rs

### 验证

- [x] T021 [US2] Write integration test: parse 2 Java files with cross-file dependency, verify sled index resolves dependency correctly in src/psi/index.rs
- [x] T022 [US2] Run doctor diagnose on mumu, verify cross-module import resolution works (same simple name in different packages resolved correctly)

---

## Phase 5: US3 — 增量扫描 (P3)

**Goal**: 仅重新解析修改过的文件；其他文件复用 sled 索引缓存。

**Independent Test**: 修改 1 个文件后重新扫描，验证重新解析文件数 ≤5。

- [x] T023 [US3] Implement file hash tracker: SHA256 hash each .java file → store in sled under FILE_HASH:{path} key; detect changed files by comparing hashes in src/psi/cst.rs
- [x] T024 [US3] Implement incremental reparse: only re-parse files with changed hashes; update their PsiClass entries in sled; recompute affected dependency edges in src/psi/index.rs
- [x] T025 [US3] Verify: modify 1 Controller in mumu, re-run diagnose, confirm re-parsed files ≤5 and total time < 2s in specs/005-rust-psi-engine/quickstart.md

---

## Phase 6: Polish

- [x] T026 Run cargo fmt and cargo clippy; fix all issues (except tree-sitter C code warnings)
- [x] T027 Run cargo test and verify all tests pass
- [x] T028 Run cargo build --release; end-to-end on mumu: ERROR <= 12, duration < 30s (SC-P01)

---

## Dependencies & Execution Order

- **Phase 1→2→3**: 严格顺序（Setup → CST → AST → Bean）
- **Phase 4 (US2)**: 可与 Phase 3 并行（index.rs 独立于 bean_collector.rs）
- **Phase 5 (US3)**: 依赖 Phase 4（增量需 sled 索引）
- **Phase 6**: 依赖全部

### Parallel Opportunities

- **Phase 3 T007-T011** 与 **Phase 4 T017-T019** 可并行（不同文件）
- T023 与 T024 可并行（hash tracker 与增量重解析独立）

---

## MVP Scope

仅 **Phase 1+2+3**（16 tasks）即可实现 tree-sitter 替代正则，Bean 检出精度从 ~95% 提升到 ~99%。

## Notes

- tree-sitter 依赖 C 编译器（macOS 自带，Linux 需 build-essential）
- sled 数据库写入 ~/.doctor/cache/psi-index/，与现有 JSON 缓存共存
- 旧正则逻辑保留在 bean_graph.rs 中，通过 `#[cfg(feature = "regex-parser")]` feature gate 切换
