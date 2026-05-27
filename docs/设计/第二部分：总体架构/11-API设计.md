# API 设计

---

## 一、是什么

Bugs 核心引擎暴露 REST + WebSocket 双层 API，供四种前端（TUI/Web/GUI/Android）调用。所有 API 都走本地端口或远程域名。

---

## 二、架构

```
┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐
│Web UI  │ │GUI桌面  │ │TUI终端  │ │Android   │
└───┬────┘ └───┬────┘ └───┬────┘ └────┬─────┘
    │          │          │           │
    └──────────┼──────────┼───────────┘
               │          │
      ┌────────▼──────────▼──────────┐
      │   REST API (同步操作)          │
      │   WebSocket API (实时推送)     │
      └──────────────────────────────┘
                     │
            ┌────────▼────────┐
            │  Bugs 核心引擎   │
            └─────────────────┘
```

---

## 三、REST API

| 模块 | 端点 | 操作 |
|:----|:----|:----|
| **对话** | `POST /api/chat` | 发送消息，返回 SSE 流式响应 |
| | `GET /api/chat/history?session=id` | 获取对话历史 |
| **场景** | `GET /api/scenes` | 列出场景 |
| | `POST /api/scenes/switch` | 切换场景 |
| | `GET /api/scenes/current` | 获取当前场景 |
| **记忆** | `GET /api/memory/search?q=...&scene=...` | 搜索记忆 |
| | `POST /api/memory/entry` | 手动写入记忆 |
| | `DELETE /api/memory/entry/:id` | 删除记忆 |
| | `GET /api/memory/trust/:id` | 获取某记忆的信任值 |
| **工具** | `GET /api/tools` | 列出可用工具 |
| | `POST /api/tools/:tool/call` | 手动调用工具 |
| **配置** | `GET /api/config` | 查看配置 |
| | `PUT /api/config` | 修改配置 |
| | `POST /api/config/reload` | 热重载配置 |
| **系统** | `GET /api/health` | 健康检查 |
| | `GET /api/status` | 系统状态（任务数/内存/执行数） |
| | `GET /api/executors` | Executor 列表和状态 |
| **模块** | `GET /api/plugins` | 列出已安装模块 |
| | `POST /api/plugins/install` | 安装模块 |

---

## 四、WebSocket API

| 事件 | 方向 | 说明 |
|:----|:----|:----|
| `chat:message` | C→S | 发送对话消息 |
| `chat:response` | S→C | 流式返回内容 |
| `task:status` | S→C | 任务状态变更通知 |
| `memory:updated` | S→C | 记忆更新通知 |
| `scene:changed` | S→C | 场景切换通知 |
| `executor:status` | S→C | Executor 状态变更 |
| `health:event` | S→C | 健康事件（警告/错误/恢复） |

---

## 五、认证

```
本地端口 (127.0.0.1)：自动信任，无需认证
远程域名：Token 认证
    Header: Authorization: Bearer <token>
    Token 在首次启动时自动生成，存于 ~/.bugs/auth.json
客户端错误认证：返回 401 + 固定重试间隔建议
```

---

## 七、错误响应

所有 API 统一错误格式：

```json
{
  "error": {
    "code": "TASK_TIMEOUT",
    "message": "任务超时：调度引擎在 30 秒内未收到结果",
    "detail": { "task_id": 42 }
  }
}
```

错误码规范：

| 类别 | 示例 | HTTP 状态码 |
|:----|:----|:---------:|
| 客户端错误（参数问题） | INVALID_ARGS | 400 |
| 认证错误 | UNAUTHORIZED | 401 |
| 权限不足 | FORBIDDEN | 403 |
| 资源不存在 | NOT_FOUND | 404 |
| 并发限制 | TOO_MANY_REQUESTS | 429 |
| 服务端错误 | INTERNAL_ERROR | 500 |
| 服务不可用 | SERVICE_UNAVAILABLE | 503 |

---

---

## 八、CLI 命令集（bugsctl）

```
bugsctl
├── 核心
│   ├── bugs start                   启动守护进程
│   ├── bugs stop                    停止守护进程
│   ├── bugs restart                 重启守护进程
│   ├── bugs status                  查看运行状态
│   └── bugs                         启动 TUI 交互界面
│
├── 子Agent
│   ├── bugs subagent list           列出所有子Agent
│   ├── bugs subagent kill <id>      强制终止子Agent
│   └── bugs subagent info <id>      查看子Agent详情
│
├── 场景
│   ├── bugs scene list              列出场景
│   ├── bugs scene switch <id>       切换场景
│   └── bugs scene create <name>     创建新场景
│
├── 记忆
│   ├── bugs memory search <q>       搜索记忆
│   ├── bugs memory compact <scene>  压缩场景记忆
│   └── bugs memory stats            记忆统计
│
├── 模块
│   ├── bugs plugin install <src>    安装模块
│   ├── bugs plugin list             列出模块
│   ├── bugs plugin enable <id>      启用模块
│   └── bugs plugin disable <id>     禁用模块
│
├── 配置
│   ├── bugs config get <key>        查看配置
│   ├── bugs config set <key> <val>  修改配置
│   └── bugs config reload           热重载配置
│
├── 运维
│   ├── bugs doctor                  全系统诊断
│   ├── bugs logs                    查看日志（最近 100 行）
│   ├── bugs logs --follow           实时日志流
│   ├── bugs logs --level error      按级别过滤
│   ├── bugs memory export <scene>   导出记忆
│   ├── bugs upgrade                 检查并安装新版本
│   └── bugs uninstall               清理卸载（需 --force 确认）
│
└── 全局
    ├── bugs --version               版本信息
    ├── bugs --help                  帮助信息
    └── bugs install                 安装指引
```

### 卸载流程

```
bugs uninstall
    ↓
警告："将永久删除以下内容：
       ~/.bugs/（所有配置、记忆、数据）
       Bugs 二进制文件"
    ↓
bugs uninstall --force
    ├── 停止守护进程
    ├── 备份所有数据到 ~/bugs-backup-YYYY-MM-DD.tar.zst
    ├── 删除 ~/.bugs/
    ├── 删除二进制文件
    └── 提示："Bugs 已卸载。备份保存在 ~/bugs-backup-..."
```

### 日志

```
日志位置: ~/.bugs/logs/
    bugs.log        → 当前日志
    bugs.1.log      → 轮转日志（默认 7 天轮转，保留 30 天）
    
日志级别: trace / debug / info / warn / error
    默认: info（生产环境）
    开发/调试: bugs 启动时加 --log-level debug
```

输出支持 `--json`，所有 `list`/`stats` 命令默认人类可读，`--json` 用于脚本。

---

## 十、安装与首次启动

### 安装方式

| 平台 | 方式 | 状态 |
|:----|:----|:---:|
| Linux/macOS/WSL2 | `curl -fsSL https://path/to/bugs/install.sh \| bash` | 主流程 |
| 备选 Shell | zsh/fish 适配（务必做好兼容再上线） | 后续 |
| Windows | PowerShell 脚本 | 暂时搁置 |
| Arch AUR | `yay -S bugs-bin` | 等 GUI 出来后做 |

### 首次启动流程（参考 OpenClaw onboard）

```
$ bugs
    ↓
检测 ~/.bugs/ 不存在 → 首次运行向导
    ↓
┌─────────────────────────────────────┐
│ ① 模型提供商                         │
│    [OpenAI] [第三方Provider] [Ollama本地]  │  ← 每步只选一项，不过载
│    每个选项旁有简短说明                 │
├─────────────────────────────────────┤
│ ② API Key                           │
│    [输入]  [跳过，稍后配置]            │  ← 允许跳过
├─────────────────────────────────────┤
│ ③ 工具套件                           │
│    [coding] 文件操作+Shell+浏览器      │
│    [general] 仅对话+知识检索          │
│    [minimal] 仅对话                  │
├─────────────────────────────────────┤
│ ④ 环境检测                           │
│    Chromium: ✅ 已安装                │  ← 自动检测
│    GPU: ✅ NVIDIA RTX 3060           │
│    内存: 16 GB                      │
├─────────────────────────────────────┤
│ ⑤ 创建第一个场景                     │
│    当前目录: /home/user/project-a/   │  ← 自动检测 pwd
│    [创建场景 "project-a"]  [跳过]    │
├─────────────────────────────────────┤
│ ⑥ 完成                              │
│    ~/.bugs/ 已自动生成               │
│    敲 bugs 即可开始对话               │
└─────────────────────────────────────┘
```

### 自动生成的内容

首次启动后无需编辑任何文件即可使用。用户后续可手动编辑：

| 文件 | 内容 | 可否编辑 |
|:----|:----|:------:|
| `~/.bugs/config.json` | 完整默认配置 | ✅ |
| `~/.bugs/BUG.md` | 默认 Agent 规则 | ✅ |
| `~/.bugs/IDENTITY.md` | 默认身份 | ✅ |
| `~/.bugs/auth.json` | 自动生成的 Token | ⚠️ 可改但建议不碰 |
| `~/.bugs/data/` | 空数据目录 | ❌ |

### 配置错误处理

```
配置格式错误 → 降级为默认值 → 告警通知 → 不死机
缺失可选字段 → 使用默认值 → 透明处理
缺失必选字段 → 使用默认值 → 引导用户补充
```

---

## 十一、假性能自检
