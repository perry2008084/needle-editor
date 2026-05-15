# Project Needle 项目设置（基础版）

Project Needle 当前支持在项目目录下通过以下文件加载基础设置：

```text
<project-root>/.needle/settings.json
```

当前这套能力仍处于早期阶段，但已经支持：

- Sidebar 宽度
- 是否显示隐藏文件
- 自定义快捷键（基础版）

---

## 示例

```json
{
  "show_hidden_files": false,
  "sidebar_width": 260,
  "keybindings": [
    { "command": "show_command_palette", "key": "P", "modifiers": ["command", "shift"] },
    { "command": "show_find", "key": "F", "modifiers": ["command"] },
    { "command": "goto_line_panel", "key": "L", "modifiers": ["command"] },
    { "command": "save", "key": "S", "modifiers": ["command"] },
    { "command": "open_folder", "key": "O", "modifiers": ["command", "shift"] }
  ]
}
```

---

## 字段说明

### `show_hidden_files`
- 类型：`boolean`
- 默认值：`false`
- 作用：是否在 Sidebar 和 Quick Open 中显示以 `.` 开头的隐藏文件/目录

### `sidebar_width`
- 类型：`number`
- 默认值：`220`
- 作用：Sidebar 初始宽度
- 当前会被限制在 `140 ~ 480` 之间

### `keybindings`
- 类型：`array`
- 作用：定义额外的快捷键映射

每个快捷键对象结构：

```json
{
  "command": "show_find",
  "key": "F",
  "modifiers": ["command"]
}
```

支持字段：
- `command`: 命令名
- `key`: 键名（如 `A`、`P`、`Up`、`Down`）
- `modifiers`: 修饰键数组，当前支持：
  - `command`
  - `ctrl`
  - `shift`
  - `alt`
  - `option`

---

## 当前支持的 UI 级命令名

这些命令可直接用于 `keybindings`：

- `new_file`
- `open_file`
- `open_folder`
- `save`
- `save_as`
- `copy`
- `cut`
- `paste`
- `show_command_palette`
- `show_find`
- `show_project_search`
- `goto_line_panel`

此外，已经注册到核心命令系统的内建命令也可用，例如：

- `select_all`
- `select_line`
- `duplicate_line`
- `move_left`
- `move_right`
- `move_up`
- `move_down`
- `move_lines_up`
- `move_lines_down`
- `split_selection_into_lines`
- `goto_line`（注意：这个命令需要参数，当前更适合使用 `goto_line_panel`）

---

## 热重载

当前实现会在运行时检测 `settings.json` 的修改时间。

也就是说：

- 你修改项目设置文件后
- 编辑器会尝试自动重新加载

当前这套热重载还属于基础版：
- 没有复杂的冲突处理
- 没有设置错误面板
- 解析失败时会通过状态消息提示

---

## 当前限制

这套项目设置目前还是 MVP 级：

- 只有项目级设置，没有全局设置 UI
- keymap 还不支持复杂组合和参数化命令
- 没有完整 schema 校验
- 没有设置编辑器或设置管理界面

后续会逐步扩展。