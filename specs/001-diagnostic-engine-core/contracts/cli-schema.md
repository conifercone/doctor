# CLI Contract: Doctor 诊断引擎

**Date**: 2026-07-04

## Command Structure

```
doctor <COMMAND> [OPTIONS]
```

## Commands

### `doctor diagnose`

执行完整诊断流程：扫描 → 建模 → 收集证据 → 执行规则 → 输出报告。

```text
USAGE:
    doctor diagnose [OPTIONS] [PATH]

ARGS:
    <PATH>  项目根目录路径 [default: .]

OPTIONS:
    -o, --output <FORMAT>      输出格式 [default: terminal]
                               可选: terminal, json, markdown, html, sarif
    -p, --plugin <NAME>        启用的插件名称（可多次指定）
    --no-ai                    跳过 AI 解释步骤
    --offline                  强制离线模式（不尝试网络连接）
    --timeout <SECONDS>        诊断超时时间（秒） [default: 120]
    -v, --verbose              详细输出模式
    -h, --help                 显示帮助信息

EXIT CODES:
    0   诊断成功完成（无论是否发现问题）
    1   诊断过程出现错误（项目无法识别、解析失败等）
    2   命令行参数错误

EXAMPLES:
    doctor diagnose
    doctor diagnose /path/to/spring-boot-project
    doctor diagnose --output json
    doctor diagnose --plugin spring-boot --output sarif
    doctor diagnose --offline -o markdown
```

### `doctor explain`

对已有诊断报告进行 AI 自然语言解释。

```text
USAGE:
    doctor explain [OPTIONS] [REPORT]

ARGS:
    <REPORT>  诊断报告 JSON 文件路径 [default: 最近一次诊断结果]

OPTIONS:
    --api-url <URL>        LLM API endpoint URL [env: DOCTOR_LLM_URL]
    --api-key <KEY>        LLM API key [env: DOCTOR_LLM_KEY]
    --model <MODEL>        LLM model name [default: claude-sonnet-4-6]
    --locale <LANG>        输出语言 [default: zh-CN] 可选: zh-CN, en-US
    -h, --help             显示帮助信息

EXIT CODES:
    0   解释成功生成
    1   LLM API 不可用或返回错误
    2   命令行参数错误
    3   诊断报告文件无效或不存在

EXAMPLES:
    doctor explain
    doctor explain ./diagnosis-report.json
    doctor explain --model gpt-4o --locale en-US
```

## Output Contracts

### Terminal Output (default)

```text
╔══════════════════════════════════════════════╗
║  Doctor Diagnosis Report                     ║
║  Project: my-spring-app                      ║
║  Health Score: 82/100                        ║
║  Duration: 12.3s                             ║
╚══════════════════════════════════════════════╝

System: Spring Boot 3.2.0 | Maven | Java 17

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Issues: 3 ERROR | 2 WARNING | 5 INFO
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🔴 [ERROR] [Bean] Bean 'userRepository' not found
   In: com.example.service.UserService (line 25)
   Fix: Define a @Bean method or add @Repository to UserRepository

🟡 [WARNING] [Config] Duplicate property 'server.port'
   Sources: application.yml:5, environment variable: SERVER_PORT
   ...

🔵 [INFO] [Startup] Bean 'dataSource' initialization took 2.3s
   ...
```

### JSON Output

```json
{
  "$schema": "https://doctor.dev/schemas/diagnostic-report/v1.json",
  "project_name": "my-spring-app",
  "timestamp": "2026-07-04T10:30:00Z",
  "duration_ms": 12300,
  "health_score": 82,
  "system_overview": {
    "build_tool": "Maven",
    "spring_boot_version": "3.2.0",
    "java_version": "17",
    "starters": ["spring-boot-starter-web", "spring-boot-starter-data-jpa"],
    "module_count": 1
  },
  "issues": [
    {
      "id": "BEAN-001",
      "title": "Bean 'userRepository' not found",
      "severity": "Error",
      "category": "Bean",
      "description": "UserService depends on UserRepository but no bean of type UserRepository is declared.",
      "evidence": [
        {
          "evidence_type": "SourceCode",
          "source": "src/main/java/com/example/service/UserService.java:25",
          "summary": "@Autowired field 'userRepository' of type UserRepository — no matching @Bean, @Repository, or @Component found",
          "reliability": "Confirmed"
        }
      ],
      "fix_suggestion": "Add @Repository to UserRepository interface or declare a @Bean method in a @Configuration class.",
      "confidence": "High"
    }
  ],
  "summary": {
    "total_rules_executed": 8,
    "total_evidence_collected": 45,
    "issues_by_severity": {"Error": 3, "Warning": 2, "Info": 5},
    "runtime_sources_available": true
  }
}
```

### SARIF Output

Compliant with SARIF v2.1.0 specification. Key mappings:

| Doctor Concept | SARIF Field |
|----------------|-------------|
| Issue ID | `results[].ruleId` |
| Severity.ERROR | `results[].level = "error"` |
| Severity.WARNING | `results[].level = "warning"` |
| Severity.INFO | `results[].level = "note"` |
| Evidence.source | `results[].locations[].physicalLocation` |
| Issue.category | `results[].taxa[].name` |

## Plugin Contract

### Trait: Scanner

```rust
pub trait Scanner: Send + Sync {
    /// 返回 Scanner 名称
    fn name(&self) -> &str;

    /// 检测当前项目是否匹配此 Scanner 的技术栈
    fn detect(&self, project_path: &Path) -> Result<bool>;

    /// 执行扫描，返回技术栈信息
    fn scan(&self, project_path: &Path) -> Result<SystemOverview>;
}
```

### Trait: ModelBuilder

```rust
pub trait ModelBuilder: Send + Sync {
    fn name(&self) -> &str;
    fn build_bean_graph(&self, project_path: &Path) -> Result<BeanGraph>;
    fn build_auto_config_model(&self, project_path: &Path) -> Result<AutoConfigModel>;
    fn build_config_model(&self, project_path: &Path) -> Result<ConfigModel>;
}
```

### Trait: EvidenceCollector

```rust
pub trait EvidenceCollector: Send + Sync {
    fn name(&self) -> &str;

    /// 收集源码级证据
    fn collect_source_evidence(&self, project_path: &Path) -> Result<Vec<Evidence>>;

    /// 收集配置文件证据
    fn collect_config_evidence(&self, project_path: &Path) -> Result<Vec<Evidence>>;

    /// 收集运行时证据（可选：Actuator endpoint）
    async fn collect_runtime_evidence(&self, base_url: &str)
        -> Result<Vec<Evidence>>;
}
```

### Trait: RuleProvider

```rust
pub trait RuleProvider: Send + Sync {
    fn name(&self) -> &str;

    /// 返回此 Provider 提供的所有诊断规则
    fn rules(&self) -> Vec<Box<dyn DiagnosticRule>>;
}

pub trait DiagnosticRule: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn category(&self) -> Category;

    /// 执行诊断，输入系统模型 + 证据，输出发现的问题
    fn diagnose(&self, model: &SystemModel, evidence: &[Evidence])
        -> Result<Vec<Issue>>;
}
```

## Configuration File Contract (`.doctor.toml`)

```toml
[plugins]
# 启用的插件列表
enabled = ["spring-boot"]

# 插件扫描目录
scan_dirs = ["~/.doctor/plugins/"]

[ai]
# LLM API 配置
api_url = "https://api.anthropic.com/v1/messages"
api_key_env = "DOCTOR_LLM_KEY"    # 从环境变量读取
model = "claude-sonnet-4-6"

[output]
# 输出默认值
default_format = "terminal"
color = true

[diagnosis]
# 诊断配置
timeout_seconds = 120
max_issues = 100
```
