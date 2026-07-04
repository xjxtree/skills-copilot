# Agent Copilot

[English](README.md) | [简体中文](README.zh-CN.md)

Agent Copilot 是一个原生 macOS 控制台，用于检查本地 coding agent
会话、技能目录、配置快照和验证证据，同时不扩大仓库的写入、脚本执行、
凭据、云同步或遥测边界。

## 能做什么

- 展示本地 agent 会话、技能目录和受支持的配置快照。
- 在原生 macOS app 后面使用类型化 Rust JSON stdio 服务。
- 默认保持本地分析可重复、可解释。
- 将可选 provider 调用限制在预览、脱敏、目标可见和显式确认之后。
- 将技能脚本、会话记录、LLM 输出和配置文件都视为不可信输入。

## 不做什么

- 不做云同步、账号系统、遥测、匿名崩溃上报或不受控的外部网络调用。
- 默认不调用 provider。
- 不提供隐藏的 apply/write 路径。
- 不会从扫描、导入、预览、建议或 LLM 输出中执行技能脚本。
- 不会把凭据写入项目目录、SQLite、日志、prompt、截图、报告或响应产物。
- 默认不做签名、公证、DMG、自动更新或发布自动化。v0.1.0 macOS ZIP 是手动限定范围的发布产物。

## 下载

从 GitHub Release 页面下载最新 macOS app：

- [Agent Copilot v0.1.0](https://github.com/xjxtree/agent-copilot/releases/tag/v0.1.0)
- Apple Silicon ZIP：
  [AgentCopilot-0.1.0-macos-arm64.zip](https://github.com/xjxtree/agent-copilot/releases/download/v0.1.0/AgentCopilot-0.1.0-macos-arm64.zip)
- Intel ZIP：
  [AgentCopilot-0.1.0-macos-x86_64.zip](https://github.com/xjxtree/agent-copilot/releases/download/v0.1.0/AgentCopilot-0.1.0-macos-x86_64.zip)

架构说明：Apple Silicon Mac 请选择 `arm64`，Intel Mac 请选择 `x86_64`。

v0.1.0 app 以未签名、未公证的 macOS app bundle 形式放在 ZIP 文件中。首次启动时，macOS Gatekeeper 可能需要用户显式批准。可以在 Finder 中右键选择 **打开**，或在 macOS 阻止首次启动时到 **系统设置 > 隐私与安全性** 中批准该 app。

## 使用

1. 从 Release 页面下载与你的 Mac 架构匹配的 ZIP。
2. 解压后，将 `AgentCopilot.app` 移到 `/Applications` 或你控制的其他本地目录。
3. 打开 `AgentCopilot.app`。
4. 在 app 内使用 **Scan** 或项目上下文控件，检查受支持的本地 agent 会话、技能根目录和配置快照。

Agent Copilot 是 local-first 的。除非你配置了可选 provider 功能，并且预览 prompt 后显式确认，否则它不会发送 provider 请求。

## 从源码构建

前置要求：

- macOS 13 或更新版本。
- Xcode Command Line Tools。
- 带 Cargo 的 Rust toolchain。
- 带 Corepack/pnpm 的 Node.js。

构建并运行 macOS app：

```sh
git clone https://github.com/xjxtree/agent-copilot.git
cd agent-copilot
corepack enable
pnpm install
pnpm build:macos
open dist/AgentCopilot.app
```

仅构建指定架构的 app bundle，不启动：

```sh
pnpm build:macos:arm64
rustup target add x86_64-apple-darwin
pnpm build:macos:x86_64
```

运行主要本地验证 gate：

```sh
pnpm check:macos
pnpm check:privacy
```

## 文档

| 文件 | 用途 |
| --- | --- |
| `AGENTS.md` | 面向 coding agent 的操作规则 |
| `CLAUDE.md` | Claude Code 兼容行为 |
| `docs/architecture.md` | 仓库架构 |
| `docs/adapters/agent-adapters.md` | Adapter 根目录、写入范围和阻断操作 |
| `docs/service-protocol.md` | 类型化服务方法契约 |
| `docs/security-model.md` | 安全与隐私规则 |
| `docs/data-model.md` | 持久化与临时数据模型 |
| `docs/ai-layer.md` | Provider 与 LLM 安全边界 |
| `docs/ui-delivery-standards.md` | UI 与截图验证标准 |
| `docs/plans/roadmap.md` | 后续规划与非目标 |
| `docs/plans/development-tasks.md` | 当前任务路由 |
| `CHANGELOG.md` | 版本化 release 影响说明 |
| `docs/verification/` | 版本检查清单和 benchmark 趋势 |

## 常用命令

```sh
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
pnpm test:macos-native-models
swift test --package-path apps/macos
pnpm check:macos
pnpm check:privacy
pnpm verify:gate-parity
pnpm verify:service-protocol-drift
pnpm verify:module-size
pnpm verify:macos-ui-layout
pnpm smoke:macos-app -- --fixture-data --capture-window
pnpm dev:macos
```
