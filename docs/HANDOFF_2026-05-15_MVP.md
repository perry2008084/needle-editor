# Project Needle 交接文档（2026-05-15）

## 1. 本次停点说明

本次停止原因：**已完成一个可编译、具备基础可用性的桌面 MVP**。

当前状态不再是“纯文档 + 内核骨架”，而是已经具备：

- Rust workspace 与项目结构
- 核心编辑内核（Buffer / View / Selection / Command Bus / AppState）
- 基础编辑命令
- transaction-based undo/redo
- 基于 egui/eframe 的桌面 GUI 外壳
- 打开/保存/另存为/新建文件
- 文本编辑与状态栏展示

继续往下做将进入 **MVP 之后的增强阶段**，包括：
- 更细粒度文本同步
- 更强的光标/选择语义
- 更完整的编辑命令
- 插件宿主接入
- 语法高亮与项目视图增强

因此此处适合作为一版阶段性交付点。

---

## 2. 当前已完成内容

## 2.1 项目结构

根目录：
- `~/project/needle-editor`

核心目录：
- `apps/desktop`：桌面应用入口
- `crates/core`：核心编辑内核
- `crates/ui`：桌面 GUI
- `plugin_host`：Python 插件宿主占位结构
- `docs`：PRD / 架构 / 开发计划 / 兼容清单 / 任务拆解 / 本交接文档

---

## 2.2 文档资产

已存在：
- `docs/MVP_PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/DEVELOPMENT_PLAN.md`
- `docs/API_COMPATIBILITY.md`
- `docs/TASK_BREAKDOWN.md`
- `docs/HANDOFF_2026-05-15_MVP.md`

---

## 2.3 核心内核能力

位于：`crates/core/src/`

已实现模块：
- `app.rs`
- `buffer.rs`
- `command.rs`
- `history.rs`
- `ids.rs`
- `selection.rs`
- `settings.rs`
- `state.rs`
- `view.rs`

### 已完成能力
- Buffer / View 分层
- Selection / Region 基础模型
- Settings 基础模型
- CommandBus 基础模型
- AppState 状态树
- 行列定位与行范围处理
- UTF-8 字符边界前后移动辅助
- selection-driven editing
- transaction-based undo/redo

---

## 2.4 已实现的基础编辑命令

内建命令位于：`crates/core/src/app.rs`

### 已支持
- `new_file`
- `insert_text`
- `delete_backward`
- `delete_forward`
- `move_left`
- `move_right`
- `move_to_beginning_of_line`
- `move_to_end_of_line`
- `insert_line_after`
- `insert_line_before`
- `select_all`
- `undo`
- `redo`
- `buffer_info`

说明：
- 编辑命令已支持空 selection 和非空 selection 两种情况
- undo/redo 已按 transaction 回退，不是单字符级地鼠战

---

## 2.5 桌面 GUI MVP

位于：
- `crates/ui/src/lib.rs`
- `apps/desktop/src/main.rs`

### GUI 框架选择
本次默认选型：**egui / eframe**

原因：
- Rust 生态成熟
- 足够快地拿到可用桌面 MVP
- 与当前 Rust 内核对接成本低
- 未来如果要切到自绘或更重的原生方案，不会把核心内核一起绑死

### 已完成功能
- 新建文件
- 打开本地文本文件
- 保存
- 另存为
- 多行文本编辑
- undo / redo
- select all
- 左右移动
- 行首 / 行尾移动
- 向上/向下插入空行（按钮触发）
- 状态栏显示：
  - 修改状态
  - 光标行列
  - 字符数

---

## 3. 验证结果

## 3.1 已通过

### 编译检查
已通过：
```bash
cargo check
```

### 核心测试
已通过：
```bash
cargo test -p needle-core
```

结果：
- 12 个测试全部通过

覆盖内容包括：
- buffer 替换
- 行范围
- 行列转换
- UTF-8 字符边界辅助
- selection 合并
- region caret 语义
- undo/redo 回环
- delete backward / forward
- line before / after
- line boundary movement
- 多 selection 替换

---

## 3.2 未完成的现场验证

### 桌面窗口运行验证
尝试执行：
```bash
cargo run -p needle-desktop
```

结果：
- 当前环境为无头环境
- 冷编译 GUI 依赖耗时较长，`timeout` 下未完成完整启动验证
- 因此目前**已确认可以编译通过，但未在当前环境完成可视化窗口实机验证**

这不是代码级 blocker，更像是运行环境限制。
在有桌面环境的机器上优先验证：
```bash
source ~/.cargo/env
cd ~/project/needle-editor
cargo run -p needle-desktop
```

---

## 4. 当前 MVP 的已知不足

以下不是 bug，而是当前 MVP 边界：

## 4.1 文本同步策略比较粗
GUI 文本区目前采用的是**整缓冲替换同步**：
- 用户在 GUI 中编辑时
- 当前实现会将整个 buffer 替换为新的文本内容

影响：
- undo 粒度偏粗
- 大文件性能不会太漂亮
- 与未来插件事件语义不完全一致

建议后续升级为：
- 细粒度 diff
- 或直接将 GUI 编辑操作映射到核心编辑命令

---

## 4.2 GUI 与核心 selection 同步仍是 MVP 级
当前已做：
- 从 egui cursor range 同步 selection

但还不够完整：
- 多光标 GUI 交互尚未真正打通
- 复杂 selection 扩展语义未实现
- 上下移动与列保持逻辑尚未实现

---

## 4.3 缺少这些典型编辑命令
建议下一阶段优先补：
- `move_up`
- `move_down`
- `duplicate_line`
- `select_line`
- `split_selection_into_lines`
- `move_lines_up`
- `move_lines_down`
- `delete_to_beginning_of_line`
- `delete_to_end_of_line`

---

## 4.4 插件宿主仍未打通
目前仅有目录结构与预留：
- `plugin_host/bootstrap`
- `plugin_host/needle_sublime`
- `plugin_host/needle_sublime_plugin`
- `plugin_host/loader`

尚未完成：
- Python plugin_host 启动器
- JSON-RPC 协议
- `sublime` / `sublime_plugin` shim
- 命令注册桥接
- 事件派发

这是 **MVP 之后最重要的下一阶段目标之一**。

---

## 5. 推荐的下一步路线（按优先级）

## 路线 A：补全编辑体验（推荐先做）
目标：让桌面 MVP 更像一个真正可日常试用的编辑器。

优先级建议：
1. `move_up / move_down`
2. `select_line`
3. `duplicate_line`
4. `delete_to_beginning_of_line / delete_to_end_of_line`
5. 更细粒度的 text diff 应用
6. GUI 快捷键（Ctrl/Cmd+S, Z, Y, A）

---

## 路线 B：打通 plugin_host 最小握手
目标：把“兼容 Sublime 生态”从文档承诺推进到工程现实。

建议顺序：
1. Python plugin_host 启动入口
2. Rust ↔ Python JSON-RPC 握手
3. `sublime.active_window()` 最小 stub
4. `TextCommand` 注册与调用
5. 样本插件 smoke test

---

## 路线 C：项目视图与搜索增强
目标：从“单文件编辑器”进化到“项目编辑器”。

建议项：
- Sidebar 文件树真正接上
- 模糊文件搜索
- 当前文件查找
- 项目内搜索
- 最近文件/最近项目

---

## 6. 关键设计决策记录

## 已做决策
### 1) MVP GUI 选 egui/eframe
理由：
- 落地快
- Rust 生态友好
- 不阻碍未来替换为更定制的 UI/rendering 方案

### 2) 内核先走自研 Buffer/View/State，而不是直接依赖 GUI 文本控件抽象
理由：
- 为后续插件兼容层留空间
- 为命令系统和 selection 模型打基础

### 3) undo/redo 先做 transaction 模型
理由：
- 多 selection 编辑必须按事务回退
- 这比以后返工省很多命

---

## 7. 如何继续接手

## 7.1 环境准备
Rust 环境已安装并配置：
- `rustup`
- `cargo`
- `rustfmt`
- `clippy`
- `rust-src`
- `rust-analyzer`

如果新 shell 没加载 PATH：
```bash
source ~/.cargo/env
```

---

## 7.2 常用命令

### 编译检查
```bash
cd ~/project/needle-editor
cargo check
```

### 核心测试
```bash
cargo test -p needle-core
```

### 运行桌面应用
```bash
cargo run -p needle-desktop
```

---

## 7.3 推荐接手顺序

### 如果目标是继续打磨 MVP：
1. 先在有桌面环境的机器上运行 `needle-desktop`
2. 人工验证打开/编辑/保存/撤销/重做
3. 补 `move_up / move_down / duplicate_line / select_line`
4. 加快捷键
5. 改进编辑同步粒度

### 如果目标是推进插件兼容：
1. 建 JSON-RPC schema
2. 写最小 Python plugin_host 启动脚本
3. 先实现 `sublime.active_window()`
4. 再实现 `TextCommand`

---

## 8. 本次完成度结论

### 结论
**Project Needle 已完成一个“可编译、带 GUI、具备基础文件编辑能力”的 MVP。**

### 仍需注意
- 当前尚未在本机图形环境里完成最终人工运行验证
- 当前 GUI 编辑同步仍偏粗
- 当前还不具备插件兼容能力

### 但这版已经足够作为：
- 第一阶段交付物
- 内测起点
- 后续插件系统接入基座
- 工程继续迭代的稳定落点

如果要一句话总结：

> 现在这项目已经从“想做个编辑器”走到了“这里有一个能继续长成编辑器的东西”。
