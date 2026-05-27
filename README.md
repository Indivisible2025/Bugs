# 🧠 Bugs

> 像星空虫族一样高效协作的 AI Agent 运行时。
>
> Overmind 分裂任务 → 调度引擎管理 SubAgent → 子Agent 并行执行

[![License](https://img.shields.io/badge/license-AGPLv3%20%7C%20MIT-blue)]()
[![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange)]()

Bugs 是一个个人 AI Agent 运行时，名字取自《星船伞兵》中 Arachnids 的俗称。它运行在你自己的设备上，像星空虫族一样高效协作。

---

## 安装

### 方式一：GitHub（推荐）

```bash
# 稳定版（main 分支）
curl -fsSL https://raw.githubusercontent.com/Indivisible2025/Bugs/main/scripts/install.sh | sh

# 开发版（dev 分支）
curl -fsSL https://raw.githubusercontent.com/Indivisible2025/Bugs/main/scripts/install.sh | BUGS_CHANNEL=dev sh

# 测试版（beta 分支）
curl -fsSL https://raw.githubusercontent.com/Indivisible2025/Bugs/main/scripts/install.sh | BUGS_CHANNEL=beta sh
```

| 频道 | 分支 | 说明 |
|------|------|------|
| `stable`（默认） | `main` | 生产就绪，经过充分测试 |
| `beta` | `beta` | 新功能预览，基本稳定 |
| `dev` | `dev` | 最新代码，可能不稳定 |

### 方式二：镜像加速（暂不可用，网站未部署）

```bash
curl -fsSL https://bugs.neaneu.top/install.sh | sh
```

### 指定版本

```bash
curl -fsSL .../install.sh | BUGS_VERSION=v0.1.0 sh
```

---

`bugs` 是唯一的入口命令：

```bash
bugs              # 对话模式（自动启动守护进程）
bugs tui          # 终端界面（键盘操作 + Tab 切换面板）
bugs start|stop   # 管理守护进程
bugs status       # 查看运行状态
```

---

## 核心能力

- **Overmind + SubAgent** — 主Agent 理解意图、分裂任务，子Agent 并行执行
- **30+ AI Provider** — OpenAI · Anthropic · DeepSeek · MiniMax · Moonshot · Ollama 等
- **记忆系统** — 三层记忆（全局/概要/完整）+ 速率驱动信任引擎
- **冥想整理** — 后台自动去重、提炼经验、发现关联
- **三端界面** — TUI 终端 / WebUI 浏览器 / GUI 桌面
- **Chromium 浏览器模块** — 无头搜索 + 双池架构 + 拟人化反检测
- **微信渠道** — 扫码绑定，通过微信接收和回复消息
- **MCP + LSP** — Model Context Protocol 和 Language Server Protocol 支持
- **SKILL.md** — 兼容 OpenClaw 格式的技能系统

---

## 配置

```bash
export DEEPSEEK_API_KEY=sk-...     # DeepSeek（推荐）
export OPENAI_API_KEY=sk-...       # OpenAI
export ANTHROPIC_API_KEY=sk-ant-.. # Anthropic
export OLLAMA_BASE_URL=http://localhost:11434  # 本地 Ollama
```

---

## 文档

- [📖 开始使用](docs/说明/开始使用.md) — 安装 · 配置 · 启动 · 卸载
- [📐 设计总目录](docs/设计/00-目录.md) — 38章完整设计

## 许可

双许可证结构（OpenCore 模式）：

- **核心引擎**（`src/core` `src/api` `src/cli`） — [AGPLv3](LICENSE.md)：自由使用，SaaS 部署需开源修改
- **模块与界面**（`src/tui` `modules/*`） — [MIT](LICENSE.md)：零限制，商业友好，促进生态发展

详情见 [LICENSE.md](LICENSE.md)。
