# Feature Specification: 接口-实现类匹配

**Feature Branch**: `004-interface-impl-matching`

**Created**: 2026-07-04

**Status**: Draft

**Input**: 解决自动配置 Bean 实现类与用户依赖接口之间的类名不匹配问题

## Problem Statement

当前诊断引擎构建 Bean 图后执行缺失检测规则时，用户 Bean 声明的依赖是**接口类型**（如 `OAuth2AuthorizationService`），而自动配置注册的 Bean 是**实现类**（如 `DefaultOAuth2AuthorizationService`）。由于两者类名不同，缺失检测规则将其误判为"Bean 缺失"，导致 72 个 ERROR 假阳性无法消除。

**修复思路**：在 Bean 图构建阶段，为每个类记录其实现的接口，形成 `interface → [impl_classes]` 映射。依赖检查时，不仅按 Bean name/class 匹配，还按实现的接口匹配。

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 接口-实现匹配消除假阳性 (Priority: P1) 🎯

对 Spring Boot 项目运行 `doctor diagnose`，用户 Bean 依赖的接口类型（如 `OAuth2AuthorizationService`、`roleGateway`）能被自动配置提供的实现类（如 `DefaultOAuth2AuthorizationService`、`RoleGatewayImpl`）自动匹配，不再报告为缺失。

**Why this priority**: 这是 72 个 ERROR 假阳性中最主要的来源。解决后 ERROR 数量预计从 72 降至个位数。

**Independent Test**: 对 mumu 项目运行诊断，验证依赖接口 `OAuth2AuthorizationService` 不再报告为缺失（因为自动配置 bean `DefaultOAuth2AuthorizationService` 实现了它）。

**Acceptance Scenarios**:
1. **Given** 自动配置注册了 `DefaultOAuth2AuthorizationService` Bean 且该类 `implements OAuth2AuthorizationService`，用户 Bean 依赖 `OAuth2AuthorizationService`，**When** 运行诊断，**Then** 该依赖不报告为缺失。
2. **Given** 自动配置注册了 `RoleGatewayImpl` Bean 且该类 `implements RoleGateway`，用户 Bean 依赖 `RoleGateway`，**When** 运行诊断，**Then** 该依赖不报告为缺失。
3. **Given** 用户 Bean 依赖 `accountService`（它自己的接口，无实现类），**When** 运行诊断，**Then** 仍然报告为缺失（真正的缺失）。

---

### User Story 2 - 接口解析覆盖源码 Bean (Priority: P2)

用户项目的源码 Bean 同样识别 `implements` 关系。例如 `UserServiceImpl implements UserService`，依赖 `UserService` 的 Bean 应该能匹配到 `UserServiceImpl`。

**Why this priority**: 源码 Bean 的接口匹配与自动配置 Bean 使用同一机制，实现一次即覆盖全部。

**Independent Test**: 对包含 `UserServiceImpl implements UserService` 的项目运行诊断，依赖 `UserService` 的 Controller 不报告缺失。

**Acceptance Scenarios**:
1. **Given** 源码中存在 `UserServiceImpl` 且标注 @Service，实现了 `UserService`，**When** Controller 依赖 `UserService`，**Then** 不报告缺失。

---

### Edge Cases

- Java 8+ 接口中的 `default` 方法不影响匹配。
- 泛型接口（如 `Gateway<T>`）在 implements 子句中可能写为 `implements Gateway<User>`——提取原始接口名 `Gateway` 作为匹配 key。
- 一个类实现多个接口，所有接口都应与该类建立映射。
- 间接实现（A implements B, B extends C）——仅匹配直接 `implements` 声明中的接口，不递归追查父接口链。
- `@ConfigurationProperties` 类与其前缀/命名约定无关——仅通过 `implements` 源码声明匹配。

## Requirements *(mandatory)*

### Functional Requirements

**源码接口解析**

- **FR-I01**: 系统 MUST 在解析 Java 源码时，提取 `class Xxx implements Abc, Def` 声明中的接口名称（简单类名，去除泛型参数）。
- **FR-I02**: 系统 MUST 为每个解析到的类建立 `interface_name → [impl_class_names]` 映射。
- **FR-I03**: 接口名 MUST 使用 import 解析推断完整限定名（如 `OAuth2AuthorizationService`），与依赖声明中的类名匹配。

**依赖匹配增强**

- **FR-I04**: Bean 缺失检测 MUST 在检查依赖时，除按 Bean name 和 class_name 匹配外，还按 `interface → impl_classes` 映射匹配。若依赖 target 是某接口，且 Bean 图中有实现该接口的 Bean，则该依赖视为已满足。
- **FR-I05**: 接口匹配 MUST 仅对 `BeanSource::UserDefined` Bean 的依赖生效（用户代码需要接口 → 自动配置/用户源码提供实现）。`BeanSource::AutoConfig` Bean 的依赖不应用此规则（自动配置 Bean 的依赖在 jar 内部已自洽）。

**性能约束**

- **FR-I06**: 接口解析 MUST 在现有源码扫描阶段一并完成，不增加额外的文件遍历。性能开销 ≤10%（即增加 ≤0.1s）。

## Success Criteria *(mandatory)*

- **SC-I01**: mumu 项目 ERROR 数量从 72 降至 ≤15（减少 ≥79%）。
- **SC-I02**: 不引入新的假阴性——用户项目真正缺失的 Bean（如 `accountService` 接口无实现类）仍然报告 ERROR。
- **SC-I03**: 接口解析在现有扫描阶段完成，额外耗时 ≤0.1s。

## Assumptions

- Java 源码中 `implements` 声明格式规范：`class X implements InterfaceA, InterfaceB { ... }`。泛型参数在接口名中忽略。
- 接口名在 `imports` 声明中可解析；无法解析的使用简单名（与依赖声明一致）。
- 一个实现类可以同时注册为 Bean（@Component/@Service/@Bean 等），无需额外标记。
- 间接实现链（A extends B, B implements C）不在本 spec 范围内——仅匹配直接 `implements`。
