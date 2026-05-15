# Project Needle 任务拆解（v0.1）

## 1. 文档目标

本文档将 PRD、架构设计和开发方案进一步细化为可执行任务，适合直接转成：

- GitHub Issues
- Jira / Linear / Trello 卡片
- Sprint Backlog
- 周计划 / 月计划

原则：
- 任务尽量独立
- 每项任务有明确输出
- 每个阶段都能产出可运行结果

---

## 2. 任务拆解方式

采用四级结构：

- **Epic**：大模块
- **Feature**：功能块
- **Task**：开发任务
- **Check**：验收检查点

建议编号规则：
- CORE-*：核心文本系统
- UI-*：界面与交互
- CMD-*：命令系统
- CFG-*：配置系统
- PLUGIN-*：插件系统
- PKG-*：包管理
- TEST-*：测试
- PERF-*：性能
- REL-*：发布与工程化

---

## 3. Epic：工程初始化

## Feature：仓库与基础工程

### Task REL-001 初始化仓库结构
**输出：** monorepo / workspace 基础目录
- 建立 `apps/`, `crates/`, `plugin_host/`, `docs/`, `tests/`, `scripts/`
- 配置 `.gitignore`
- 配置基础 README

### Task REL-002 建立 Rust workspace
**输出：** 可编译基础工程
- 创建 workspace 配置
- 创建 core/ui/services/package-manager 等 crate
- 统一 lint / format 配置

### Task REL-003 建立基础日志系统
**输出：** 统一日志接口
- 主进程日志
- plugin_host 日志
- 日志级别与输出格式

### Task REL-004 建立 CI 雏形
**输出：** 基础自动检查
- format check
- lint check
- unit test check

### Check
- 仓库可 clone 后一键构建
- CI 可跑通基础检查

---

## 4. Epic：Core 文本核心

## Feature：Buffer 与编辑模型

### Task CORE-001 设计 Buffer 接口
**输出：** `Buffer` trait / struct 草案
- 插入/删除/替换接口
- revision 管理
- 行列转换接口

### Task CORE-002 实现文本存储结构原型
**输出：** Rope 或 Piece Tree 初版
- 支持插入删除
- 支持大文本装载
- 支持截取 substr

### Task CORE-003 行索引缓存
**输出：** line index cache
- point -> row/col
- row/col -> point
- 增量更新缓存

### Task CORE-004 Undo/Redo 事务模型
**输出：** 基础编辑事务
- push edit op
- undo
- redo
- 合并连续编辑

### Task CORE-005 多选区模型
**输出：** `SelectionSet`
- 支持多 region
- 处理排序、重叠、合并
- 提供批量编辑坐标映射

### Task CORE-006 Buffer 快照与共享机制
**输出：** Buffer/View 解耦能力
- 同一 Buffer 可绑定多个 View
- 快照/版本号可查询

### Check
- 单元测试覆盖插入/删除/替换/undo/redo
- 100MB 文本加载通过 smoke test

---

## 5. Epic：UI 与渲染

## Feature：主窗口与编辑视图

### Task UI-001 建立主窗口壳
**输出：** 可打开应用窗口
- 主窗口初始化
- 空白编辑区
- 最小菜单/标题区（按选型实现）

### Task UI-002 文本绘制基础链路
**输出：** 文本可显示
- 可视区文本绘制
- 行号绘制
- 当前行高亮

### Task UI-003 光标与选区绘制
**输出：** 可见光标/选区
- 单光标
- 单选区
- 后续扩展多选区

### Task UI-004 滚动与可见区计算
**输出：** viewport 管理
- 垂直滚动
- 水平滚动（基础）
- 可见行计算

### Task UI-005 多标签页
**输出：** tabs 初版
- 新开文件标签
- 切换标签
- dirty 状态展示

### Task UI-006 Sidebar 文件树
**输出：** 项目文件树
- 展开/折叠目录
- 点击打开文件

### Task UI-007 Command Palette UI
**输出：** 命令面板界面
- 输入框
- 列表
- 键盘导航

### Check
- 可打开多个文件并切换
- 基础滚动平稳
- 选区显示正确

---

## 6. Epic：命令系统

## Feature：命令注册与执行

### Task CMD-001 设计命令抽象
**输出：** Command 接口
- app/window/view 三种上下文
- 参数对象结构

### Task CMD-002 命令注册中心
**输出：** registry
- 注册命令
- 查询命令
- 卸载命令

### Task CMD-003 命令执行器
**输出：** dispatcher
- 按上下文执行命令
- 参数校验
- 结果与错误返回

### Task CMD-004 命令状态查询
**输出：** enabled / visible / description
- 为菜单和命令面板提供能力

### Task CMD-005 内建命令首批实现
**输出：** 常用命令
- open_file
- save_file
- close_file
- undo
- redo
- find
- goto_line

### Check
- 命令可注册、查询、执行
- 命令面板能展示命令

---

## 7. Epic：配置系统

## Feature：Settings 管理

### Task CFG-001 设计设置层级
**输出：** 默认/用户/项目/视图层设计

### Task CFG-002 实现 Settings 存储
**输出：** Key-Value Settings 对象
- get/set/erase
- 序列化与反序列化

### Task CFG-003 配置文件加载器
**输出：** settings loader
- 读取 JSON / JSONC
- 错误处理

### Task CFG-004 热加载与广播
**输出：** change notification
- 配置变化监听
- 广播到 UI / Core / Plugin

### Task CFG-005 快捷键配置接入
**输出：** keymap loader
- 读取快捷键配置
- 绑定到命令系统

### Check
- 修改用户配置后可热生效
- 项目设置能覆盖默认值

---

## 8. Epic：搜索与项目能力

## Feature：文件搜索 / 内容搜索

### Task CORE-007 最近文件与最近项目
**输出：** recent manager

### Task UI-008 打开文件夹 / 项目
**输出：** project open flow

### Task SEARCH-001 模糊文件搜索
**输出：** fuzzy file finder
- 构建文件索引
- 提供查询接口

### Task SEARCH-002 当前文件查找
**输出：** find in file
- 正则/普通搜索基础支持

### Task SEARCH-003 项目内文本搜索
**输出：** grep-like service
- 后台搜索
- 支持取消
- 分批返回结果

### Check
- 中型项目文件搜索可用
- 搜索不会阻塞主线程

---

## 9. Epic：语法高亮

## Feature：基础语法系统

### Task SYNTAX-001 文件类型识别
**输出：** extension -> syntax mapping

### Task SYNTAX-002 Tokenizer 接口
**输出：** syntax service interface

### Task SYNTAX-003 主流语言基础支持
**输出：** JS/TS/Python/Go/Rust/JSON/Markdown/YAML/HTML/CSS/Shell

### Task SYNTAX-004 增量 token 更新
**输出：** 局部刷新机制

### Check
- 主流语言文件打开后能正确高亮基础结构
- 编辑时不会整文件重算导致卡顿

---

## 10. Epic：插件系统

## Feature：plugin_host 基础设施

### Task PLUGIN-001 建立 plugin_host 进程
**输出：** 可独立启动的 Python 宿主

### Task PLUGIN-002 建立 JSON-RPC 协议定义
**输出：** request/response/event schema
- 命令调用
- 事件派发
- 对象句柄访问

### Task PLUGIN-003 注入 `sublime` shim
**输出：** Python 模块雏形

### Task PLUGIN-004 注入 `sublime_plugin` shim
**输出：** 命令与监听器基类

### Task PLUGIN-005 包扫描器
**输出：** package discovery
- 扫描目录包
- 扫描 `.sublime-package`

### Task PLUGIN-006 插件加载器
**输出：** package import / reload
- 加载包内 Python 文件
- 捕获异常

### Task PLUGIN-007 主进程 Plugin Bridge
**输出：** bridge 层
- 维护 view/window/buffer 句柄
- 转发 RPC

### Task PLUGIN-008 命令注册同步
**输出：** 插件命令接入主命令系统

### Task PLUGIN-009 事件派发器
**输出：** EventListener 支持
- on_load/on_modified/on_save 等

### Task PLUGIN-010 插件日志与报错面板
**输出：** plugin console/panel

### Check
- 能运行最小 TextCommand 插件
- 插件异常不拖垮主程序
- 插件 reload 后命令状态正确

---

## 11. Epic：API 兼容实现

## Feature：Level 0 / Level 1 API

### Task API-001 实现 `Region`
**输出：** begin/end/size/contains/intersects 等

### Task API-002 实现 `Selection`
**输出：** clear/add/add_all/subtract/iter

### Task API-003 实现 `Settings`
**输出：** get/set/erase/has

### Task API-004 实现 `Window` 代理
**输出：** active_view/open_file/run_command 等

### Task API-005 实现 `View` 代理（读接口）
**输出：** size/substr/line/full_line/sel/settings/rowcol/text_point

### Task API-006 实现 `View` 代理（写接口）
**输出：** insert/erase/replace/run_command

### Task API-007 实现 `sublime.active_window`
**输出：** 当前窗口查询

### Task API-008 实现 `sublime.set_timeout`
**输出：** 基础调度

### Task API-009 实现 `sublime.load_settings`
**输出：** settings 加载

### Task API-010 实现 `status_message` / `error_message`
**输出：** 用户提示能力

### Check
- API smoke tests 通过
- 可支持至少 3 个简单插件

---

## 12. Epic：包管理

## Feature：安装与启停

### Task PKG-001 Package Registry
**输出：** 包注册表
- 包名
- 路径
- 状态
- 版本/元数据（基础）

### Task PKG-002 安装目录包
**输出：** package import flow

### Task PKG-003 安装 `.sublime-package`
**输出：** zip package flow

### Task PKG-004 启用/禁用包
**输出：** package state toggle

### Task PKG-005 卸载包
**输出：** uninstall flow

### Task PKG-006 Reload 机制
**输出：** reload package / reload host

### Check
- 包启停后命令列表更新正确
- 包卸载后状态干净

---

## 13. Epic：测试体系

## Feature：自动化测试

### Task TEST-001 Core 单元测试框架
**输出：** core tests scaffold

### Task TEST-002 命令系统测试
**输出：** command behavior tests

### Task TEST-003 API 兼容测试
**输出：** `sublime` / `sublime_plugin` tests

### Task TEST-004 样本插件 smoke test
**输出：** sample plugin runner

### Task TEST-005 大文件性能测试
**输出：** benchmark scripts

### Task TEST-006 事件顺序回归测试
**输出：** listener sequence tests

### Check
- 核心模块变更可自动回归
- 插件兼容性退化能被及早发现

---

## 14. Epic：性能优化

## Feature：性能观测与优化

### Task PERF-001 启动耗时埋点
**输出：** startup timing

### Task PERF-002 打开文件耗时埋点
**输出：** open file profiling

### Task PERF-003 滚动与输入延迟指标
**输出：** interaction metrics

### Task PERF-004 搜索服务 profiling
**输出：** search hotspot report

### Task PERF-005 plugin_host 调用开销分析
**输出：** RPC overhead metrics

### Check
- 有量化指标，不靠体感吹牛

---

## 15. Epic：发布与产品化

## Feature：Beta 交付准备

### Task REL-005 跨平台打包脚本
**输出：** macOS / Windows / Linux 包

### Task REL-006 用户文档
**输出：** 安装、配置、插件兼容说明

### Task REL-007 已知问题列表
**输出：** known issues 文档

### Task REL-008 兼容性矩阵
**输出：** supported / partial / fail 列表

### Task REL-009 崩溃与日志导出
**输出：** bug report support

### Check
- 外部用户可下载安装
- 遇到问题能给出足够诊断信息

---

## 16. 第一阶段 Sprint 建议（前 4 周）

## Sprint 1
### 目标
搭好骨架，不追功能多。

### 建议任务
- REL-001
- REL-002
- REL-003
- CORE-001
- UI-001
- CMD-001
- CFG-001

## Sprint 2
### 目标
打通最小文本编辑链路。

### 建议任务
- CORE-002
- CORE-003
- UI-002
- UI-003
- CMD-002
- CMD-003
- CFG-002

## Sprint 3
### 目标
具备基础可用性。

### 建议任务
- CORE-004
- UI-004
- UI-005
- CMD-005
- CFG-003
- SEARCH-002

## Sprint 4
### 目标
打通插件宿主第一条链路。

### 建议任务
- PLUGIN-001
- PLUGIN-002
- PLUGIN-003
- PLUGIN-004
- API-001
- API-007
- API-008

---

## 17. 建议优先级排序

如果资源极紧，优先顺序如下：

1. Buffer / View / Command Bus
2. 基础渲染与编辑
3. Undo/Redo / 多选区
4. plugin_host 通路
5. API Level 1
6. 文件搜索与项目能力
7. 包管理
8. 高级 UI / 高级 API

别一上来就想着 popup、phantom、minihtml，那是编辑器版的支线地狱。

---

## 18. 最终建议

这个项目成败，不取决于文档写得多漂亮，而取决于你能不能持续把任务拆到“本周能做完”。

所以执行上建议记住三句话：

1. **每周都要有可运行结果**
2. **每阶段都要有回归测试**
3. **每新增兼容能力，都要拿真实插件验收**

这样项目才不会慢慢长成一个优雅的烂尾楼。