# Doctor 产品需求文档（PRD）

**产品名称：** Doctor

**产品定位：** 面向开发者的软件系统智能诊断引擎（Diagnostic Engine）

**产品形态：** CLI First，支持 MCP、CI/CD、REST API 等多种接入方式。

---

## 一、产品背景

随着软件系统规模不断扩大，开发者面临的问题已经从"如何编写代码"逐渐转变为"如何理解系统"。

现代 AI 工具已经能够帮助开发者：

- 编写代码
- 修改代码
- 重构代码
- 解释代码

但是对于下面这类问题，仍然缺乏专业工具：

- 为什么 Bean 没有注入？
- 为什么自动配置没有生效？
- 为什么事务没有开启？
- 为什么应用启动变慢？
- 为什么配置没有生效？
- 为什么容器启动失败？
- 为什么 Kubernetes Pod 不断重启？
- 为什么 Redis 主从异常？
- 为什么数据库连接耗尽？

这些问题本质上不是代码生成问题，而是系统诊断问题。

Doctor 的目标不是帮助开发者写代码，而是帮助开发者理解软件系统。

---

## 二、产品目标

构建一个统一的软件系统智能诊断平台，为开发者提供基于证据的自动化诊断能力。

Doctor 能够：

- 自动发现系统结构
- 建立系统模型
- 收集运行证据
- 执行诊断规则
- 输出根因分析
- 使用 AI 对诊断结果进行解释

最终帮助开发者快速定位问题，提高系统可理解性。

---

## 三、产品定位

Doctor 是一个软件系统智能诊断引擎（Diagnostic Engine）。

Doctor 不是：

- IDE 插件
- AI 编程助手
- 自动编码 Agent
- 日志分析平台
- 监控平台

Doctor 的职责只有一个：

**理解系统，并完成系统诊断。**

---

## 四、产品原则

### CLI First

CLI 是 Doctor 的核心交互方式。

所有能力必须能够通过命令行完成。

IDE、Web、MCP 等均属于扩展能力。

### Diagnosis First

Doctor 首先完成诊断。

AI 不负责诊断。

AI 只负责解释诊断结果。

### Evidence First

所有诊断必须具备证据。

每一个结论都必须能够回答：

- 为什么得出这个结论
- 使用了哪些证据
- 证据来源是什么
- 可信度是多少

Doctor 不允许猜测。

### Runtime First

Doctor 不仅理解源码。

Doctor 同时理解：

- 运行状态
- 配置
- 元数据
- 拓扑结构
- 日志
- 指标

Doctor 建立的是系统模型，而不是代码模型。

### Plugin First

Doctor 核心保持技术无关。

所有技术支持均通过插件实现。

例如：

- Spring
- Redis
- Docker
- Kubernetes
- PostgreSQL
- Kafka

均作为插件接入。

---

## 五、目标用户

主要用户：

- Java 开发工程师
- 后端开发工程师
- DevOps 工程师
- SRE 工程师
- 架构师
- AI 编程工具使用者

---

## 六、核心能力

### 1. 系统扫描（Scanner）

自动识别当前项目技术栈。

支持识别：

- Maven
- Gradle
- Spring Boot
- Docker
- Kubernetes
- Redis
- MyBatis
- PostgreSQL
- Kafka
- Node.js
- Python

输出统一系统描述。

### 2. 系统建模（Model Builder）

根据扫描结果建立系统模型。

例如：

Spring：

- Bean 图
- 配置图
- 自动配置图
- 依赖关系图

Docker：

- 镜像关系
- 网络关系
- 容器关系

Kubernetes：

- Deployment
- Service
- Pod
- Ingress

所有模型均采用统一数据结构。

### 3. 证据收集（Evidence Engine）

收集诊断所需事实。

包括：

- 源码信息
- 配置文件
- 运行状态
- Actuator 信息
- 日志
- Metrics
- 环境变量
- JVM 信息
- 网络信息
- 容器信息

所有证据均保留来源。

### 4. 规则诊断（Rule Engine）

根据规则执行自动诊断。

例如：

Spring：

- Bean 缺失
- Bean 冲突
- 循环依赖
- 自动配置失败
- 配置冲突
- 事务失效
- AOP 未生效
- 条件装配失败

Redis：

- 主从异常
- 内存压力
- 慢查询

Docker：

- 端口冲突
- 镜像异常
- 网络异常

Kubernetes：

- CrashLoopBackOff
- ImagePullBackOff
- OOMKilled

规则支持持续扩展。

### 5. AI 解释（AI Explain）

AI 不负责诊断。

AI 根据诊断结果生成：

- 问题解释
- 根因分析
- 影响分析
- 修复建议
- 学习资料

AI 必须基于诊断结果进行解释，不允许脱离诊断结果自由推理。

---

## 七、产品架构

```
CLI
↓
Diagnostic Engine
├── Scanner
├── Model Builder
├── Evidence Engine
├── Rule Engine
└── AI Explain
```

所有外部接口均调用 Diagnostic Engine。

---

## 八、插件体系

Doctor 提供插件机制。

每个插件包括：

- Scanner
- Model Builder
- Evidence Collector
- Rule Provider

插件之间互相独立。

Doctor Core 不依赖任何具体技术。

---

## 九、产品接口

### CLI

主要命令：

```
doctor diagnose
doctor explain
doctor graph
doctor health
doctor report
doctor fix
```

CLI 是默认入口。

### MCP

Doctor 提供 MCP Server。

供 Claude Code、Codex、Cursor 等 AI 工具调用。

AI 不直接分析系统，而是调用 Doctor 获取诊断结果。

### REST API

Doctor 可启动 HTTP 服务。

提供统一诊断接口。

供平台系统调用。

### CI/CD

支持：

- GitHub Actions
- GitLab CI
- Jenkins
- Azure DevOps

在持续集成阶段自动完成诊断。

---

## 十、输出能力

支持输出：

- 终端输出
- JSON
- Markdown
- HTML
- SARIF
- PDF

输出内容包括：

- 系统概览
- 问题列表
- 严重程度
- 证据
- 根因
- 修复建议
- 健康评分

---

## 十一、MVP 范围

第一阶段仅支持 Spring Boot。

包括：

- Bean 诊断
- 自动配置诊断
- 配置诊断
- 事务诊断
- 启动分析
- 健康评分
- CLI 输出
- JSON 输出
- AI 解释

---

## 十二、后续规划

| 阶段 | 内容 |
|------|------|
| 第二阶段 | Redis、MyBatis、Kafka |
| 第三阶段 | Docker、Docker Compose、Nginx |
| 第四阶段 | Kubernetes、Helm、Istio |
| 第五阶段 | PostgreSQL、MySQL、MongoDB |

最终形成统一的软件系统诊断平台。

---

## 十三、非目标

Doctor 不负责：

- 代码生成
- 自动编码
- 自动修改业务代码
- 日志采集平台
- APM 监控平台
- 替代 IDE
- 替代 AI 编程助手

Doctor 专注于软件系统诊断能力。

---

## 十四、成功标准

Doctor 应能够回答以下问题：

- 系统当前处于什么状态？
- 出现了哪些问题？
- 每个问题的根因是什么？
- 根因依据是什么？
- 问题影响范围是什么？
- 应如何修复？
- 修复建议可信度是多少？

Doctor 的所有诊断结果均应具备可验证的证据，并能够帮助开发者快速理解系统行为。
