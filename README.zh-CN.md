# Agent Copilot

[English](README.md) | [简体中文](README.zh-CN.md)

Agent Copilot 是用于查看和管理本地 coding agent 数据的原生 macOS 工作台，
支持 Claude Code、Codex、opencode、Pi、Hermes 和 OpenClaw。

## 主要功能

- 在已记录的 Agent 根目录中查找本地技能。
- 浏览经过脱敏的本地会话摘要和消息。
- 搜索技能、会话、配置和详情页。
- 查看配置并执行受保护的操作。
- 通过明确的预览和确认流程管理本地技能包。
- 查看 Agent Copilot 可选 AI 功能的脱敏用量信号。
- 通过启动缓存和手动刷新缓存保持列表与导航流畅。

Agent Copilot 坚持 local-first，不添加云同步、账号、遥测或不受控网络访问。
凭据优先保存在 Keychain；技能脚本默认不执行；provider 调用必须由用户明确
查看并确认。

## 架构

- Rust workspace 负责产品逻辑、扫描、adapter、持久化和类型化服务协议。
- 原生 SwiftUI/AppKit shell 展示状态并发送类型化服务请求。
- Adapter 的读取和写入限制在已记录的本地根目录和受保护操作内。
- 已移除的 Tauri/React shell 不属于本项目。

## 构建

需要 macOS、Xcode Command Line Tools、Rust、Node.js、Corepack 和 pnpm。

```sh
git clone https://github.com/xjxtree/agent-copilot.git
cd agent-copilot
corepack enable
pnpm install
pnpm build:macos
open dist/AgentCopilot.app
```

开发时运行：

```sh
pnpm dev:macos
```

技能包管理操作还需要本机 Node/npm，因为它使用本地 `npx skills` 管理器。

## 文档

| 文件 | 用途 |
| --- | --- |
| `AGENTS.md` | Coding agent 的统一工作流和安全规则 |
| `docs/README.md` | 文档索引 |
| `docs/architecture.md` | 架构和职责 |
| `docs/data-model.md` | 持久化与临时数据 |
| `docs/adapters/agent-adapters.md` | Adapter 根目录和受保护操作 |
| `docs/service-protocol.md` | App/服务契约 |
| `docs/security-model.md` | 安全与隐私边界 |
| `docs/ui-delivery-standards.md` | 原生 UI 规范 |
| `docs/runbooks/macos-app-runbook.md` | 本地构建与运行命令 |

提交改动前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

## License

[MIT](LICENSE)
