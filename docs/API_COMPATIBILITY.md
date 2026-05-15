# Project Needle API 兼容清单（v0.1）

## 1. 文档目标

本文档定义 Project Needle 首版对 Sublime Text 插件 API 的兼容范围、优先级和实现策略。

目标不是一开始 100% 兼容，而是：

1. 优先兼容高价值插件的主路径
2. 为 shim 与主进程接口提供实现边界
3. 为后续兼容测试和兼容矩阵提供依据

---

## 2. 兼容策略总原则

### 2.1 原则
- **先主路径，后边角**
- **先命令与编辑能力，后复杂 UI**
- **先同步常见行为，后兼容历史怪癖**
- **先保证 API 存在与核心语义，后补完全等价行为**

### 2.2 分级策略

| 等级 | 目标 | 状态 |
|---|---|---|
| Level 0 | 包结构、配置、命令资源兼容 | MVP 必做 |
| Level 1 | 命令、View/Window、Settings、Region、Selection | MVP 必做 |
| Level 2 | EventListener、timeout、基础面板/交互 | MVP 后半段 |
| Level 3 | completion、quick panel、input panel、popup 子集 | Beta 前补齐 |
| Level 4 | 边角行为、冷门 API、历史兼容 | 后续迭代 |

---

## 3. 包与资源兼容（Level 0）

## 3.1 包目录结构
需要支持：
- `Packages/<PackageName>/...`
- `Installed Packages/*.sublime-package`
- `Packages/User/`

### 首版要求
- 目录包与压缩包都可识别
- User 包具有更高优先级
- 支持 package reload

---

## 3.2 资源文件类型
首版兼容以下资源概念：
- settings
- key bindings
- commands
- menus（基础）
- syntax（基础映射，不承诺全兼容）
- color scheme / theme（基础）

### 首版策略
- settings / commands / key bindings 优先实现
- menu、syntax、theme 做基础支持
- 不强求完整资源格式语义完全一致

---

## 4. `sublime` 模块兼容清单

## 4.1 MVP 必做（P0）

### 应用级函数

#### `sublime.active_window()`
**优先级：P0**
- 返回当前活动窗口代理
- 若无窗口则返回 `None`

#### `sublime.windows()`
**优先级：P1**
- 返回所有窗口列表
- 首版可先在单窗口模式下返回单元素列表

#### `sublime.set_timeout(fn, timeout_ms=0)`
**优先级：P0**
- 提供基础回调调度
- 回调在 plugin_host 事件循环中执行

#### `sublime.set_timeout_async(fn, timeout_ms=0)`
**优先级：P1**
- 后续支持异步任务分发
- MVP 可先退化到后台线程执行

#### `sublime.load_settings(name)`
**优先级：P0**
- 返回 Settings 对象
- 支持同名 settings 复用与监听基础能力

#### `sublime.save_settings(name)`
**优先级：P1**
- 持久化插件 settings
- User 设置改动写回文件

#### `sublime.status_message(msg)`
**优先级：P0**
- 在状态栏或底部临时显示消息

#### `sublime.error_message(msg)`
**优先级：P0**
- 显示错误提示
- 首版可退化为模态提示或 error panel

#### `sublime.message_dialog(msg)`
**优先级：P2**
- 简单提示框
- 非 MVP 核心路径

#### `sublime.ok_cancel_dialog(msg, ok_title=None)`
**优先级：P2**
- 基础确认框

---

## 4.2 视图/窗口查找函数

#### `sublime.find_resources(pattern)`
**优先级：P2**
- 用于扫描包资源
- 对包生态兼容很重要，但可延后

#### `sublime.load_resource(path)`
**优先级：P1**
- 加载包内资源文本

#### `sublime.load_binary_resource(path)`
**优先级：P3**
- 用于部分主题/图标/资源插件

---

## 4.3 常量与基础类型

#### `sublime.Region(a, b=None)`
**优先级：P0**
- 必做
- 提供 begin/end/size/empty/contains/intersects 等行为

#### `sublime.Edit`
**优先级：内部概念**
- 对插件显示为 TextCommand 的编辑上下文
- 实现上由主进程管理

#### 常见 flags / 常量
**优先级：P2**
- `DRAW_*`
- `ENCODED_POSITION`
- `TRANSIENT`
- `KEEP_OPEN_ON_FOCUS_LOST`

首版不必一次补全，但要为后续扩展保留命名空间。

---

## 5. `sublime_plugin` 模块兼容清单

## 5.1 命令基类（P0）

#### `ApplicationCommand`
- 支持 `run(**args)`
- 支持 `is_enabled()`
- 支持 `is_visible()`
- 支持 `description()``

#### `WindowCommand`
- 自动注入 window 对象
- 支持 `run(**args)`
- 支持 enabled / visible / description

#### `TextCommand`
- 自动注入 view 对象
- 支持 `run(edit, **args)`
- 支持标准命令调用链

### 实现要求
- 命令名称从类名自动转换，如 `MyCoolCommand -> my_cool`
- 注册与卸载要支持热重载
- 冲突命令要可诊断

---

## 5.2 事件监听器（P0/P1）

#### `EventListener`
首版优先支持：
- `on_new`
- `on_load`
- `on_modified`
- `on_pre_save`
- `on_post_save`
- `on_activated`
- `on_deactivated`
- `on_close`
- `on_selection_modified`

后续支持：
- `on_query_completions`
- `on_text_command`
- `on_window_command`
- `on_post_text_command`
- `on_post_window_command`
- `on_hover`
- `on_query_context`

### 实现要求
- 保证事件顺序可测
- 保证插件异常不会中断整条事件总线
- 支持事件过滤和视图上下文

---

## 6. Window API 兼容清单

## 6.1 MVP 必做

#### `window.active_view()`
**优先级：P0**

#### `window.views()`
**优先级：P1**

#### `window.new_file()`
**优先级：P1**

#### `window.open_file(path, flags=0, group=-1)`
**优先级：P0**
- 支持基础打开文件
- flags/group 可先部分忽略，但接口保留

#### `window.find_open_file(path)`
**优先级：P1**

#### `window.run_command(name, args=None)`
**优先级：P0**
- 命令链核心能力之一

#### `window.show_quick_panel(items, on_done, ...)`
**优先级：P2**
- Beta 前优先补齐

#### `window.show_input_panel(caption, initial_text, on_done, on_change, on_cancel)`
**优先级：P2**

#### `window.status_message(msg)`
**优先级：P1**

---

## 6.2 后续 API
- `window.project_data()`
- `window.set_project_data(data)`
- `window.folders()`
- `window.extract_variables()`
- `window.num_groups()`
- `window.focus_group(group)`
- `window.focus_view(view)`

这些对复杂插件重要，但首版可分批上。

---

## 7. View API 兼容清单

View 是兼容层的重灾区，也是高价值区。

## 7.1 MVP 必做（P0）

#### 基础信息
- `view.id()`
- `view.file_name()`
- `view.name()` / `view.set_name()`
- `view.buffer_id()`
- `view.window()`
- `view.is_dirty()`
- `view.is_read_only()` / `view.set_read_only()`
- `view.is_scratch()` / `view.set_scratch()`

#### 内容读取
- `view.size()`
- `view.substr(region)`
- `view.line(point)`
- `view.full_line(point)`
- `view.sel()`
- `view.settings()`

#### 坐标转换
- `view.rowcol(point)`
- `view.text_point(row, col)`

#### 编辑相关
- `view.insert(edit, point, text)`
- `view.erase(edit, region)`
- `view.replace(edit, region, text)`
- `view.run_command(name, args=None)`

#### 查找基础
- `view.find(pattern, start_pt, flags=0)`
- `view.find_all(pattern, flags=0, fmt=None, extractions=None)`

---

## 7.2 P1 建议支持
- `view.match_selector(point, selector)`
- `view.score_selector(point, selector)`
- `view.scope_name(point)`
- `view.syntax()` / `view.assign_syntax()`
- `view.set_status(key, value)`
- `view.erase_status(key)`
- `view.word(point)`
- `view.lines(region)`
- `view.viewport_position()`
- `view.set_viewport_position(pos, animate=True)`

---

## 7.3 P2/P3 后续支持
- `view.add_regions(...)`
- `view.get_regions(key)`
- `view.erase_regions(key)`
- `view.show(point_or_region, show_surrounds=True)`
- `view.show_popup(...)`
- `view.hide_popup()`
- `view.show_popup_menu(...)`
- `view.run_command` 的复杂上下文拦截
- completion 相关接口

---

## 8. Settings API

## 8.1 MVP 必做

#### `settings.get(key, default=None)`
**优先级：P0**

#### `settings.set(key, value)`
**优先级：P0**

#### `settings.erase(key)`
**优先级：P0**

#### `settings.has(key)`
**优先级：P1**

#### `settings.add_on_change(tag, callback)`
**优先级：P1**

#### `settings.clear_on_change(tag)`
**优先级：P1**

### 语义要求
- 支持监听器
- 支持按作用域返回不同 Settings 实例
- 写入后能触发必要广播

---

## 9. Selection API

## 9.1 MVP 必做
- `clear()`
- `add(region)`
- `add_all(regions)`
- `subtract(region)`
- 迭代访问
- 长度查询
- index 获取

### 要求
- Selection 内部按顺序维护
- 允许多选区
- 处理重叠/合并规则要与主编辑模型一致

---

## 10. Region API

## 10.1 MVP 必做
- `begin()`
- `end()`
- `size()`
- `empty()`
- `contains(x)`
- `intersects(region)`
- `cover(region)`
- `intersection(region)`
- 比较、排序、repr

Region 是插件兼容的地基，别小看这块，看着简单，炸起来很烦。

---

## 11. Command 相关行为兼容

## 11.1 命令命名
- 按 Sublime 规则将类名转 snake_case
- 例如：
  - `ExpandSelectionCommand -> expand_selection`
  - `MyHTTPCommand -> my_http`

## 11.2 调用链
- `app.run_command`
- `window.run_command`
- `view.run_command`

### 要求
- 统一走主进程 Command Bus
- 插件侧只保留代理与注册信息
- 参数必须 JSON-compatible

## 11.3 enabled / visible / description
这三件事很重要，很多命令面板和菜单插件靠它吃饭。

首版必须支持：
- `is_enabled()`
- `is_visible()`
- `description()`

---

## 12. 延后兼容项

以下内容不建议在 MVP 初期投入过多：
- `minihtml` 全量支持
- phantoms 全量支持
- 复杂 popup 生命周期
- 复杂 layout/group 管理
- 自定义 sheet / html sheet
- 完整 command interception 行为
- 冷门 UI flag 和平台相关差异

这些都是经典“看着不大，进去出不来”的坑。

---

## 13. 兼容测试建议

## 13.1 API 测试分层
1. 单个对象行为测试
2. 命令调用链测试
3. 事件顺序测试
4. 插件热重载测试
5. 样本插件 smoke test

## 13.2 样本插件类型建议
- 文本变换类
- 命令面板增强类
- 设置类
- 搜索/导航类
- 简单补全类

---

## 14. 首版实现优先级总结

## P0（必须）
- `sublime.active_window`
- `sublime.set_timeout`
- `sublime.load_settings`
- `sublime.status_message`
- `TextCommand / WindowCommand / ApplicationCommand`
- `EventListener` 常用事件
- `Window.run_command`
- `View` 基础读写与选择操作
- `Settings` 基础读写
- `Region` / `Selection`
- 包目录结构与基础资源兼容

## P1（建议首版后半段）
- `sublime.windows`
- `sublime.save_settings`
- `sublime.load_resource`
- `window.views`
- `window.new_file`
- `view.match_selector`
- `view.set_status`
- settings change listener

## P2（Beta 前）
- quick panel
- input panel
- completion
- add_regions / popup 基础子集
- find_resources
- 常量/flags 丰富化

## P3（后续）
- 更复杂 UI API
- 冷门历史行为
- 更完整 Package Control 交互细节

---

## 15. 最终建议

不要一开始追着 API 数量跑，应该追着这三个指标跑：

1. **有多少高价值插件能跑**
2. **命令和编辑语义是否稳定**
3. **事件顺序是否靠谱**

API 名字补齐不难，行为补齐才要命。先把命保住。