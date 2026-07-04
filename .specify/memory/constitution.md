<!--
  Sync Impact Report
  ==================
  Version change: [TEMPLATE] → 1.0.0 (initial ratification)
  Modified principles: N/A (first concrete version)
  Added sections:
    - All 10 Core Principles (expanded from 5 template slots)
    - Development Workflow section
    - Quality Gates section
  Removed sections: None (template placeholders replaced)
  Templates requiring updates:
    - .specify/templates/plan-template.md ✅ no changes needed (Constitution Check is dynamic)
    - .specify/templates/spec-template.md ✅ no changes needed
    - .specify/templates/tasks-template.md ✅ no changes needed
    - .specify/templates/commands/ — directory does not exist, N/A
  Follow-up TODOs: None
-->

# Doctor Constitution

## Core Principles

### I. 安全优先 (Safety First)

默认使用安全 Rust（safe Rust）。只有在满足以下全部条件时才允许使用 `unsafe` 代码块：

- 无安全 Rust 替代方案可实现所需功能；
- `unsafe` 代码块被最小化并隔离在独立模块中；
- 代码中 MUST 以 `// SAFETY:` 注释说明为何该操作是安全的，引用相关的不变量和前提条件；
- 所有 `unsafe` 代码 MUST 经过额外的代码审查。

**理由**：Rust 的安全保证是其核心价值。`unsafe` 代码绕过了编译器的安全检查，
可能引入内存安全漏洞。严格控制 `unsafe` 的使用是保护代码库长期可靠性的基础。

### II. 保持简单 (Keep It Simple)

优先选择简单、清晰、易维护的实现方案。具体规则：

- 避免过度设计和不必要的抽象层；
- 遵循 YAGNI（You Ain't Gonna Need It）原则——不为假设的未来需求增加复杂度；
- 新增抽象 MUST 有明确、当前的需求驱动，而非"以后可能需要"；
- 能用标准库解决的问题，不引入第三方库；
- 能用简单函数解决的问题，不引入 trait、宏或设计模式。

**理由**：简单代码更易理解、调试和维护。每增加一层抽象，
都增加了认知负担和潜在 bug 面。复杂性 MUST 被证明是必要的，而非默认选项。

### III. 遵循最佳实践 (Follow Best Practices)

遵循 Rust 官方规范和社区广泛认可的最佳实践：

- 代码 MUST 通过 `cargo clippy` 的所有警告（至少默认 lint 级别）；
- 优先使用标准库（`std`）中的类型和函数；
- 引入第三方依赖前 MUST 评估：是否成熟、活跃维护、被社区广泛采用；
- 遵循 Rust API 设计指南（Rust API Guidelines）；
- 使用 `rustfmt` 统一代码格式；
- 错误类型 SHOULD 实现 `std::error::Error` trait；
- 避免使用不稳定特性（nightly-only features），除非有充分理由并记录。

**理由**：一致遵循社区最佳实践降低新成员的上手成本，
减少因风格差异导致的代码审查摩擦，并使项目受益于 Rust 生态的集体经验。

### IV. 正确性优先 (Correctness First)

始终将正确性、可靠性和可维护性置于性能优化之前：

- 功能 MUST 先做对，再做快；
- 性能优化 MUST 有基准测试（benchmark）数据支撑，证明优化是必要的且有实际收益；
- 不得为了性能牺牲代码清晰度，除非基准测试证明性能瓶颈确实存在且优化收益显著；
- 所有并发/并行代码 MUST 通过 `cargo miri` 或等价工具检查数据竞争；
- 不变量和前置条件 SHOULD 以 `debug_assert!` 或明确文档记录。

**理由**：错误的快速代码没有价值。过早优化是万恶之源。
只有在测量证明性能不足时，优化才是合理的。

### V. 测试驱动质量 (Test-Driven Quality)

核心业务逻辑和公共接口 MUST 具备适当的测试：

- 公共 API（`pub` 函数、trait 方法）MUST 有文档测试（doctest）或单元测试；
- 核心业务逻辑模块 SHOULD 达到 ≥80% 的测试覆盖率；
- 对 bug 修复，MUST 先编写能复现 bug 的测试，确认测试失败后再修复；
- 集成测试 SHOULD 覆盖关键的端到端用户场景；
- 测试 MUST 在 CI 中自动运行，任何测试失败 MUST 阻止合并。

**理由**：测试是长期代码质量的保障。没有测试的代码就是没有规格说明的代码。
测试使重构安全、使回归可见、使新人可以放心修改代码。

### VI. 正确处理错误 (Handle Errors Correctly)

不得忽略错误或静默失败：

- 所有 `Result` 和 `Option` 值 MUST 被显式处理——禁止使用 `unwrap()` 和 `expect()`，
  除非在以下情况中可证明不会失败：单元测试、示例代码，或已通过不变量确保值存在；
- 错误信息 MUST 清晰描述发生了什么以及可能的原因；
- 库代码 MUST 使用 `thiserror` 或手动实现 `std::error::Error` 定义结构化错误类型；
- 应用代码 SHOULD 使用 `anyhow` 或 `eyre` 进行错误传播，同时保留错误链上下文；
- 不得使用空的 `catch` 或 `_ => {}` 吞掉错误——每个错误路径 MUST 被有意识地处理。

**理由**：静默失败是生产环境中最难排查的问题。
清晰的错误信息直接决定了故障排查效率和用户体验。

### VII. 精简依赖 (Minimal Dependencies)

新增依赖 MUST 具有明确的价值：

- 每个新增的第三方依赖 MUST 在代码审查中被明确论证其必要性；
- 优先选择成熟、活跃维护、下载量高的库；
- 评估依赖的传递依赖树——避免引入大量间接依赖的库；
- 对于简单功能（如字符串处理、基础数据结构），MUST 优先自行实现而非引入依赖；
- 定期审查和清理未使用的依赖（使用 `cargo-udeps` 或类似工具）。

**理由**：每个依赖都是维护负担和安全攻击面。依赖膨胀导致编译时间增长、
供应链风险增加、版本升级困难。Rust 标准库已提供丰富的功能。

### VIII. 文档即设计 (Documentation as Design)

公共接口 SHOULD 提供必要的文档：

- 所有 `pub` 项（函数、结构体、trait、模块）MUST 有文档注释（`///`）；
- 文档 MUST 说明"为什么"和"如何使用"，而非简单复述代码；
- 复杂算法和设计决策 SHOULD 以模块级文档（`//!`）记录设计意图和权衡；
- 不安全函数 MUST 在文档中说明调用者需要遵守的安全前置条件；
- 示例代码（doctest）SHOULD 是可编译和可运行的。

**理由**：好的文档是 API 设计的一部分。文档驱动设计迫使开发者
在实现前理清接口语义，减少后期返工。对维护者和使用者都是效率倍增器。

### IX. 保持一致性 (Maintain Consistency)

统一代码风格和开发流程：

- 代码 MUST 在每次提交前通过 `cargo fmt --check` 和 `cargo clippy`；
- 模块和文件命名 MUST 遵循项目既定约定（snake_case 文件名，模块层级清晰）；
- 提交信息 MUST 遵循 Conventional Commits 规范（如 `feat:`, `fix:`, `docs:`, `refactor:`）；
- CI 流水线 MUST 包含格式化检查、静态分析和测试三个步骤；
- 项目配置（`.editorconfig`, `rustfmt.toml`, `clippy.toml`）MUST 存在于仓库根目录。

**理由**：一致性降低认知负担。当代码风格统一时，审查者可以专注于逻辑和设计，
而非格式细节。自动化检查消除了人工判断的不确定性。

### X. 持续改进 (Continuous Improvement)

每次修改都应让代码库比之前更好：

- 修改代码时，清理遇到的死代码、过时注释和未使用的导入；
- 发现技术债务时，MUST 记录为 issue 或 TODO 注释（含负责人和日期），而非忽略；
- 重构 SHOULD 作为独立提交，不混入功能变更；
- 每完成一个功能，SHOULD 审视是否可以简化或删除相关代码；
- 每次提交前，检查是否引入了重复代码或不必要的耦合。

**理由**：代码库不是静态的——它持续演化。每个开发者都有责任
在离开代码时让它比自己发现时更整洁。累积的小改进带来长期可持续性。

## Development Workflow

### 分支策略

- `main` 分支 MUST 始终保持可发布状态；
- 功能开发 MUST 在特性分支（`feat/###-description`）上进行；
- 合并到 `main` MUST 通过 Pull Request，且至少通过 CI 检查；
- PR MUST 关联对应的 spec 文档（在 `specs/` 目录下）。

### 代码审查

- 所有 PR MUST 至少经过一次审查（review）方可合并；
- 审查者 MUST 检查：正确性、安全性（`unsafe` 使用）、错误处理、测试覆盖、文档完整性；
- 包含 `unsafe` 代码的 PR MUST 经过至少两位审查者同意。

### 质量门禁

在合并前，以下条件 MUST 全部满足：

1. `cargo fmt --check` 通过（无格式问题）；
2. `cargo clippy` 通过（无 lint 警告）；
3. `cargo test` 全部通过（单元测试 + 集成测试 + doctest）；
4. 测试覆盖率不低于现有水平（使用 `cargo-tarpaulin` 或类似工具跟踪）；
5. 若有 `unsafe` 代码，已通过 `cargo miri` 检查（如适用）。

## Quality Gates

在功能开发的各阶段，以下检查点适用：

| 阶段 | 检查内容 |
|------|---------|
| Spec 设计 | 是否违反"保持简单"原则？是否需要新的依赖？ |
| Plan 设计 | Constitution Check 全部通过？设计方案是否最简？ |
| 实现中 | 每个 `pub` 项是否有文档？测试是否先于实现？ |
| 代码审查 | 错误处理是否完整？`unsafe` 使用是否合理？ |
| 合并前 | CI 全部通过？覆盖率达标？ |

## Governance

本 Constitution 是 Doctor 项目的最高开发准则，所有设计决策、代码审查和
开发实践 MUST 遵守本文档中的原则。任何与 Constitution 的冲突 MUST
以 Constitution 为准，或在修改 Constitution 后继续。

### 修订流程

1. 提出修订 PR，说明变更理由和影响范围；
2. 修订 MUST 经过项目维护者讨论并达成共识；
3. 重大变更（原则增删、规则重新定义）MUST 附带迁移计划；
4. 修订后的 Constitution 版本号 MUST 按语义化版本规则递增。

### 版本规则

- **MAJOR**：原则的删除或不兼容的重新定义；
- **MINOR**：新增原则或章节，或实质性扩展指导；
- **PATCH**：措辞澄清、格式修正、非语义性优化。

### 合规审查

- 每个 PR 的审查者 MUST 验证变更符合 Constitution 原则；
- 每季度 SHOULD 进行 Constitution 适用性回顾，评估是否需要调整；
- 如需运行时开发指导，参见项目根目录的 `CLAUDE.md`。

**Version**: 1.0.0 | **Ratified**: 2026-07-04 | **Last Amended**: 2026-07-04
