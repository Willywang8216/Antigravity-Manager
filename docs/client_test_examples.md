# 稳定性与搜索功能：测试示例指南

为了验证近期对 API 400 错误及“搜索文件错误”的修复效果，您可以在 Claude CLI (Claude Code) 中运行以下指令进行实测。

## 1. 验证搜索工具自愈 (Grep/Glob Fix)

针对之前的 "Error searching files" 问题，这些指令将触发 `Grep` 和 `Glob` 工具调用，并验证参数映射是否正确。

### 测试指令示例
*   **指令 A**：`在当前目录中搜索包含 "fn handle_messages" 的 Rust 文件。`
    *   *验证点*：检查代理是否能正确将 `query` 映射为 `pattern`，并注入默认的 `path: "."`。
*   **指令 B**：`列出 src-tauri 目录下所有 .rs 文件。`
    *   *验证点*：验证 `Glob` 工具名是否被正确识别，且路径过滤逻辑正常。

---

## 2. 验证协议顺序与签名稳定性 (Thinking/Signature Fix)

针对之前的 `Found 'text'` 和 `Invalid signature` 400 错误。

### 测试指令示例
*   **指令 A（推理+搜索）**：`分析本项目中处理云端请求的核心逻辑，按调用顺序总结，并给出关键代码行的 Grep 搜索证据。`
    *   *验证点*：验证在“思维 -> 工具调用 -> 结果 -> 继续思维”循环中，块顺序是否正确。
*   **指令 B（历史记录重试）**：在长对话中频繁切换模型，观察系统是否在 400 报错时静默修复签名并重试。

---

## 附录：深度错误对照与修复方案

| 错误类别 | 具体报错特征码 (Error Detail) | 代理采取的修复/应对逻辑 |
| :--- | :--- | :--- |
| **消息流顺序违规** | `If an assistant message contains any thinking blocks... Found 'text'.` | **已修复**：`streaming.rs` 不再允许在文字块之后非法追加思维块。 |
| **思维签名不匹配** | `Invalid signature in thinking block` | **已修复**：优先保留原始名称以保护 Google 后端签名校验。 |
| **思维签名缺失** | `Function call is missing a thought_signature` | **已修复**：自动注入 `skip_thought_signature_validator` 占位符。 |
| **非法缓存标记** | `thinking.cache_control: Extra inputs are not permitted` | **已修复**：全局剔除历史消息中的 `cache_control` 标记。 |
| **Plan Mode 报错**| `EnterPlanMode tool call: InputValidationError: Extra inputs are not permitted` | **已修复**：`streaming.rs` 强制清空工具参数以符合官方无参协议。 |
| **连续 User 消息**| `Consecutive user messages are not allowed` | **已修复**：`merge_consecutive_messages` 自动合并相邻同角色消息。 |

---

## 3. 验证 Claude Code Plan Mode 与角色交替 (Issue #813)

针对 Plan Mode 切换导致的协议报错问题。

### A. 验证 Plan Mode 激活 (UI 状态)
*   **指令**：`进入 Plan Mode 调研 src-tauri 的目录结构。`
*   **预期结果**：
    *   终端左下角应立即出现蓝色的 **`plan mode on`** 标签。
    *   日志中应看到 `[Streaming] Tool Call: 'EnterPlanMode' Args: {}`。

### B. 验证角色交替自愈 (Consecutive Messages)
*   **指令**：`在 Plan Mode 下帮我分析 proxy/mappers/claude/request.rs 的逻辑，然后退出 Plan Mode 并给出一个简要总结。`
*   **预期结果**：
    *   模型切换模式（如从 Plan 到 Code）时不会因“连续两条 User 消息”而报 400 错误。
    *   日志中会体现 `merge_consecutive_messages` 的合并动作。

---

## 4. QuotaData 字段逻辑解析

设置页面中的“账号管理”列表，下方的进度条数据来源于 `QuotaData`。系统会在请求前检查账号配额，并在触及阈值时自动轮换。

---

## 调试建议
```bash
RUST_LOG=debug npm run tauri dev
```
在日志中搜索 `[Claude-Request]`，关注消息角色的排列顺序。
