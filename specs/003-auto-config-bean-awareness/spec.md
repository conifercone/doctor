# Feature Specification: 自动配置 Bean 感知

**Feature Branch**: `003-auto-config-bean-awareness`

**Created**: 2026-07-04

**Status**: Draft

**Input**: 减少自动配置 Bean 导致的假阳性——识别 @ConfigurationProperties 和已知自动配置提供的 Bean

## Clarifications

### Session 2026-07-04

- Q: 自动配置 Bean 映射表应如何构建？→ A: Classpath jar 扫描 + class 文件常量池解析——扫描所有依赖 jar 中的 AutoConfiguration.imports（含 spring.factories），对发现的自动配置类解析其 .class 常量池提取 @Bean 方法，类似 IDE 的 bean 发现机制。
- Q: 如何处理大量依赖 jar 的扫描性能？→ A: 缓存策略——首次全量扫描所有 jar → 结果缓存到 `~/.doctor/cache/`，后续诊断复用缓存（仅依赖变更时自动重建），保证不遗漏 bean 且后续诊断保持快速。
- Q: Spring 定义 Bean 的方式不止 @Bean 一种？→ A: 扩展为四种方式：(a) @Bean 方法 (b) @Component/@Service/@Repository/@Controller/@Configuration 类级注解 (c) @Import({Xxx.class}) 一级递归展开 (d) @ConfigurationProperties。ImportSelector/BeanDefinitionRegistryPostProcessor 等运行时注册方式暂不支持。

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 减少自动配置 Bean 假阳性 (Priority: P1)

开发者对 Spring Boot 项目运行 `doctor diagnose` 时，由 Spring Boot/Cloud 自动配置提供的 Bean（如 `OAuth2AuthorizationService`、`jsonMapper`、`ObjectProvider`）不应报告为"缺失"。诊断结果只应报告真正缺失的用户自定义 Bean。

**Why this priority**: 72 个 ERROR 中大部分是自动配置 Bean 假阳性，严重影响报告可用性和开发者信任度。

**Independent Test**: 对 mumu 项目运行诊断，验证 `OAuth2AuthorizationService`、`objectProvider`、`jsonMapper` 不再出现在缺失 Bean 列表中。ERROR 数量从 72 下降到 ≤30。

**Acceptance Scenarios**:
1. **Given** mumu 项目依赖 Spring Security OAuth2，**When** 运行诊断，**Then** `OAuth2AuthorizationService` 不再报告为缺失 Bean。
2. **Given** 项目中使用了 @ConfigurationProperties 注解的类，**When** 构建 Bean 图，**Then** 该类被识别为自动配置提供的 Bean（而非用户需定义的 Bean）。
3. **Given** 项目中 @Autowired 了 `ObjectMapper`（Jackson 自动配置提供），**When** 运行诊断，**Then** `ObjectMapper` 不报告为缺失。

---

### User Story 2 - 多模块同名 Bean 去重 (Priority: P2)

多模块项目中每个子模块重复定义的配置类（如 `JacksonConfiguration`、`SwaggerConfiguration`）不应报告为 Bean 冲突——它们是独立模块的独立实例，不存在冲突。

**Why this priority**: 20 个 WARNING 中有 16 个是多模块重复类假阳性。

**Independent Test**: 对 mumu 项目运行诊断，验证 `JacksonConfiguration x4`、`SwaggerConfiguration x4` 等不再出现在 Bean 冲突列表中。WARNING 数量从 20 下降到个位数。

**Acceptance Scenarios**:
1. **Given** 4 个子模块各自定义 `JacksonConfiguration`，**When** 运行诊断，**Then** 这些 Bean 不报告为冲突（因为位于不同模块/包路径中）。
2. **Given** 同一个模块内存在真正冲突的 Bean（同一类型、同一包路径的两个 Bean），**When** 运行诊断，**Then** 仍然报告为冲突。

---

### Edge Cases

- Spring Boot 版本不同（2.x vs 3.x）提供的自动配置 Bean 集合不同，应基于项目实际的 Spring Boot 版本匹配。
- 通过 `@EnableConfigurationProperties` 激活的配置属性类，应作为 Bean 识别。
- `spring.factories` 旧格式中的 `EnableAutoConfiguration` 条目也应解析。
- Java 泛型参数（如 `ObjectProvider<SomeType>`、`Map<String, SecurityFilterChain>`）在依赖解析时正确处理——泛型参数也可能是 Bean。
- `@Import` 递归展开限制一级（最大深度 2），防止 `ImportSelector` 等运行时接口导致的无限循环。
- `ImportSelector`、`ImportBeanDefinitionRegistrar`、`BeanDefinitionRegistryPostProcessor`、`FactoryBean` 等运行时注册方式不在静态分析范围——记录为局限性。
- `@Bean` 注解的 `name`/`value` 属性显式指定 Bean 名称时，优先使用该名称而非方法名。

## Requirements *(mandatory)*

### Functional Requirements

**Classpath 扫描 & Bean 发现**

- **FR-A01**: 系统 MUST 扫描项目所有依赖 jar（Gradle: `~/.gradle/caches/modules-2/files-2.1/`，Maven: `~/.m2/repository/`），查找每个 jar 内 `META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports` 文件（及旧版 `META-INF/spring.factories` 中的 `EnableAutoConfiguration` key），提取所有候选自动配置类名。
- **FR-A02**: 系统 MUST 对 FR-A01 发现的每个自动配置类，在对应 jar 中定位其 `.class` 文件，通过解析 Java class 文件常量池（RuntimeVisibleAnnotations），从以下四种方式提取 Bean 定义：
  - **(a) `@Bean` 方法**: 检测 `Lorg/springframework/context/annotation/Bean;` 注解的方法，提取返回类型描述符和方法名。
  - **(b) `@Component` 及派生注解**: 检测类级别以下注解描述符（任一命中即视为 Bean）：
    - `Lorg/springframework/stereotype/Component;`
    - `Lorg/springframework/stereotype/Service;`
    - `Lorg/springframework/stereotype/Repository;`
    - `Lorg/springframework/stereotype/Controller;`
    - `Lorg/springframework/web/bind/annotation/RestController;`
    - `Lorg/springframework/context/annotation/Configuration;`
    
    类名推导 Bean 名称（首字母小写）。
  - **(c) `@Import({Xxx.class})`**: 检测类级别 `Lorg/springframework/context/annotation/Import;`，提取其 value 数组中的类引用。递归展开一级（被 Import 的类重复 a→c 检测），避免无限递归（最大深度 2）。
  - **(d) `@ConfigurationProperties`**: 检测 `Lorg/springframework/boot/context/properties/ConfigurationProperties;` 注解的类，标记为自动配置 Bean。
- **FR-A03**: 系统 MUST 将 FR-A02 发现的所有 Bean 注册到 Bean 图中，标记来源为 `AutoConfig`。FR-A02d 中发现的 @ConfigurationProperties 类同时标记为 `AutoConfig`。
- **FR-A04**: 系统 MUST 识别用户项目源码中的 `@EnableConfigurationProperties({Xxx.class})` 引用，将参数中列出的类标记为已知 Bean。
- **FR-A05**: 系统 MUST 将全量 jar 扫描结果缓存到 `~/.doctor/cache/auto-config-beans.json`。后续诊断优先读取缓存；当项目的依赖树发生变化（通过对比 `build.gradle.kts` + `libs.versions.toml` 的 hash 值）时自动触发缓存重建。首次扫描允许耗时较长，缓存命中时扫描开销降为近乎零。

**常量池解析技术要求**（补充说明）
- 仅需解析 class 文件常量池中的 UTF8 项、类引用、方法描述符和 RuntimeVisibleAnnotations，不涉及字节码指令级分析。标准库二进制读取即可完成。
- `@Bean` 方法返回类型从方法描述符推导（如 `()Ljavax/sql/DataSource;` → `javax.sql.DataSource`）。Bean 名称默认为方法名，若 `@Bean` 有 `name`/`value` 属性则使用显式名称。
- `@Component` 扫描需显式匹配 6 种注解描述符：Component、Service、Repository、Controller、RestController、Configuration。JVM class 文件中不保留元注解关系，因此必须逐一枚举。
- `@Import` 递归展开仅限一级（最大深度 2：自动配置类 → Imported 类 → Imported 类的 @Bean/@Component）。
- 不支持 `ImportSelector`、`ImportBeanDefinitionRegistrar`、`BeanDefinitionRegistryPostProcessor`、`FactoryBean` 等运行时注册方式——这些需要运行 Spring 容器，静态分析无法覆盖。在局限性文档中记录。
- 若 class 文件不存在或常量池解析失败，跳过该类并记录 warning，不阻塞诊断流程。

**Bean 冲突去重**

- **FR-A06**: Bean 冲突检测 MUST 排除位于不同 Maven/Gradle 子模块（不同 `src/main/java` 根路径）中的同名类——它们被视为独立 Bean 实例而非冲突。
- **FR-A07**: 同一子模块内、不同 Java 包中的同名 Bean 仍然报告为冲突。

**诊断规则改进**

- **FR-A08**: Bean 缺失检测 MUST 在检查依赖时排除已知的自动配置 Bean（已在 Bean 图中标记为 `source: AutoConfig`）。
- **FR-A09**: 系统 MUST 为每个 BeanDef 增加来源标记：`UserDefined`（用户项目源码中发现的 Bean）或 `AutoConfig`（依赖 jar 自动配置提供的 Bean）。该标记影响缺失检测和冲突检测的判定。

## Success Criteria *(mandatory)*

- **SC-A01**: mumu 项目诊断 ERROR 数量从当前 72 降低到 ≤30（减少 ≥58% 假阳性）。
- **SC-A02**: mumu 项目诊断 WARNING 数量从当前 20 降低到 ≤8（减少 ≥60%，主要消除多模块重复类假阳性）。
- **SC-A03**: 自动配置 Bean 来源标注准确率 ≥90%——即 10 个自动配置 Bean 中至少 9 个被正确标记为 `AutoConfig`。
- **SC-A04**: 缓存命中时诊断总耗时 ≤1.0s（接近当前 0.76s，缓存读取开销可忽略）。首次扫描或缓存重建时允许 ≤5s（全量 jar 解析有一次性开销）。

## Assumptions

- Spring Boot 2.x 使用 `META-INF/spring.factories` 中的 `org.springframework.boot.autoconfigure.EnableAutoConfiguration` key；3.x 使用 `META-INF/spring/org.springframework.boot.autoconfigure.AutoConfiguration.imports`。两者均需支持。
- Gradle 缓存路径 `~/.gradle/caches/modules-2/files-2.1/` 是依赖 jar 的确定位置。项目必须已执行过 `gradle build` 或 `gradle resolveDependencies`（依赖已下载）。Maven 本地仓库路径为 `~/.m2/repository/`。
- Class 文件常量池解析仅处理 UTF8_CONSTANT 项和方法描述符，无需引入 ASM 等外部字节码库。无效或损坏的 class 文件跳过并记录 warning。
- 自动配置 Bean 发现失败（jar 不存在、class 解析失败）时优雅降级——不阻塞诊断，继续基于用户源码 Bean 执行规则。
- 缓存 key 由构建文件（`build.gradle.kts` + `settings.gradle.kts` + `gradle/libs.versions.toml`）的 SHA256 hash 生成，构建文件不变则缓存有效。
- 首次扫描全量 jar 可能耗时 3-5 秒，缓存命中后扫描步骤几乎零开销。缓存文件路径：`~/.doctor/cache/auto-config-beans.json`。
- 多模块同名类去重基于文件路径前缀判断，同一 `src/main/java` 下为同模块。
