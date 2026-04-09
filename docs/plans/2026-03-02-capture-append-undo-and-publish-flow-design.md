# FlashNote 追加输入 / 撤销 / 发布流程设计（2026-03-02）

## 目标

本轮聚焦三个即时问题与一个流程建设目标：

1. 在同一 topic 的连续思考场景下，支持“追加上一条想法”，避免每次都生成独立代码块。
2. 确保 `Cmd+Z` 在捕捉输入时稳定可用。
3. 解决 `Cmd+Enter` 偶发把组合键泄漏到其他应用（Obsidian/Claude）的问题。
4. 跑通 `Obsidian + Agent 生成 + note2mp 发布` 流程，并提供 Anthropic 风格样式对齐改造路径。

## 交互与格式决策

- 发送新条目：`Cmd+Enter`
- 追加上一条：`Cmd+Shift+Enter`
- 不新增 GUI，不新增显式模式切换按钮。

追加格式采用“同一代码块内分段追加”：

```markdown
```
first idea
2026-03-02 11:34
---
second idea
2026-03-02 11:48
```
```

这样同时满足：
- 同 topic 连续输入保持在一条记录内。
- 每次追加仍保留独立时间戳，便于回溯。
- 向后兼容旧数据（旧条目不需要迁移）。

## 实现边界

### FlashNote（本轮已落地）

- `DailyNoteWriter` 增加 `DailyNoteSaveMode`：`createNewEntry / appendToLatestEntry`。
- `appendToLatestEntry` 逻辑：在同分区内寻找最新代码块并在闭合 fence 前插入分段；找不到则自动回退为新建。
- 快捷键映射：
  - `Cmd+Enter` -> 新建发送
  - `Cmd+Shift+Enter` -> 追加发送
- `CaptureTextEditor` 显式开启 `allowsUndo`，确保 `Cmd+Z` 可用。
- 捕捉面板去除 `nonactivatingPanel`，减少按键事件穿透到底层应用。
- 保存后恢复前台应用增加轻微延迟，进一步规避组合键尾事件干扰。

### Obsidian 发布流（下一轮）

- 引入 `note2mp` 作为“样式保真复制层”：Agent 输出 Markdown -> Obsidian 润色 -> note2mp 复制到公众号编辑器。
- 新增 Anthropic 风格映射层：
  - 标题层级规范
  - 引用块与分割线规则
  - 代码块主题、行号、背景色
  - 链接脚注展示策略（适配公众号不支持超链接）

## 多 Agent 并行执行拆解

1. Agent A（Capture Core）
- 负责输入行为：追加模式、快捷键路由、窗口焦点恢复。
- 产出：核心功能 PR + 手工验证记录。

2. Agent B（Writer & Parser）
- 负责 Markdown 写入策略：分区定位、代码块定位、回退策略健壮性。
- 产出：写入逻辑单测与边界测试。

3. Agent C（Publish Pipeline）
- 负责 `note2mp` 接入与 Anthropic 风格模板定义。
- 产出：发布模板、示例文章、从 Obsidian 到公众号的 SOP。

4. Agent D（Release QA）
- 负责回归清单：热键冲突、IME、多应用切换、撤销/重做、Pin 模式。
- 产出：验收报告与失败样例复现步骤。

## 验收标准

- 同分区中，`Cmd+Shift+Enter` 连续两次后只保留一个代码块。
- 在无可追加代码块时，自动新建条目且不报错。
- `Cmd+Z` 在输入区可撤销最近编辑。
- `Cmd+Enter` 不再触发底层应用可见行为（重点回归 Obsidian / Claude）。
- 发布链路可以在 10 分钟内完成一次“写作 -> 转换 -> 复制到公众号编辑器”。
