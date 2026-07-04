# Feature Specification: 框架 Bean 全覆盖发现

**Feature Branch**: `006-framework-bean-discovery`

**Created**: 2026-07-04

**Status**: Draft

**Input**: 全面发现 Bean（用户定义 + 自动配置 + 框架内置），消除剩余 44 个 ERROR

## Problem

当前 auto-config Bean 发现仅扫描 `AutoConfiguration.imports` 中列出的类。但 Spring 框架的大量 Bean 来自：
- 直接标注 @Configuration 的类（通过 @ComponentScan 自动发现，不在 AutoConfiguration.imports 中）
- `spring.factories` 旧格式
- Spring Cloud 的 `BootstrapConfiguration`
- 接口实现类（`DiscoveryClient`、`GrpcChannelFactory` 等）

## Solution

扩展 jar 扫描策略：将全局类索引（178K 类）中的 @Configuration/@AutoConfiguration 类全部纳入解析，而不限于 AutoConfiguration.imports 列表。

## User Scenarios

### US1 — 消除框架 Bean 假阳性 (P1)

mumu 项目中依赖的 `DiscoveryClient`、`GrpcChannelFactory`、`ApplicationContext` 等框架 Bean，能从依赖 jar 中的 @Configuration 类发现，不再报告为缺失。

**Acceptance**: mumu ERROR 从 44 降至 ≤15。

## Requirements

- **FR-01**: 对全局类索引中所有包含 @Configuration/@AutoConfiguration 注解的 `.class` 文件执行解析（不仅限 AutoConfiguration.imports 中的类）。
- **FR-02**: 对 @Configuration 类名与 `*Configuration` 模式匹配的类（Spring 命名约定），即使注解检测失败也尝试解析。
- **FR-03**: 解析结果缓存到 sled，支持增量更新。

## Success Criteria

- **SC-01**: mumu ERROR 从 44 降至 ≤15。
- **SC-02**: 不引入新的假阴性（用户的真正缺失 Bean 仍然报告）。
- **SC-03**: 全量扫描时间不超过当前的 2 倍（≤50s）。

## Assumptions

- 全局类索引已覆盖所有依赖 jar（已验证 178K 类）。
- @Configuration 类的 .class 文件在对应 jar 中可读取。
