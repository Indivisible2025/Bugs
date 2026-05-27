---
name: code-review
description: "审查 Rust 代码，找出潜在问题"
requires: ["read", "write", "exec"]
os: ["linux", "macos"]
---
# Code Review 技能

## 当你被要求审查 Rust 代码时：

1. 使用 `read` 读取目标文件
2. 检查以下常见问题：
   - 不必要的 `clone()` 调用
   - 可以合并的 `match` 分支
   - 未使用的变量和导入
   - 性能问题（如使用 `Rc<RefCell<T>>` 可用 `Arc<Mutex<T>>` 替代）
3. 使用 `exec` 运行 `cargo check` 验证
4. 输出审查结果
