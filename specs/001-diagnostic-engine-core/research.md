# Research: Doctor 诊断引擎核心（MVP）

**Date**: 2026-07-04

## 1. Rust CLI Framework

**Decision**: `clap` v4+ with derive macro

**Rationale**:
- Clap is the de facto standard for Rust CLI applications (100M+ downloads).
- Derive macro provides declarative, type-safe argument definitions that align with Rust best practices.
- Built-in support for subcommands (`diagnose`, `explain`), argument validation, help text generation.
- No competitor matches its ecosystem maturity and documentation quality.

**Alternatives considered**:
- `bpaf` — lighter weight but smaller community and fewer examples
- Manual arg parsing — violates "don't reinvent the wheel"
- `lexopt` — too minimal; would need to build subcommand dispatch manually

## 2. Async Runtime

**Decision**: `tokio` (rt-multi-thread feature)

**Rationale**:
- Required for `reqwest` HTTP client (Actuator endpoint polling, LLM API calls).
- Multi-threaded runtime allows parallel evidence collection (source analysis + config parsing + Actuator queries).
- Industry standard — largest Rust async ecosystem, best documentation.

**Alternatives considered**:
- `async-std` — smaller ecosystem, reqwest requires tokio anyway
- `smol` — too minimal, less mature
- Blocking-only — would serialize HTTP operations, hurting performance for Actuator polling

## 3. HTTP Client

**Decision**: `reqwest` with `native-tls`

**Rationale**:
- De facto Rust HTTP client. Supports both blocking and async APIs.
- `native-tls` uses OS-native TLS (Security.framework on macOS, OpenSSL on Linux) — no extra certificate management.
- Connection pooling for repeated Actuator endpoint calls.
- Timeout support critical for graceful degradation when runtime info is unavailable.

**Alternatives considered**:
- `ureq` — blocking-only, doesn't integrate with tokio
- `hyper` — too low-level; would need manual TLS, connection management
- `curl` crate — C dependency, heavier build

## 4. Serialization Format

**Decision**: `serde` + `serde_json` + `serde_yaml`

**Rationale**:
- JSON output is mandatory (SC-006 requires sarif → JSON-based).
- YAML for reading Spring Boot config files (`application.yml`).
- `serde` is the universal Rust serialization framework — every Rust project uses it.
- Derive macros minimize boilerplate for model structs.

**Alternatives considered**:
- `simd-json` — performance-oriented but less compatible with serde ecosystem
- Manual JSON building — error-prone, hard to maintain

## 5. Error Handling

**Decision**: `thiserror` for library-style errors, `anyhow` for CLI glue code

**Rationale**:
- Constitution VI mandates structured error types and clear error messages.
- `thiserror` provides `#[derive(Error)]` — minimal boilerplate, automatic `Display` + `source()`.
- `anyhow` (or `eyre`) for `main.rs` and CLI dispatch where detailed error chains are needed but typed errors are overkill.
- Both are recommended by the Rust community and constitution.

**Alternatives considered**:
- Manual `std::error::Error` impl — too verbose
- `snafu` — more complex API, smaller community
- Only `anyhow` — insufficient for library code (Constitution VI requires structured types)

## 6. Maven POM Parsing

**Decision**: `quick-xml` for XML parsing

**Rationale**:
- Rust standard library has no XML parser.
- `quick-xml` is the fastest and most widely used Rust XML library.
- Pull-parser model is efficient for extracting specific elements (dependencies, parent POM).
- POM files are typically <500KB — performance is not a bottleneck.

**Alternatives considered**:
- `roxmltree` — DOM-based, easier API but loads entire document into memory
- `xml-rs` — older, slower, less maintained
- Java interop (`pom.xml` → effective POM via `mvn help:effective-pom`) — requires Maven installed, violates offline requirement

## 7. Java Source Code Analysis

**Decision**: Manual AST-like parsing via `regex` + structured pattern matching

**Rationale**:
- Full Java parser (e.g., tree-sitter, javaparser) is overkill for MVP — we only need to extract annotations, class declarations, method signatures, and `@Autowired`/`@Bean`/`@Transactional` usage.
- Regex-based pattern matching covers 95%+ of Spring Boot diagnostic use cases.
- Zero additional dependencies — Rust's `regex` crate is lightweight and fast.

**Alternatives considered**:
- `tree-sitter` + `tree-sitter-java` — full AST, more accurate but heavyweight dependency
- `javaparser` (calling Java tool) — requires JVM, violates offline and dependency constraints
- Source code as plain text scanning — too fragile for complex patterns

**Decision refined**: Use `regex` for MVP. Plan migration path to tree-sitter if accuracy issues arise.

## 8. Terminal Output Formatting

**Decision**: `colored` (or `ansi_term`)

**Rationale**:
- SC requires "终端彩色输出" with health score and issue list.
- `colored` is simple: `"text".red().bold()` style API, no template engine needed.
- ANSI escape codes work on all modern terminals (macOS Terminal, iTerm2, Linux terminals, Windows Terminal).

**Alternatives considered**:
- `termcolor` — more cross-platform (Windows console API) but more verbose API
- `crossterm` / `ratatui` — full TUI framework, overkill for simple colored output
- `console` — nice API but adds padding/formatting features we don't need yet

## 9. HTML Output

**Decision**: Inline template rendering via `format!()` or a minimal template approach

**Rationale**:
- HTML output is a simple diagnostic report — no interactive components.
- Avoid adding `handlebars` or `tera` dependencies for MVP (Constitution VII).
- If HTML complexity grows, revisit template engine decision.

**Alternatives considered**:
- `handlebars` — powerful but adds dependency weight
- `tera` — Jinja2-like syntax but another dependency
- `maud` — compile-time HTML macros, interesting but niche

## 10. Plugin Architecture

**Decision**: Rust trait-based plugin system (static dispatch via generics or dynamic dispatch via `Box<dyn Trait>`)

**Rationale**:
- Constitution II mandates simplicity: no dynamic library loading (dlopen) in MVP.
- Plugins are Rust modules that implement traits: `Scanner`, `ModelBuilder`, `EvidenceCollector`, `RuleProvider`.
- Plugin registry: scan default directory for plugin config files (TOML), load corresponding Rust modules.
- Future: dynamic loading via `libloading` or Wasm runtime when plugin ecosystem matures.

**Alternatives considered**:
- `libloading` (dlopen/dlsym) — unsafe code required, violates Constitution I without strong justification
- Wasm-based plugins (wasmtime) — heavy dependency, over-engineered for MVP
- Script-based plugins (Rhai, mlua) — scripting overhead, type-safety loss

## 11. SARIF Output

**Decision**: Serde-serializable SARIF struct types

**Rationale**:
- SARIF v2.1.0 is a JSON-based format with a well-defined JSON Schema.
- Define Rust structs that serialize to the SARIF schema — no external SARIF library exists in Rust.
- Simple model: `SarifLog { runs: Vec<Run> }` → `serde_json::to_string_pretty()`.

**Alternatives considered**:
- Manual JSON building — duplicate work
- SARIF via Python/Node bridge — unnecessary complexity

## 12. LLM API Integration

**Decision**: `reqwest` POST to OpenAI-compatible chat completions endpoint

**Rationale**:
- Industry standard API format (OpenAI Chat Completions API), supported by most LLM providers.
- Structured prompt: system message with diagnostic context, user message with problem summary.
- FR-030 requires structured summary only — small payload (<2KB per request).
- Configurable endpoint URL and API key via environment variable or config file.

**Alternatives considered**:
- Anthropic-specific API — vendor lock-in
- `llm-chain` / `langchain-rust` — heavy dependencies, immature
- Local model via `candle` / `burn` — too heavy for MVP scope

## 13. Configuration File Format

**Decision**: TOML (`.doctor.toml`) in project root or user home

**Rationale**:
- TOML is Rust's native config format (Cargo.toml).
- Human-readable, well-specified.
- Supports plugin enable list, LLM endpoint config, output preferences.

**Alternatives considered**:
- YAML — already using serde_yaml for Spring Boot configs, but TOML is more Rust-idiomatic
- JSON — less human-friendly for hand-editing
- HCL — niche, no Rust ecosystem
