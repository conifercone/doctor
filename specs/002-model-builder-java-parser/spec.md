# Feature Specification: Java 源码模型构建器

**Feature Branch**: `002-model-builder-java-parser`

**Created**: 2026-07-04

**Status**: Draft

**Input**: 实现 Model Builder，解析 Java 源码让诊断规则真正工作

## Clarifications

### Session 2026-07-04

- Q: Lombok @RequiredArgsConstructor 的启发式规则应如何定义？ → A: 简单匹配——检测到 @RequiredArgsConstructor 或 @AllArgsConstructor 时，将类的所有 private final 字段视为构造器注入依赖。
- Q: 自动配置模型（AutoConfigModel）是否纳入 002 范围？ → A: 纳入——同时构建 AutoConfigModel，解析 AutoConfiguration.imports 文件并匹配 @Conditional 条件注解。

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Bean 依赖图构建 (Priority: P1)

开发者对 Spring Boot 项目运行 `doctor diagnose` 时，系统自动解析所有 Java 源码（含子模块），提取 Bean 定义、依赖注入关系，构建完整的 Bean 依赖图。诊断规则基于该图检测 Bean 缺失、冲突和循环依赖问题。

**Why this priority**: Bean 图是所有后续诊断的基础。没有 Bean 图，FR-015/016/017 三条规则无法工作。

**Independent Test**: 在一个包含 3 个 Bean（Controller → Service → Repository 依赖链）的 Spring Boot 项目中运行诊断，验证输出 Bean 图和依赖关系。

**Acceptance Scenarios**:
1. **Given** 一个含有 `@Service UserService` 和 `@Autowired UserRepository` 的项目，**When** 运行诊断，**Then** Bean 图包含 UserService 和 UserRepository，且存在依赖边 UserService → UserRepository。
2. **Given** 一个多模块 Gradle 项目（如 mumu），**When** 运行诊断，**Then** 系统遍历所有子模块的 `src/main/java` 目录，聚合所有 Bean。
3. **Given** 一个含有 `@Qualifier("paymentService")` 注解的项目，**When** 构建 Bean 图，**Then** 依赖边正确标注 Qualifier 信息。

---

### User Story 2 - 配置模型构建 (Priority: P2)

系统自动解析 `application.yml` / `application.properties` 及环境变量，构建配置属性模型。诊断规则基于该模型检测配置冲突问题。

**Why this priority**: 配置模型是 FR-019（配置冲突检测）的前提。优先级低于 Bean 图但高于事务/启动分析。

**Independent Test**: 在一份多 profile YAML（default + dev + prod override）上运行诊断，验证配置模型包含所有属性及来源优先级。

**Acceptance Scenarios**:
1. **Given** `application.yml` 和 `application-dev.yml` 同时存在，**When** 构建配置模型，**Then** 每个属性标注其所有来源文件及优先级。
2. **Given** 同一属性 `server.port` 在 YAML 和环境变量中定义了不同值，**When** 构建配置模型，**Then** 该属性标记为"冲突"状态。

---

### User Story 3 - 诊断规则端到端产出 (Priority: P3)

Model Builder 完成后，整合进现有 `doctor diagnose` pipeline，诊断规则产出真实的诊断问题（而非空报告）。

**Why this priority**: 验证所有组件正确集成。这是用户能感知到的最终价值。

**Independent Test**: 对一个包含已知 Bean 缺失问题的项目运行 `doctor diagnose`，验证输出至少 1 个诊断问题（非空报告）。

**Acceptance Scenarios**:
1. **Given** `UserService` @Autowired 了不存在的 `UserRepository`，**When** 运行 `doctor diagnose`，**Then** 输出至少 1 个 ERROR 级别 Bean 缺失问题。
2. **Given** 同一个类型有两个 Bean 实例且无 @Qualifier，**When** 运行诊断，**Then** 输出至少 1 个 WARNING 级别 Bean 冲突问题。

---

### Edge Cases

- Java 文件有语法错误时，跳过该文件并记录 warning，不影响其他文件解析。
- 多模块项目中没有 `src/main/java` 目录的模块（如 infra 模块），不应崩溃。
- @Configuration 类中通过 @Bean 方法定义的 Bean，正确提取方法名作为 Bean 名称。
- @ComponentScan 自定义 basePackages 时，在指定包路径下搜索组件。
- Lombok 生成的代码（如 @RequiredArgsConstructor 构造器注入）无法通过源码分析检测——应记录为局限性。
- YAML 多文档格式（`---` 分隔）正确解析为独立配置源。

## Requirements *(mandatory)*

### Functional Requirements

**Bean 图构建**

- **FR-M001**: 系统 MUST 扫描所有 Java 源文件（`.java`），识别以下注解标记的类：`@Component`, `@Service`, `@Repository`, `@Controller`, `@RestController`, `@Configuration`。
- **FR-M002**: 系统 MUST 识别 `@Configuration` 类中通过 `@Bean` 注解方法声明的 Bean，方法名作为 Bean 名称。
- **FR-M003**: 系统 MUST 识别以下注入方式：`@Autowired` 字段注入、`@Autowired` 构造器注入、setter 注入。
- **FR-M004**: 系统 MUST 处理 `@Qualifier` 注解，在依赖边中记录限定符信息。
- **FR-M005**: 系统 MUST 支持多模块项目，遍历所有子模块的 `src/main/java` 目录。
- **FR-M006**: Bean 的类名 MUST 使用 import 解析或简单名匹配推断完整限定类名。

**自动配置模型构建**

- **FR-M007**: 系统 MUST 解析 `META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports` 文件（及旧版 `spring.factories` 中的 `EnableAutoConfiguration` key），获取候选自动配置类列表。
- **FR-M008**: 系统 MUST 通过扫描项目 classpath（Maven/Gradle 依赖中的 jar）或源码注解，判断自动配置类是否在项目中可用，区分 enabled（条件满足）/ disabled（条件不满足）状态。
- **FR-M009**: 对于 disabled 的自动配置类，MUST 记录其失败的 @Conditional 条件及原因。

**配置模型构建**

- **FR-M010**: 系统 MUST 解析所有 `application.yml`、`application.yaml`、`application-{profile}.yml` 文件。
- **FR-M011**: 系统 MUST 解析所有 `application.properties`、`application-{profile}.properties` 文件。
- **FR-M012**: 系统 MUST 为每个配置属性记录来源文件路径和属性名。
- **FR-M013**: 同一属性被多个来源定义且值不同时，MUST 标记为配置冲突。

**Pipeline 集成**

- **FR-M014**: 系统 MUST 在 `doctor diagnose` pipeline 中调用 Model Builder，替换当前的 `Default::default()` stub。
- **FR-M015**: Model Builder 执行时间 MUST NOT 超过总诊断时间的 80%（预留资源给规则执行和输出格式化）。

### Key Entities

复用 001 spec 中定义的 `BeanGraph`、`AutoConfigModel`、`ConfigModel`、`SystemModel` 结构体，无需新增实体。

## Success Criteria *(mandatory)*

- **SC-M01**: 对于包含 100 个 Bean 的标准 Spring Boot 项目，Bean 图构建在 10 秒内完成。
- **SC-M02**: Bean 定义的检出率 ≥95%（即源码中 100 个 Bean 至少识别 95 个）。
- **SC-M03**: 诊断规则在 Model Builder 就绪后产出真实诊断结果——对已知问题项目（Bean 缺失），问题检出率 ≥90%。
- **SC-M04**: 多模块项目（如 mumu 的 9 模块）中，所有模块的 Bean 均被聚合到统一 Bean 图中，无遗漏模块。

## Assumptions

- Java 文件编码为 UTF-8。
- 不考虑通过 XML 配置（`<bean>` 标签）定义的 Bean——现代 Spring Boot 项目已极少使用。
- Lombok 生成的构造器注入：检测 `@RequiredArgsConstructor` 或 `@AllArgsConstructor` 时，将类的所有 `private final` 字段视为构造器注入依赖（简单匹配，不做类型验证）。
- import 解析从同一文件中的 import 语句推断完整类名；未能解析的保持简单类名。
- 配置文件路径遵循 Spring Boot 默认约定（`src/main/resources/`）。
- 仅解析主源码路径（`src/main/`），不包括测试源码（`src/test/`）。
