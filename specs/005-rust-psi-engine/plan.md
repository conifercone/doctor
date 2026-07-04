# Implementation Plan: Rust PSI 引擎（tree-sitter）

**Branch**: `005-rust-psi-engine` | **Date**: 2026-07-04 | **Spec**: [spec.md](./spec.md)

## Summary

用 tree-sitter + tree-sitter-java 替换当前 `src/model/bean_graph.rs` 中的正则表达式解析器，建立 CST→AST→PSI 三层架构。新增 `src/psi/` 模块（5 文件），新增 3 个依赖（tree-sitter, tree-sitter-java, sled 移入 cargo add）。修改 `bean_graph.rs`（删除正则逻辑，改为调用 PSI 层）和 `diagnose.rs`（集成 PSI 索引）。

## Technical Context

**新增依赖**: `tree-sitter = "0.26"`, `tree-sitter-java` (grammar crate), `sled = "0.34"`

**修改文件**: `Cargo.toml`、`src/model/bean_graph.rs`、`src/cli/diagnose.rs`、`src/lib.rs`

**新增文件**: `src/psi/mod.rs`、`src/psi/cst.rs`、`src/psi/ast.rs`、`src/psi/index.rs`、`src/psi/bean_collector.rs`

**保留/复用**: `src/model/bean_graph.rs` 的数据结构（BeanDef/BeanDep/BeanGraph）、`src/classpath/` 模块（class 常量池解析器）

## Constitution Check: 10/10 PASS

- **VII (精简依赖)**: tree-sitter + tree-sitter-java + sled 共 3 个新增依赖。tree-sitter 引入 C 编译依赖（cc crate 已有），sled 是纯 Rust。均已论证必要性。

## Project Structure

```text
src/psi/
├── mod.rs           # 模块入口：orchestrate CST→AST→PSI→BeanGraph
├── cst.rs           # tree-sitter 封装：parse .java → CST tree
├── ast.rs           # CST→AST 转换：提取类/注解/字段/方法
├── index.rs         # 全局 PSI 索引：sled KV，FQCN→PsiClass
└── bean_collector.rs # Bean 收集：遍历 AST，识别 4 类 Bean，构建 BeanGraph

src/model/
└── bean_graph.rs    # 数据结构保留，build_bean_graph() 改为调用 psi 模块
```

## 实现策略

### Phase 1: CST 层 (`cst.rs`)

封装 tree-sitter 解析：

```rust
pub fn parse_java_file(path: &Path) -> Result<Tree, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;
    parser.parse(&source, None).ok_or("parse failed")
}
```

### Phase 2: AST 层 (`ast.rs`)

遍历 CST 提取 Java AST 节点：

- PackageDeclaration → 包名
- ImportDeclarations → HashMap<simple_name, FQCN>
- ClassDeclaration → PsiClass（类名、FQCN、interfaces、注解列表）
- FieldDeclarations → PsiField（类型、注解、@Autowired/@Qualifier 标记）
- MethodDeclarations → PsiMethod（返回类型、参数、注解、@Bean 标记）

### Phase 3: PSI 索引 (`index.rs`)

sled 数据库存储：
```
Key: FQCN (String) → Value: PsiClass (bincode/serde serialized)
Key: ANNOTATION_INDEX:{annotation_name} → Value: [FQCNs]
Key: IMPORT_INDEX:{simple_name} → Value: [FQCNs]
```

### Phase 4: Bean 收集 (`bean_collector.rs`)

遍历全局索引，识别 4 类 Bean：stereotype 注解、@Bean 方法、@Import、自动配置扫描。

### Phase 5: 替换正则

`bean_graph.rs` 的 `build_bean_graph()` 改为调用 `psi::build_bean_graph()`。旧正则逻辑 #ifdef 保留一个版本（feature gate），验证通过后删除。
