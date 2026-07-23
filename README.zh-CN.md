# Agent Copilot

[English](README.md) | [简体中文](README.zh-CN.md)

Agent Copilot 是一款面向多编码 Agent 的原生 macOS 就绪度与工作延续控制中心。
它帮助你确认各 Agent 在所选项目中实际可用的能力、理解需要处理的问题，并从上次
结束的位置继续本地工作。

## 项目信息总览

- **平台：** 原生 macOS 桌面 App。
- **最新版本：** 通过
  [GitHub Releases](https://github.com/xjxtree/agent-copilot/releases/latest)
  页面查看当前 App 下载、release notes 和校验文件。
- **支持的 Agent：** Claude Code、Codex、opencode、Pi、Hermes、OpenClaw。
- **主要用途：** 项目与 Agent 环境核验、有效技能检查、本地会话延续、基于证据
  的问题处理，以及受保护的技能与配置工作流。
- **发布形式：** 面向 Apple Silicon 和 Intel Mac 的架构专用 ZIP 下载包。

## 技术架构

Agent Copilot 按 local-first 的桌面产品方式组织：

- 有边界的 adapter 与 scanner 只读取文档明确允许的本地 Agent 数据源。
- Rust 产品逻辑通过类型化服务协议派生项目健康度、技能状态、会话证据与受保护
  操作。
- 原生 macOS App 展示项目、技能、会话、设置和工作流，不在 Swift 中重新实现
  adapter 策略。
- 本地缓存让列表和搜索保持流畅，同时保留显式刷新入口处理较重的扫描。
- 可选 AI 只能在 prompt 预览和明确确认后解释经过脱敏、绑定 revision 的证据；
  它不是环境事实或写入权限的来源。
- 仓库内置 fixtures 和验证脚本，用于在发版前一起检查 App、服务和文档变更。

这种拆分让桌面体验保持原生和快速，同时把不同 Agent 的解析规则和工作流逻辑
放在共享项目代码中维护。

长期产品与交互契约见 [`docs/product-design.md`](docs/product-design.md)。对应工程
改造必须按 [`docs/implementation-sequence.md`](docs/implementation-sequence.md)
中的编号依赖顺序执行；执行状态与验证结果只记录在当前任务会话、相关 GitHub Issue
或 PR 中。

## App 产品功能

OpenAI 当前的桌面体验已将 Codex 集成到 ChatGPT App 中。Agent Copilot 仍保留
`codex` 作为稳定的 adapter 标识，继续读取同一套安全的 `$CODEX_HOME` 配置和
本地会话目录。技能清单只以持久化的 `SKILL.md` 文件为依据，也包括已启用、已安装
Codex 插件中由 manifest 明确声明的技能目录；插件 store/cache 绝不会作为通用扫描
源，物理 cache 路径也不会展示。Agent Copilot 不向 Codex runtime 查询技能清单。

- **项目上下文：** 选择需要核对的工作区，使 Agent 证据、技能、会话、搜索与配置
  历史保持在同一项目范围。最近项目可逐条删除或整体清空，且不会暗中切换当前项目。
- **技能：** 扫描受支持的 Agent 根目录，区分已安装、已启用和实际有效，按 Agent、
  范围和状态筛选，并查看证据、问题项与受支持的开关操作。文件系统发现的技能保留
  来源与只读归属；已安装的 Codex 插件副本可见但始终不可写。
- **会话：** 浏览受支持 adapter 的本地会话预览，在有边界的历史中搜索，并查看
  脱敏后的用户消息、Agent 最终回复、时间、来源完整性和技能调用摘要。
- **全局搜索：** 从顶部工具栏搜索，并直接跳转到匹配的技能、会话、配置或详情页。
- **就绪度：** 确定性的本地证据用于报告环境覆盖；指定任务后可做匹配技能预检，
  未提供任务时不会声称某个 Agent 对所有工作都已就绪。
- **配置与证据：** 查看受支持 Agent 的配置快照、当前文件与诊断信息，预览回滚
  差异，并在 App 支持的范围内执行受保护的配置操作。
- **技能包管理：** 通过本地 `npx skills` 管理器搜索、预览、安装、更新、移除
  和创建本地技能包。
- **Provider 观测：** 按日期范围查看 Agent Copilot 自身可选 AI 功能产生的
  provider 用量、模型活动、延迟、令牌估算和成本估算。它不统计 Claude Code、
  Codex、opencode、Pi 等被管理 Agent 自己配置的 provider 或其调用消费。
- **可选上下文 AI：** 预览并明确确认经过脱敏的 Agent Copilot 请求，用于解释或
  排序既有本地证据。输出保持不可信和仅复制，不能授权写入或执行脚本。
- **外观：** 在设置中跟随系统外观，也可以手动选择浅色或深色主题。

## 下载使用

从 [GitHub Releases](https://github.com/xjxtree/agent-copilot/releases/latest)
页面下载最新 macOS App。每个 release 会提供不同架构的 ZIP 和校验文件。

Apple Silicon Mac 请选择 `arm64`，Intel Mac 请选择 `x86_64`。

1. 下载与你的 Mac 架构匹配的 ZIP。
2. 解压后，将 `AgentCopilot.app` 移到 `/Applications` 或你控制的其他本地目录。
3. 打开 `AgentCopilot.app`。
4. 选择项目并查看其本地技能、会话、配置证据、Agent Copilot AI provider
   活动和设置。

如果 macOS 阻止首次启动，可以在 Finder 中右键选择 **打开**，或到
**系统设置 > 隐私与安全性** 中批准该 App。

技能包管理流程会使用本机的 `npx skills` 管理器，因此需要本机安装 Node/npm。
App 从 Finder 启动时会自动探测 Homebrew、Volta、asdf、nvm 等常见路径；自定义
安装位置可以设置 `SKILLS_COPILOT_NPX_PATH`。

ChatGPT 的 Plugin Directory 与 Agent Copilot 的技能包管理是两个独立来源。
Agent Copilot 只读取已启用插件的安装记录、manifest 及其声明的本地技能文件，
不会把插件 store/cache 当作通用来源，也不会安装、更新、移除或执行插件内容；
可写的技能包操作仍只通过带命令预览和确认的 `npx skills` 流程执行。

## 源码构建指引

前置要求：

- macOS 13 或更新版本。
- Xcode Command Line Tools。
- 带 Cargo 的 Rust toolchain。
- 带 Corepack/pnpm 的 Node.js。

构建并运行 macOS App：

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

## 重要文档导航

| 文件 | 用途 |
| --- | --- |
| `docs/README.md` | 文档索引与职责导航 |
| `docs/product-design.md` | 产品承诺、对象模型、信息架构、事实模型和验收契约 |
| `docs/implementation-sequence.md` | 强制工程任务顺序、边界、退出条件与验证要求 |
| `docs/architecture.md` | 仓库架构和代码职责边界 |
| `docs/data-model.md` | 持久化与临时数据概览 |
| `docs/adapters/agent-adapters.md` | 支持的 Agent 根目录、配置行为和 adapter 范围 |
| `docs/service-protocol.md` | App/服务集成的方法契约 |
| `docs/security-model.md` | 安全、隐私、凭据和本地数据规则 |
| `docs/ai-layer.md` | 可选 provider 工作流边界 |
| `docs/ui-delivery-standards.md` | 原生 UI、交互与实机验证标准 |
| `docs/runbooks/macos-app-runbook.md` | 本地 macOS 构建、运行和 smoke 验证流程 |
| `docs/runbooks/release-checklist.md` | 维护者发版前检查清单 |
| `AGENTS.md` | 本仓库的 coding agent 操作规则 |

## Contributing

欢迎贡献。为了让 review 更顺：

- 修改前先阅读 `AGENTS.md` 和相关文档。
- 产品重构必须按 `docs/implementation-sequence.md` 的编号顺序执行，任务状态只写在
  Issue 或 PR 中。
- 让改动聚焦在当前 App 功能和既有架构内。
- 如果服务行为变化，同步更新 fixtures 和 `docs/service-protocol.md`。
- 小改动运行聚焦检查；涉及 UI、协议或发版影响的改动运行 `pnpm check:macos`。
- 提交或推送改动前运行 `pnpm check:privacy`。
- 在 PR 或交接说明里写清楚运行过的命令。

## License

Agent Copilot 使用 [MIT License](LICENSE) 发布。
