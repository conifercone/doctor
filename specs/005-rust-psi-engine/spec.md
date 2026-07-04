# Feature Specification: Rust PSI 引擎 — tree-sitter 替代正则

**Feature Branch**: `005-rust-psi-engine`

**Created**: 2026-07-04

**Status**: Draft

**Input**: 用 tree-sitter 替代正则表达式，建立 CST→AST→PSI 三层 Java 解析架构

## Clarifications

### Session 2026-07-04

- Q: PSI 索引用什么格式持久化？→ A: sled 嵌入式 KV 数据库——支持增量更新（单文件变更只更新对应 key），无需全量重写，长远比 JSON 更高效。

## Problem Statement

当前 Doctor 的 Java 源码解析依赖正则表达式（`src/model/bean_graph.rs` 中约 15 个正则模式）。这种方案存在系统性缺陷：

1. **语法脆弱**：正则无法正确处理嵌套注解、多行声明、泛型参数、注释中误匹配
2. **无跨文件语义**：每个文件独立解析，无法建立"字段类型→类定义"的跨文件引用链
3. **无增量更新**：修改一个文件需要全量重扫描所有文件
4. **注解语义缺失**：无法区分 `@Component` 元注解派生链（`@Service` → `@Component`）
5. **Jar 内 class 解析与源码解析是两套独立机制**，无法统一索引

## Solution: CST→AST→PSI 三层架构

替换范围为 `src/model/bean_graph.rs` 的整个 Java 解析逻辑，以 `tree-sitter` + `tree-sitter-java` 替代正则。选择 tree-sitter 的理由：

- **稳定成熟**：GitHub 维护，数百万用户，不会消失
- **Java grammar 经过实战检验**：800+ 规则覆盖 Java 8-21 全部语法
- **增量解析**：只重解析修改的文件子树
- **唯一成本是 C 编译器**：macOS/Linux 默认自带，可接受

### Assumptions 补充

- tree-sitter 需要系统 C 编译器（macOS: Xcode CLI，Linux: gcc/build-essential）。CI 暂不配置，先聚焦功能实现。
- tree-sitter-java grammar 作为编译时依赖嵌入二进制。

## User Scenarios & Testing *(mandatory)*

### User Story 1 — 精确的 Bean 定义提取 (Priority: P1)

用 tree-sitter CST 替代正则匹配，提取所有 Bean 定义。不再遗漏跨行注解、不再误匹配注释中的伪注解。

**Independent Test**: 对包含跨行 `@Autowired` 和 `// @Component` 注释的 Java 文件运行扫描，验证跨行注入被正确识别、注释中的 `@Component` 不被误判为 Bean。

**Acceptance Scenarios**:
1. **Given** `@Autowired\nprivate UserService userService;`，**When** 扫描源码，**Then** 正确识别跨行 @Autowired 注入。
2. **Given** `// @Component is not a real component` 出现在注释中，**When** 扫描源码，**Then** 不报告该行为 Bean。
3. **Given** `@Service("customName")` 显式指定 Bean 名称，**When** 扫描，**Then** Bean 名称为 `customName` 而非类名推导值。

---

### User Story 2 — 全局符号索引与跨文件引用 (Priority: P2)

建立全项目 FQCN→PSI 节点索引，支持从字段类型名直接定位被依赖的类定义。消除当前"字段类型→Bean 名称"的模糊匹配。

**Independent Test**: 对包含 `import com.example.service.UserService; @Autowired UserService service;` 的项目运行扫描，验证依赖边 `UserService` 能通过 import 解析找到完整类名 `com.example.service.UserService`，并在全局索引中定位该类。

**Acceptance Scenarios**:
1. **Given** A 类依赖 `com.example.FooService`，B 类在另一个文件/模块中定义了 `@Service FooService`，**When** 构建 Bean 图，**Then** 依赖边正确连接 A→B。
2. **Given** 不同包中存在同名类 `com.a.Utils` 和 `com.b.Utils`，**When** 通过 import 解析注入类型，**Then** 根据文件自身的 import 声明匹配到正确的 FQCN。

---

### User Story 3 — 增量扫描 (Priority: P3)

仅重新扫描修改过的文件及其依赖方，显著缩小二次扫描范围。

**Independent Test**: 修改一个 Controller 文件后运行第二次扫描，验证仅该 Controller 和直接依赖它的文件被重新解析，其他文件复用缓存。

**Acceptance Scenarios**:
1. **Given** 项目有 500+ Java 文件，仅修改 1 个文件，**When** 增量扫描，**Then** 重新解析文件数 ≤5（修改文件 + 直接依赖方）。
2. **Given** 首次全量扫描后的 AST 索引已缓存到磁盘，**When** 增量扫描，**Then** 扫描耗时 < 首次的 10%。

---

### Edge Cases

- tree-sitter-java 不支持的 Java 语法（如 Java 22+ 新特性）降级为跳过该文件并 warning。
- 超大文件（>10K 行）的 CST 解析不阻塞——设置超时，超时文件标记为"跳过"。
- Gradle 多模块项目的跨模块引用：通过模块 build.gradle 依赖关系确定模块间可见性。
- 注解继承链（`@Service` → `@Component`）需要硬编码或通过 Spring 元注解规则推断。
- 泛型参数 `List<FooService>` 中的 Bean 注入——仅识别原始类型 FooService，忽略泛型包裹。

## Requirements *(mandatory)*

### CST 层

- **FR-P01**: 系统 MUST 使用 tree-sitter-java 解析所有 `.java` 文件，产出结构化 CST 树（带 span）。
- **FR-P02**: CST 解析 MUST 保留以下节点类型：package_declaration、import_declaration、class_declaration、interface_declaration、annotation_type_declaration、field_declaration、method_declaration、annotation、modifiers。

### AST 层

- **FR-P03**: 系统 MUST 将每个文件的 CST 转换为精简 AST：提取包名、类名（含 FQCN）、接口列表（implements）、所有注解（含全限定名和属性键值对）、字段（含类型和注解）、方法（含返回类型、参数、注解）。
- **FR-P04**: AST MUST 为每个节点保留源码 span（文件路径 + 起止行列），用于诊断报告中引用源码位置。

### PSI 语义层

- **FR-P05**: 系统 MUST 构建全局符号索引：`HashMap<FQCN, PsiClass>`，包含该类的所有 AST 信息 + Bean 标记 + 依赖列表。
- **FR-P06**: 系统 MUST 实现注解语义解析：从注解名解析全限定名（通过 import），提取注解属性键值对（如 `@Service("name")`）。
- **FR-P07**: 系统 MUST 识别 @Component 元注解派生链：`@Service`、`@Repository`、`@Controller`、`@RestController`、`@Configuration` 均视为 @Component Bean。

### Bean 图生成

- **FR-P08**: 系统 MUST 遍历全局 PSI 索引，收集所有标记为 Bean 的类 → 构建 BeanGraph（复用现有数据结构）。
- **FR-P09**: 系统 MUST 从注入点（字段 @Autowired、构造参数、@Bean 方法参数）解析依赖 → 查全局索引 → 生成依赖边 BeanDep。

### 增量 & 缓存

- **FR-P10**: 系统 MUST 支持增量扫描：通过文件修改时间戳或内容 hash 判断是否需重新解析。
- **FR-P11**: 全局 PSI 索引 MUST 持久化到磁盘（sled 嵌入式 KV 数据库），支持增量更新——文件变更时仅更新对应的 key，无需全量重写索引文件。下次启动从 sled 加载，避免全量重解析。

## Success Criteria *(mandatory)*

- **SC-P01**: 对 mumu 项目（~500 Java 文件）的全量 tree-sitter 解析在 30s 内完成。
- **SC-P02**: Bean 定义检出率 ≥98%（当前正则方案约 95%），跨行注解和注释中的伪注解不再产生假阳性/假阴性。
- **SC-P03**: 跨文件 import 解析准确率 ≥99%——同一简单名在不同包中被正确区分。
- **SC-P04**: 增量扫描耗时 < 全量扫描的 10%（修改 1 个文件时）。
- **SC-P05**: 全局索引从磁盘加载耗时 ≤2s。

## Assumptions

- tree-sitter-java 作为编译时依赖链接到 Rust 二进制中。
- tree-sitter 的 CST 解析输出为 tree-sitter crate 的标准 Tree 结构，可递归遍历。
- 注解元信息（如 @Service 的 @Component 元注解）在外部 jar 中——采用硬编码映射表处理已知派生注解，非标准自定义派生注解降级为按注解全名匹配。
- 磁盘缓存目录沿用 `~/.doctor/cache/`。sled 数据库存储于 `~/.doctor/cache/psi-index/`，与现有 JSON 缓存共存。
- 当前的正则解析逻辑在 PSI 引擎稳定后逐步移除（Phase 2 切换）。
