# Bugs v2026.05.27-1-1--dev-overmind-1

首个开发版本。版本号详情见 [版本控制](../../docs/设计/第六部分：技术决策/32-版本控制.md)。

## 状态

**dev** — 未经长时间稳定性测试，API 和模块可能在后续版本变更。

## 二进制

| 文件 | 大小 | 说明 |
|:----|:---:|:----|
| bugs | 2.3MB | 主命令（对话 + 守护进程管理 + 子Agent派遣） |
| bugs-daemon | 4.5MB | API 守护进程（所有前端的统一后端） |
| bugs-tui | 2.4MB | TUI 终端界面（四面板 + Tab 切换） |

## 使用

```bash
chmod +x bugs bugs-daemon bugs-tui
export DEEPSEEK_API_KEY=sk-...
./bugs start       # 启动守护进程
./bugs tui          # 打开 TUI
./bugs              # 对话模式
```

## 构建信息

Rust 1.95.0 · LTO · strip
