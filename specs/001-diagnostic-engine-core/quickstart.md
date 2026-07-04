# Quickstart: Doctor 诊断引擎核心（MVP）

**Date**: 2026-07-04

验证 Doctor 诊断引擎 MVP 功能端到端可用的操作指南。

## 前置条件

- Rust 1.85+ (stable) — `rustup update stable`
- 一个 Spring Boot 测试项目（使用 Maven 构建）
- （可选）Spring Boot Actuator 暴露的 endpoints

## 安装与构建

```bash
# 克隆并构建
cd /path/to/doctor
cargo build --release

# 验证二进制文件
./target/release/doctor --help
# 期望输出：显示子命令 diagnose, explain 和帮助信息
```

## 验证场景

### 场景 1：正常项目诊断

```bash
# 对一个标准 Spring Boot 项目执行诊断
./target/release/doctor diagnose /path/to/demo-spring-app

# 期望结果：
# - 终端彩色输出，包含项目概览（构建工具、Spring Boot 版本、Starters）
# - 显示健康评分（0-100）
# - 列出诊断问题（如有），按严重程度排序
# - 每个问题附带证据来源（文件:行号）
# - 退出码 0
```

### 场景 2：JSON 格式输出

```bash
./target/release/doctor diagnose /path/to/demo-spring-app --output json > report.json

# 验证 JSON 结构
cat report.json | python3 -m json.tool | head -50

# 期望结果：
# - 有效的 JSON 文档
# - 包含 project_name, timestamp, health_score, issues, summary 字段
# - issues 数组中每个对象包含 id, title, severity, category, evidence
```

### 场景 3：已知问题检出

```bash
# 对一个包含已知 Bean 缺失问题的测试项目运行诊断
./target/release/doctor diagnose tests/fixtures/bean-missing-app

# 期望结果：
# - 至少检出 1 个 ERROR 级别的 Bean 缺失问题
# - 问题证据指向具体的注入点和缺失的 Bean 类型
# - 健康评分 < 100（被 ERROR 扣分）
```

### 场景 4：无项目目录处理

```bash
# 在一个非 Java 项目目录中运行
mkdir -p /tmp/empty-dir
./target/release/doctor diagnose /tmp/empty-dir

# 期望结果：
# - 明确错误提示："No Spring Boot project detected"
# - 退出码 1（非崩溃）
# - 不输出空报告
```

### 场景 5：离线模式

```bash
# 断开网络后运行
./target/release/doctor diagnose /path/to/demo-spring-app --offline

# 期望结果：
# - 核心诊断正常完成（Scanner、Model Builder、Evidence Engine、Rule Engine）
# - AI 解释步骤自动跳过，提示 "AI explanation unavailable in offline mode"
# - 退出码 0
```

### 场景 6：SARIF 输出

```bash
./target/release/doctor diagnose /path/to/demo-spring-app --output sarif > report.sarif

# 验证 SARIF 结构
python3 -c "
import json
with open('report.sarif') as f:
    data = json.load(f)
assert data['\$schema'].endswith('sarif-2.1.0.json')
assert 'runs' in data
print('SARIF validation: OK')
"

# 期望结果：SARIF validation: OK
```

### 场景 7：AI 解释

```bash
# 先生成诊断报告
./target/release/doctor diagnose /path/to/demo-spring-app --output json > report.json

# 设置 LLM API key
export DOCTOR_LLM_KEY="your-api-key"

# 对报告进行 AI 解释
./target/release/doctor explain report.json

# 期望结果：
# - 输出每个问题的自然语言解释
# - 包含根因分析、影响范围、修复建议
# - 解释引用诊断证据编号
```

### 场景 8：插件发现

```bash
# 创建一个测试插件目录
mkdir -p ~/.doctor/plugins/demo-plugin

# 运行诊断并指定插件
./target/release/doctor diagnose /path/to/demo-spring-app --plugin demo-plugin -v

# 期望结果：
# - 详细输出中显示 "Scanning plugins directory: ~/.doctor/plugins/"
# - 列出发现的插件（含 demo-plugin）
# - 仅加载 demo-plugin（显式启用）
```

## 测试命令

```bash
# 运行所有测试
cargo test

# 仅运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test integration

# 检查代码格式
cargo fmt --check

# 运行 lints
cargo clippy -- -D warnings

# 测试覆盖率
cargo tarpaulin --out Html
```

## 期望的 CI 集成

```yaml
# GitHub Actions 示例
- name: Doctor Diagnosis
  run: |
    ./target/release/doctor diagnose . --output sarif > doctor-report.sarif
- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: doctor-report.sarif
```
