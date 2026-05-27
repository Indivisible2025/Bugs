# OpenClaw 借鉴分析

基于 OpenClaw GitHub 主仓库（2026.5.26）的架构分析。

---

## 可直接吸收的设计

### 1. BUG.md 分层注入

OpenClaw 在项目每个子目录放 AGENTS.md，Agent 在不同子目录工作时读取不同规则。

| OpenClaw | Bugs 怎么用 |
|:---------|:----------|
| 根 `AGENTS.md` — 全局硬规则 | `~/.bugs/BUG.md` |
| `src/agents/AGENTS.md` — agent 子系统规则 | `~/.bugs/scenes/<scene>/BUG.md` |
| `src/channels/AGENTS.md` — 渠道规则 | 场景覆盖层 |
| `src/gateway/AGENTS.md` — 网关规则 | 模块级配置 |

Bugs 已有三层覆盖（全局→场景→本地），可以扩展为"按目录检测 BUG.md 自动分层注入"。

### 2. 模块能力注册模型

OpenClaw 定义了明确的能力类型和注册方式：

| 能力 | 注册方式 |
|:----|:----|
| 文本推理 | `api.registerProvider(...)` |
| 语音 | `api.registerSpeechProvider(...)` |
| 实时转录 | `api.registerRealtimeTranscriptionProvider(...)` |
| Web 搜索 | `api.registerWebSearchProvider(...)` |
| 频道/消息 | `api.registerChannel(...)` |

Bugs 应该改为类似的能力注册模型：

```rust
pub trait ModuleCapability: Send + Sync {}
pub trait TextInference: ModuleCapability {}
pub trait SpeechSynthesis: ModuleCapability {}
pub trait WebSearch: ModuleCapability {}
// ... etc
```

每个模块声明自己实现什么能力，核心保持模块无感知。

### 3. 核心保持模块无感知

> "Core stays plugin-agnostic. No bundled ids/defaults/policy in core when manifest/registry/capability contracts work."
> — OpenClaw

Bugs 已经朝这个方向走——浏览器模块是第一个内置模块，核心不依赖浏览器。但可以更彻底地抽象——所有模块通过能力注册表调用，核心永远不 import 模块代码。

### 4. 模块形状

| 形状 | 含义 | Bugs 对应 |
|:----|:----|:--------|
| `plain-capability` | 只注册一种能力 | TTS 模块、STT 模块 |
| `hybrid-capability` | 注册多种能力 | 浏览器模块（Web搜索 + Web抓取） |

### 5. 架构原则

直接复用的原则：

| 原则 | 含义 |
|:----|:----|
| "One canonical path. Delete the old path" | 重构不要留 shim 和 fallback |
| "Compatibility is opt-in" | 不是默认兼容旧配置，需要显式标记 |
| "Hot paths should carry prepared facts forward" | 热点路径不重复查询，提前准备好 |
| "Fix shape: clean bounded refactor, not smallest patch" | 修 Bug 时重构干净，不贴膏药 |

---

## 不需要吸收的

| OpenClaw 设计 | 不吸收原因 |
|:------------|:---------|
| TypeScript 运行时 | Bugs 用 Rust |
| Plugin SDK (npm) | Bugs 模块用 Rust trait |
| ClawHub 分发 | Bugs 有独立的生态分发设计 |

---

## 下一步行动

1. ~~模块能力注册模型~~ — 留到模块 SDK 设计时做
2. ~~BUG.md 目录检测注入~~ — 留到场景切换增强时做
3. ✅ 架构原则写入 02-设计哲学.md — 立即做
