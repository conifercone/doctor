# Feature Specification: Doctor 诊断引擎核心（MVP）

**Feature Branch**: `001-diagnostic-engine-core`

**Created**: 2026-07-04

**Status**: Draft

**Input**: User description: "Doctor 产品需求文档（PRD）— 面向开发者的软件系统智能诊断引擎，MVP 阶段聚焦 Spring Boot 诊断能力"

## Clarifications

### Session 2026-07-04

- Q: 发送给外部 LLM 服务进行 AI 解释时，应如何处理诊断数据中的潜在敏感信息？ → A: 结构化摘要过滤——只发送问题的结构化摘要（问题类型、涉及的类名/Bean 名、错误描述）和证据摘要，不发送源码全文和配置具体值。
- Q: Doctor 应如何发现和加载插件？ → A: 目录扫描 + 显式启用——系统自动扫描默认插件目录发现可用插件，用户通过 `--plugin <name>` 或配置文件显式启用，未启用的插件不加载。
- Q: 用户或插件开发者应如何添加自定义诊断规则？ → A: 插件内嵌规则——规则以代码形式作为插件的一部分，每个插件通过实现 Rule Provider 接口提供诊断规则，安装插件即获得对应规则。
- Q: Doctor 核心诊断引擎在完全无网络的环境中应如何工作？ → A: 核心完全离线——诊断引擎（Scanner、Model Builder、Evidence Engine、Rule Engine）完全本地运行，不依赖任何网络连接。AI 解释在有网络时可用，无网络时自动跳过并提示。
- Q: 健康评分应如何计算？ → A: 按严重程度加权扣分——满分 100，每个 ERROR 扣 10 分，WARNING 扣 3 分，INFO 扣 1 分，最低 0 分。

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Spring Boot 项目健康诊断 (Priority: P1)

作为一名 Java/Spring Boot 开发者，我希望在项目目录中运行 `doctor diagnose` 命令，自动获得当前项目的系统健康诊断报告，包括 Bean 注入状态、自动配置状态、配置问题和事务问题，以便快速了解系统是否存在潜在风险。

**Why this priority**: 这是 Doctor 最核心的价值主张——一键式自动诊断。没有这个能力，Doctor 就没有存在的意义。该场景覆盖了 MVP 最关键的诊断能力。

**Independent Test**: 在一个有已知配置问题的 Spring Boot 示例项目中运行 `doctor diagnose`，验证输出报告包含 Bean 诊断、自动配置诊断、配置诊断和事务诊断四个维度的结果。

**Acceptance Scenarios**:

1. **Given** 一个正常的 Spring Boot 项目目录，**When** 用户运行 `doctor diagnose`，**Then** 系统自动扫描项目、建立模型、收集证据、执行诊断规则，并输出包含健康评分和问题列表的诊断报告。
2. **Given** 一个存在 Bean 循环依赖的 Spring Boot 项目，**When** 用户运行 `doctor diagnose`，**Then** 诊断报告明确指出循环依赖的 Bean 名称、依赖链路径，并给出严重等级和修复建议。
3. **Given** 一个存在自动配置冲突的项目，**When** 用户运行 `doctor diagnose`，**Then** 诊断报告列出冲突的自动配置类、冲突原因及证据来源。
4. **Given** 一个存在事务失效的方法（如非 public 方法上的 @Transactional），**When** 用户运行 `doctor diagnose`，**Then** 诊断报告指出事务失效的方法、失效原因及证据。

---

### User Story 2 - AI 解释诊断结果 (Priority: P2)

作为开发者，我希望在获得诊断结果后，能够使用 AI 对诊断结果进行自然语言解释，获得问题的根因分析、影响范围和具体修复建议，以便更快理解并解决问题。

**Why this priority**: AI 解释是 Doctor 区别于传统静态分析工具的关键差异化能力。它依赖于 P1 的诊断结果，但本身不负责诊断，只负责解释。在诊断引擎核心完成后，这是最有价值的增强能力。

**Independent Test**: 针对一份包含 3 个诊断问题的 JSON 报告，运行 `doctor explain`，验证 AI 输出包含每个问题的自然语言解释、根因分析、影响分析和修复建议。

**Acceptance Scenarios**:

1. **Given** 一份已生成的诊断报告（含 Bean 缺失问题），**When** 用户运行 `doctor explain`，**Then** AI 输出该问题的自然语言描述，说明为何该 Bean 缺失、可能的影响范围，并提供具体的修复步骤。
2. **Given** 一份包含多个诊断问题的报告，**When** 用户运行 `doctor explain`，**Then** AI 按照严重程度排序解释所有问题，每个问题均包含证据引用。
3. **Given** AI 解释过程中，**When** 诊断结果证据不足，**Then** AI 明确标注"证据不足"而非猜测，并建议用户收集哪些额外信息。

---

### User Story 3 - 诊断报告输出 (Priority: P3)

作为开发者或 CI/CD 系统，我希望诊断结果能够以多种格式输出（终端友好格式、JSON、Markdown、HTML、SARIF），以便在不同场景下使用：终端查看、CI 集成、文档归档、安全扫描集成。

**Why this priority**: 多格式输出是 Doctor 作为平台化工具的基础能力。终端输出满足开发者在本地使用，JSON 满足 CI/CD 集成，SARIF 满足安全工具链集成。在核心诊断能力完成后，输出格式化是自然扩展。

**Independent Test**: 对同一个 Spring Boot 项目运行诊断，分别指定 `--output json`、`--output markdown`、`--output html`、`--output sarif`，验证每种格式的文件结构正确且内容完整。

**Acceptance Scenarios**:

1. **Given** 诊断完成，**When** 用户指定 `--output json`，**Then** 系统输出结构化 JSON 文件，包含完整的诊断结果、证据列表和元数据。
2. **Given** 诊断完成，**When** 用户指定 `--output markdown`，**Then** 系统输出 Markdown 格式报告，适合直接粘贴到 issue 或文档中。
3. **Given** 在 GitHub Actions 中集成 Doctor，**When** CI 流程运行诊断并指定 `--output sarif`，**Then** 输出 SARIF 格式文件，可被 GitHub Code Scanning 直接识别和展示。
4. **Given** 用户未指定输出格式，**When** 运行诊断命令，**Then** 默认以终端友好的彩色格式直接输出到标准输出。

---

### Edge Cases

- 项目目录中没有任何 Spring Boot 相关文件时，系统应给出明确提示而非崩溃或输出空报告。
- 项目 `pom.xml` 或 `build.gradle` 格式错误、无法解析时，系统应报告解析失败的具体原因和位置。
- 运行时信息（如 Actuator endpoint）不可达时，系统应标注该证据来源为"不可用"并继续基于静态信息完成诊断。
- 用户在没有安装 Java/Maven/Gradle 的环境中运行 Doctor 时，系统应给出依赖缺失提示而非静默失败。
- 诊断过程中项目文件被外部修改时，系统应保证不会崩溃，并基于当前快照完成诊断。
- 超大项目（500+ Bean，100+ 自动配置类）下，诊断应在 5 分钟内完成，且 MUST NOT 无限阻塞。诊断过程 MUST 显示阶段性进度（如 "Scanning... → Building model... → Collecting evidence... → Running rules..."），使用户了解当前进度。
- 配置文件包含敏感信息（密码、密钥等）时，输出报告应自动脱敏处理。
- 在完全无网络的环境中运行时，核心诊断功能 MUST 正常工作（不崩溃、不挂起等待超时），AI 解释功能应给出明确提示说明不可用。

## Requirements *(mandatory)*

### Functional Requirements

**系统扫描（Scanner）**

- **FR-001**: 系统 MUST 自动识别当前目录是否为 Maven 或 Gradle 项目。
- **FR-002**: 系统 MUST 识别项目中的 Spring Boot 版本和主要依赖（starter）。
- **FR-003**: 系统 MUST 输出统一格式的系统描述（技术栈、模块列表、主要依赖）。
- **FR-004**: 当项目无法识别时，系统 MUST 给出明确的错误信息和可能原因。

**系统建模（Model Builder）**

- **FR-005**: 系统 MUST 构建 Bean 依赖图，包含所有声明的 Bean 及其依赖关系。
- **FR-006**: 系统 MUST 构建自动配置模型，包含生效和未生效的自动配置类及其条件。
- **FR-007**: 系统 MUST 构建配置模型，包含所有配置属性及其来源（application.yml、环境变量、命令行参数等）。
- **FR-008**: 所有模型 MUST 使用统一数据结构，支持插件扩展。
- **FR-009**: 系统 MUST 自动扫描默认插件目录（如 `~/.doctor/plugins/`）发现所有已安装的插件。
- **FR-010**: 插件 MUST 通过 `--plugin <name>` 参数或配置文件显式启用后才会被加载和执行；未启用的插件不参与诊断流程。

**证据收集（Evidence Engine）**

- **FR-011**: 系统 MUST 收集源码信息（注解、类结构、方法签名）。
- **FR-012**: 系统 MUST 收集配置文件内容（application.yml、application.properties 等）。
- **FR-013**: 系统 SHOULD 在可访问时收集运行时信息（Actuator health、env、beans、conditions、configprops 等 endpoints）。
- **FR-014**: 每条证据 MUST 保留来源标识，可追溯到具体文件和行号（或运行时 endpoint URL）。

**规则诊断（Rule Engine）**

- **FR-015**: 系统 MUST 实现 Bean 缺失检测规则——当 Bean A 依赖 Bean B 但 Bean B 未被任何配置声明时，报告 Bean 缺失问题。
- **FR-016**: 系统 MUST 实现 Bean 冲突检测规则——当同一类型存在多个 Bean 且注入点未使用 @Qualifier 时，报告 Bean 冲突问题。
- **FR-017**: 系统 MUST 实现循环依赖检测规则——当 Bean 之间形成依赖环时，报告循环依赖及依赖链。
- **FR-018**: 系统 MUST 实现自动配置失败检测规则——当自动配置类因条件不满足而未生效且可能影响预期功能时，报告自动配置问题。
- **FR-019**: 系统 MUST 实现配置冲突检测规则——当同一配置属性被多个来源定义且值不一致时，报告配置冲突。
- **FR-020**: 系统 MUST 实现事务失效检测规则——当 @Transactional 注解使用在非 public 方法、自调用、非受检异常等场景时，报告事务失效风险。
- **FR-021**: 系统 MUST 实现条件装配失败检测规则——当 @ConditionalOnXxx 条件未满足导致预期 Bean 未创建时，报告条件装配问题。
- **FR-022**: 系统 MUST 实现启动分析规则——分析 Spring Boot 启动过程，识别耗时最长的 Bean 初始化和自动配置步骤。
- **FR-023**: 每个诊断结果 MUST 包含：问题标题、严重等级（ERROR/WARNING/INFO）、问题描述、证据列表、修复建议。
- **FR-024**: 健康评分 MUST 按严重程度加权扣分计算：满分 100，每个 ERROR 扣 10 分，每个 WARNING 扣 3 分，每个 INFO 扣 1 分，最低 0 分。
- **FR-025**: 诊断规则 MUST 通过插件的 Rule Provider 接口提供。内置 Spring Boot 规则作为内置插件实现。第三方插件通过实现 Rule Provider 接口添加自定义规则，安装插件即获得对应规则。

**AI 解释（AI Explain）**

- **FR-026**: 系统 MUST 基于诊断结果（而非原始代码）生成 AI 解释。
- **FR-027**: AI 解释 MUST 包含：问题自然语言描述、根因分析、影响范围、修复步骤建议。
- **FR-028**: 当证据不充分时，AI MUST 明确标注不确定性，不得编造原因。
- **FR-029**: AI 解释 SHOULD 引用具体的诊断证据编号，确保可追溯。
- **FR-030**: 发送给外部 LLM 的数据 MUST 限定为结构化摘要——仅包含问题类型、涉及的类名/Bean 名、错误描述和证据摘要，MUST NOT 包含源码全文和配置具体值。
- **FR-031**: 核心诊断引擎（Scanner、Model Builder、Evidence Engine、Rule Engine）MUST 完全本地运行，不依赖任何网络连接。AI 解释功能在有网络时可用，无网络时 MUST 优雅降级并明确提示用户。

**CLI 接口**

- **FR-032**: 系统 MUST 提供 `doctor diagnose` 命令，执行完整诊断流程并输出报告。
- **FR-033**: 系统 MUST 提供 `doctor explain` 命令，对已有诊断结果进行 AI 解释。
- **FR-034**: CLI MUST 支持 `--output` 参数指定输出格式（terminal、json、markdown、html、sarif）。
- **FR-035**: CLI MUST 支持 `--plugin` 参数指定启用的插件名称。系统自动扫描默认插件目录发现可用插件，仅加载显式启用的插件。
- **FR-036**: CLI MUST 支持 `--help` 显示完整帮助信息。

**输出格式化**

- **FR-037**: 系统 MUST 支持终端彩色输出（默认），包含健康评分和问题列表。
- **FR-038**: 系统 MUST 支持 JSON 结构化输出，包含完整诊断数据和元数据。
- **FR-039**: 系统 MUST 支持 SARIF 格式输出，兼容 GitHub Code Scanning 等安全扫描工具。
- **FR-040**: 系统 SHOULD 在输出中对敏感信息（密码、密钥、Token）进行脱敏处理。

### Key Entities

- **DiagnosticReport（诊断报告）**: 一次诊断的完整输出。包含：系统概览、健康评分（0-100，满分 100 起扣，每个 ERROR -10、WARNING -3、INFO -1，最低 0）、问题列表（按严重程度排序）、诊断执行摘要（耗时、覆盖的规则数、证据数）。
- **Issue（诊断问题）**: 单个诊断发现的问题。包含：唯一标识、标题、严重等级（ERROR/WARNING/INFO）、分类（Bean/Config/Transaction/AutoConfig/Startup）、详细描述、证据列表、修复建议、置信度评分。
- **Evidence（证据）**: 支撑诊断结论的单个事实。包含：证据类型（源码/配置/运行时）、来源（文件路径:行号 或 endpoint URL）、内容摘要、可信度（CONFIRMED/INFERRED/UNVERIFIED）。
- **SystemModel（系统模型）**: 对目标系统的结构化描述。包含：技术栈信息、Bean 依赖图、自动配置模型、配置属性模型。各子模型可独立查询。
- **Plugin（插件）**: 为特定技术栈提供诊断能力的扩展模块。包含：Scanner、Model Builder、Evidence Collector、Rule Provider 四个组件（均通过 trait 接口定义）。插件放置于默认插件目录（如 `~/.doctor/plugins/`），由系统自动扫描发现，通过 `--plugin <name>` 或配置文件显式启用。诊断规则通过实现 Rule Provider trait 添加。

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 开发者对包含 50 个 Bean 的标准 Spring Boot 项目执行 `doctor diagnose`，能在 30 秒内获得完整诊断报告。
- **SC-002**: 诊断引擎对已知问题场景（Bean 缺失、循环依赖、事务失效、自动配置失败、配置冲突）的检出率达到 100%（不漏报）。
- **SC-003**: 诊断报告中的每个问题至少附带 1 条可追溯的证据，证据追溯成功率达到 100%。
- **SC-004**: 开发者首次使用 Doctor 诊断一个陌生项目时，能在 5 分钟内理解系统的主要问题（无需额外查阅代码或文档）。
- **SC-005**: AI 解释的修复建议对简单问题（如 Bean 缺失、配置错误）的准确率达到 80% 以上（开发者无需额外搜索即可直接应用）。*注：此指标通过发布后用户反馈收集评估，非构建期自动化验证项。*
- **SC-006**: SARIF 格式输出可被 GitHub Code Scanning 正确解析并展示，零手动调整。
- **SC-007**: 诊断结果的可信度——开发者对 Doctor 诊断结论的信任度（通过用户反馈评估）达到 85% 以上。
- **SC-008**: 在 CI/CD 环境中集成 Doctor 后，每次构建的全流程诊断开销不超过 60 秒。

## Assumptions

- MVP 阶段的诊断目标为 Spring Boot 2.x 和 3.x 项目，其他 Java 框架（Quarkus、Micronaut）不在 MVP 范围内。
- 源码分析基于静态分析，不需要编译项目即可执行（分析 pom.xml/build.gradle + Java 源码 + 配置文件）。
- 运行时信息收集为可选增强——当 Actuator endpoint 可访问时能获取更准确的诊断结果，但静态分析是基础能力。
- AI 解释依赖外部 LLM 服务（通过 API 调用），Doctor 本身不内置 AI 模型。在没有 AI 服务可用时，诊断功能仍可独立运行。发送给 LLM 的数据仅包含结构化摘要（问题类型、类名/Bean 名、错误描述、证据摘要），不含源码全文和配置具体值。
- 核心诊断引擎完全本地运行，不依赖任何网络连接。在离线环境中，所有诊断功能正常工作，仅 AI 解释功能不可用（需网络访问外部 LLM）。
- 项目使用 Maven 或 Gradle 作为构建工具——这是 Spring Boot 项目的行业标准。
- 配置文件格式支持 YAML（.yml/.yaml）和 Properties 两种 Spring Boot 标准格式。
- 目标用户具备基本的 Spring Boot 知识，能理解 Bean、自动配置、事务等概念。
- Doctor 以 CLI 工具形式分发，用户通过命令行交互。IDE 插件、Web UI 为后续扩展。
