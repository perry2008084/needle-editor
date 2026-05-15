# Project Needle 架构设计（v0.1）

## 1. 文档目标

本文档定义 Project Needle 的首版技术架构，目标是支撑以下产品方向：

- 类似 Sublime Text 的轻量、快速编辑体验
- 界面简洁优雅
- 小内存占用
- 对 Sublime Text 插件生态提供高价值兼容
- 后续可扩展为长期维护的跨平台桌面编辑器

本文档重点解决三个问题：

1. 编辑器核心如何做到快、稳、轻
2. 插件系统如何兼容 Sublime Text 生态
3. 如何通过分层设计降低后续返工成本

---

## 2. 设计原则

### 2.1 核心原则

1. **核心能力优先于花哨功能**
   - 优先保证文本编辑、渲染、搜索、命令系统的性能与稳定性。

2. **插件系统与主进程强隔离**
   - 插件崩溃不能拖垮主编辑器。
   - 插件行为通过兼容层注入，不污染内核。

3. **兼容 Sublime，但不复制历史包袱**
   - 对外提供相似 API 和包结构。
   - 内部实现按更现代架构设计。

4. **异步化与增量化优先**
   - 高亮、索引、全局搜索、插件事件等尽量异步。
   - 渲染、布局、分析任务尽量增量执行。

5. **跨平台但不牺牲体验**
   - Linux/macOS/Windows 从架构层就统一抽象。
   - 平台差异收敛在边界模块。

---

## 3. 总体架构

采用以下进程与模块划分：

```text
+-------------------------------------------------------------+
|                        Needle App                           |
|                                                             |
|  +--------------------+    +----------------------------+   |
|  |   UI Layer         |    |   Core Layer               |   |
|  |--------------------|    |----------------------------|   |
|  | Window Manager     |    | Buffer / Rope Engine       |   |
|  | Tabs / Sidebar     |    | Selection / Cursor Model   |   |
|  | Command Palette    |    | Undo / Redo                |   |
|  | Panels / Popups    |    | Command Bus                |   |
|  | Theme Renderer     |    | Settings / Project Model   |   |
|  +--------------------+    +----------------------------+   |
|             |                          |                    |
|             +------------+-------------+                    |
|                          |                                  |
|                 +--------v--------+                         |
|                 |  Service Layer  |                         |
|                 |-----------------|                         |
|                 | Syntax Engine   |                         |
|                 | Search / Index  |                         |
|                 | File Watcher    |                         |
|                 | Package Manager |                         |
|                 | Plugin Bridge   |                         |
|                 +--------+--------+                         |
+--------------------------|----------------------------------+
                           |
                      IPC / RPC
                           |
+--------------------------v----------------------------------+
|                      plugin_host                            |
|-------------------------------------------------------------|
| Embedded CPython                                             |
| sublime shim                                                 |
| sublime_plugin shim                                          |
| Package Loader                                               |
| Event Dispatcher                                             |
| Command Registration                                         |
+-------------------------------------------------------------+
```

---

## 4. 模块分层

## 4.1 UI Layer

### 目标
负责一切与视觉展示和交互相关的内容，但不承担业务逻辑主导。

### 子模块

#### Window Manager
- 顶层窗口生命周期
- 多窗口管理
- 焦点窗口维护
- DPI/缩放适配

#### Editor View
- 文本布局结果消费
- 滚动区域渲染
- 光标与选区绘制
- 行号、 gutter、当前行高亮

#### Tabs / Sidebar
- 标签页管理
- 项目树显示
- 文件状态标记（dirty、readonly 等）

#### Command Palette
- 模糊检索命令
- 动态展示内建命令与插件命令
- 支持延迟加载与分组

#### Panels / Popups
- 查找替换面板
- 快速输入面板
- 输出面板
- 后续扩展 quick panel / popup / phantom

#### Theme Renderer
- UI 主题色
- 编辑区配色
- 图标与视觉资源绑定

### 设计要求
- UI 不直接改 Buffer，统一通过 Command Bus 发命令
- UI 不直接操作插件进程，统一通过 Plugin Bridge
- UI 层保持薄，避免逻辑堆积成意大利面

---

## 4.2 Core Layer

这是系统的心脏，不快就全白搭。

### 子模块

#### Buffer Engine
负责文本存储与编辑。

建议方案：
- 首版采用 **Rope** 或 **Piece Tree**
- 要求支持：
  - 大文件
  - 高频插入删除
  - 多光标批量编辑
  - 增量 offset 映射
  - 快照/历史记录支持

#### Selection / Cursor Model
- 多选区支持
- 插入点与选区统一模型
- 支持方向性 selection
- 提供批量编辑前后的坐标映射能力

#### Undo / Redo Engine
- 命令级事务模型
- 支持合并编辑操作
- 支持插件触发编辑也纳入事务
- 支持未来的宏录制

#### Command Bus
- 内建命令统一注册
- UI 与插件命令统一调度
- 执行路径可追踪、可日志化
- 支持前置校验：enabled / visible / description

#### Settings System
- 用户级设置
- 项目级设置
- 视图局部设置
- 合并优先级规则
- 热加载广播

#### Project Model
- 打开目录/工作区
- 最近项目
- 排除规则
- 索引边界

---

## 4.3 Service Layer

Service Layer 负责把耗时和辅助能力从 Core 中拆出去，避免核心过胖。

### Syntax Engine
- 基础词法高亮
- 增量 token 更新
- 后续兼容 Sublime syntax 规则的一部分
- 允许异步解析并回送渲染结果

### Search Service
- 当前文件搜索
- 项目内文本搜索
- 模糊文件搜索
- 支持中断、分页、增量刷新

### Symbol / Index Service（后续）
- 文件符号提取
- 项目符号索引
- Goto Symbol / Goto Anything 支撑

### File Watcher
- 监控外部文件变更
- 监控 package 目录变更
- 通知视图刷新、插件重载、索引刷新

### Package Manager
- 扫描包目录
- 安装/启用/禁用/卸载 package
- 解析 metadata
- 维护 package 状态

### Plugin Bridge
- 主进程与 plugin_host 的桥
- 对外提供对象句柄映射
- 负责调用转发、事件派发、命令注册同步

---

## 5. 插件架构设计

## 5.1 设计目标

插件系统必须满足：

1. 对插件作者尽量像 Sublime Text
2. 对内核尽量像一个边界清晰的外部服务
3. 插件崩溃、卡死、异常尽量隔离
4. 支持后续逐步扩展兼容 API

---

## 5.2 进程模型

采用独立进程：`plugin_host`

### 原因
- Python 崩溃不拖死 UI
- 插件可单独重启
- 插件性能问题更易诊断
- 未来可支持沙箱策略与资源配额

### 进程职责

#### 主进程负责
- 真正的 View/Window/Buffer 数据
- 渲染与 UI
- 文件系统访问主流程
- 命令最终执行
- settings/project/package 状态源

#### plugin_host 负责
- 加载 Python 包
- 暴露 `sublime` / `sublime_plugin`
- 派发插件事件
- 执行 Python 插件逻辑
- 回调主进程完成编辑器操作

---

## 5.3 API 兼容层

### 兼容思路
在 plugin_host 内提供 shim：

- `sublime`
- `sublime_plugin`

插件调用这些 API 时，本质是调用 RPC 代理。

### 示例
```python
view.run_command("expand_selection", {"to": "word"})
```

在内部会变成：
1. plugin_host 中的 ViewProxy 收到调用
2. 转换为 RPC 请求
3. 主进程 Command Bus 执行
4. 结果返回 plugin_host

### 核心代理对象
- WindowProxy
- ViewProxy
- SettingsProxy
- Region
- SelectionProxy

### 兼容分级

#### Level 1
- 命令注册与执行
- 基础 View/Window 操作
- settings
- Region / Selection

#### Level 2
- EventListener
- set_timeout / async callback
- command palette / menu / key bindings

#### Level 3
- quick panel
- input panel
- completion API
- popup / phantom / minihtml 子集

#### Level 4
- 稀有历史行为和 undocumented edge cases

---

## 5.4 事件系统

### 主事件类型
- on_new
- on_load
- on_pre_save
- on_post_save
- on_modified
- on_activated
- on_deactivated
- on_close
- on_selection_modified

### 设计要求
- 事件需带上下文对象句柄
- 事件顺序可测
- 事件默认串行派发，避免并发踩状态
- 对耗时插件事件提供诊断日志

### 风险点
- 事件顺序与 Sublime 不一致会导致兼容性问题
- 因此必须建立事件回归测试

---

## 6. 数据模型设计

## 6.1 Buffer

建议字段：
- buffer_id
- content storage
- revision
- encoding
- line index cache
- dirty flag
- readonly flag
- associated file path

### 能力要求
- O(log n) 级别插入删除
- 快速 line/column 转换
- 低成本快照
- 支持多个 View 共享同一 Buffer

---

## 6.2 View

View 是 Buffer 的可视化实例。

### 字段建议
- view_id
- buffer_id
- selections
- viewport
- syntax
- local settings
- transient / scratch 状态

### 原则
- Buffer 管文本，View 管显示与交互状态
- 一个 Buffer 可对应多个 View

---

## 6.3 Window

### 字段建议
- window_id
- open views
- active view
- layout info
- project context
- quick panel / input panel 状态

---

## 7. 命令系统设计

## 7.1 命令分类
- ApplicationCommand
- WindowCommand
- TextCommand
- InternalCommand（仅内部使用）

## 7.2 命令生命周期
1. 注册
2. 查询可见/可用
3. 参数校验
4. 执行
5. 写入 undo transaction（如涉及编辑）
6. 触发相关事件

## 7.3 设计要求
- 参数采用统一结构（JSON-compatible）
- 支持命令描述信息
- 支持插件动态注册和注销
- 支持未来命令日志与 profiling

---

## 8. 配置系统设计

## 8.1 配置层级
1. 默认配置
2. 用户全局配置
3. 项目配置
4. 视图局部配置
5. 临时运行态覆盖

## 8.2 文件格式
- 首版推荐 JSON / JSONC
- 对外尽量靠近 Sublime 配置习惯

## 8.3 热加载机制
- 文件变更监听
- 配置解析
- 差异对比
- 广播给订阅模块

---

## 9. 渲染与性能设计

## 9.1 渲染原则
- 只渲染可见区域与少量预取区域
- 布局缓存与 token 缓存分离
- 光标、选区、文本分层绘制
- 变更采用局部重绘

## 9.2 必做优化
- viewport virtualization
- 行布局缓存
- 字体度量缓存
- token 增量刷新
- 滚动时避免整屏重算

## 9.3 重点指标
- 输入延迟
- 滚动帧率
- 大文件打开耗时
- 内存峰值
- 命令面板响应耗时

---

## 10. 包管理与生态接入

## 10.1 Package 目录建议

```text
needle-editor/
  packages/
    Default/
    User/
    Package Control/
    SomePackage/
  installed_packages/
    SomePackage.sublime-package
```

### 兼容目标
- 支持目录包与压缩包并存
- 支持 User 包特殊优先级
- 支持 package metadata

## 10.2 包状态机
- discovered
- loaded
- failed
- disabled
- reloading
- removed

## 10.3 安装流程
1. 下载包
2. 校验
3. 解压或写入 installed_packages
4. 刷新 package registry
5. plugin_host reload
6. 命令与菜单重新同步

---

## 11. 并发模型

## 11.1 主线程职责
- UI 事件循环
- 视图更新提交
- 核心状态变更提交

## 11.2 后台线程职责
- 文件索引
- 文本搜索
- 语法分析
- 包扫描
- 某些 IO 操作

## 11.3 线程安全策略
- Core 状态写操作集中化
- 跨线程通信通过 message passing
- 避免 UI 线程共享可变对象

---

## 12. 错误处理与诊断

## 12.1 错误分级
- Fatal：主进程不可恢复错误
- Recoverable：单功能失败但整体可继续
- Plugin Error：插件异常
- User Error：配置或包错误

## 12.2 诊断能力
- plugin_host 日志
- 命令执行日志
- 性能采样点
- package load report
- 崩溃恢复日志

## 12.3 插件异常策略
- 单包加载失败不影响其他包
- 插件报错显示在独立 console panel
- 支持手动 reload plugin host

---

## 13. 测试策略

## 13.1 测试层次
1. Core 单元测试
2. Command/Selection 行为测试
3. 渲染快照测试（可选）
4. 插件 API 兼容测试
5. 样本插件 smoke test
6. 性能回归测试

## 13.2 重点测试对象
- 多光标编辑
- undo/redo 边界
- Buffer/View 映射
- 事件顺序
- package reload
- 大文件滚动与搜索

---

## 14. 首版技术决策建议

### 推荐组合
- 语言：Rust
- UI：先成熟 GUI 壳，后续再替换/增强为自绘
- 插件宿主：CPython + 独立进程
- IPC：JSON-RPC
- 配置：JSON/JSONC
- 文本存储：Rope / Piece Tree

### 原因
- 先把架构做对，再追逐终极性能
- 先保命，再装酷。装酷可以 later，返工不行

---

## 15. 首版不做的设计

以下内容明确不在 v0.1 强制范围内：
- 完整 LSP 客户端
- 调试器集成
- Git 图形面板大全套
- 多人协作
- 100% Sublime 插件兼容
- 复杂 minihtml 全量实现

---

## 16. 下一步落地建议

建议立刻推进以下三件事：

1. 定义 Core 数据结构草案
2. 定义 Level 1 / Level 2 API 兼容清单
3. 做 `Buffer + View + Command Bus + plugin_host stub` 的端到端原型

如果这三件打通，这项目就从 PPT 工程，进化成了真工程。