# Project Needle

Project Needle 是一款面向开发者的轻量、高性能、简洁优雅的代码编辑器。

它的目标不是做另一个臃肿的全家桶 IDE，而是做一个：

- **启动快**
- **编辑顺**
- **内存占用低**
- **键盘友好**
- **逐步兼容 Sublime Text 高价值插件生态**

> 当前项目仍处于 **早期 MVP 阶段**，架构已搭起，核心编辑能力和桌面 GUI 已跑通，但距离可长期日用还有不少工作要做。

---

## Why Needle?

很多开发者喜欢 Sublime Text 的原因很简单：

- 快
- 干净
- 不打扰
- 多光标和命令面板体验优秀

Project Needle 希望继承这种精神，并在现代 Rust 工程体系下重新构建：

- 更清晰的核心架构
- 更安全的内存模型
- 更可演进的插件兼容层
- 更适合长期维护的代码组织

我们不追求一开始就 100% 复刻 Sublime Text。
我们追求的是：**先做出一个轻、稳、快的编辑器，再逐步接上兼容层。**

---

## Current Status

当前仓库已经完成以下基础能力：

### 已完成
- Rust workspace 工程结构
- 核心编辑内核：
  - `Buffer`
  - `View`
  - `Selection / Region`
  - `AppState`
  - `CommandBus`
- transaction-based undo / redo
- 一批基础编辑命令：
  - 插入文本
  - 前删 / 后删
  - 左右移动
  - 行首 / 行尾移动
  - `select_all`
  - 插入前/后空行
- 基于 **egui / eframe** 的桌面 GUI MVP
- 文件操作：
  - 新建
  - 打开
  - 保存
  - 另存为
- 状态栏信息：
  - 修改状态
  - 光标行列
  - 字符数

### 当前还没完成
- 更细粒度的编辑同步
- 上下移动、按行复制/移动等更多编辑命令
- 项目视图 / 文件树 / 搜索
- 语法高亮
- plugin_host 与 Sublime API 兼容层
- 插件生态接入

---

## MVP Scope

当前 MVP 目标是先完成一个“真能用起来”的最小编辑器，而不是一开始就背上完整 IDE 包袱。

MVP 重点包括：

- 基础文件编辑
- 基础命令系统
- 多 selection / 光标模型基础
- undo / redo
- 桌面 GUI
- 为后续 Sublime 插件兼容层打基础

详细规划见：

- `docs/MVP_PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/DEVELOPMENT_PLAN.md`
- `docs/API_COMPATIBILITY.md`
- `docs/TASK_BREAKDOWN.md`

---

## Architecture Overview

Project Needle 当前采用分层设计：

### Core
负责编辑器内核能力：
- 文本存储
- 选区模型
- 命令分发
- 编辑事务
- undo / redo

### UI
负责桌面界面与交互：
- 顶部工具栏
- 文本编辑区
- 状态栏
- 文件打开/保存流程

### Plugin Host（预留）
未来会引入独立 Python 插件宿主，用于：
- 提供 `sublime` / `sublime_plugin` shim
- 对接 Sublime Text 高价值插件生态
- 隔离插件崩溃和异常

更多架构说明见：
- `docs/ARCHITECTURE.md`

---

## Tech Stack

当前技术栈：

- **Language:** Rust
- **Desktop GUI:** egui / eframe
- **File Dialog:** rfd
- **Logging:** tracing
- **Future Plugin Runtime:** Python plugin_host（规划中）

之所以先选 `egui/eframe`，不是因为它最终一定是终局方案，而是因为它能让 MVP 更快落地，不至于一开始就陷入 GUI 底层地狱。

---

## Repository Layout

```text
apps/
  desktop/              Desktop app entry

crates/
  core/                 Editor core
  ui/                   GUI layer
  services/             Future services layer
  plugin-bridge/        Future plugin bridge
  package-manager/      Future package manager
  syntax/               Future syntax/highlight support
  search/               Future search support
  platform/             Platform abstractions

plugin_host/
  bootstrap/            Future Python host bootstrap
  needle_sublime/       Future `sublime` shim
  needle_sublime_plugin/ Future `sublime_plugin` shim
  loader/               Future package loader

docs/
  MVP_PRD.md
  ARCHITECTURE.md
  DEVELOPMENT_PLAN.md
  API_COMPATIBILITY.md
  TASK_BREAKDOWN.md
  HANDOFF_2026-05-15_MVP.md
```

---

## Getting Started

### 1. Install Rust

如果本机还没有 Rust：

```bash
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
```

### 2. Check the project

```bash
cargo check
```

### 3. Run core tests

```bash
cargo test -p needle-core
```

### 4. Run the desktop app

```bash
cargo run -p needle-desktop
```

> 注意：如果你在无头环境、容器或没有图形桌面的机器上运行，GUI 启动可能失败。这不是项目逻辑 bug，更多是运行环境问题。

---

## What Works Today

如果你现在启动桌面应用，理论上应该能体验到：

- 新建空文件
- 打开本地文本文件
- 编辑文本
- 保存 / 另存为
- Undo / Redo
- Select All
- 行首 / 行尾移动
- 插入前后空行
- 查看状态栏信息

当前 GUI 编辑同步还是 MVP 级实现，适合验证链路，不适合拿它挑战几十万行巨型文件然后骂它 😄

---

## Roadmap

### Near Term
- 补齐基础编辑命令：
  - `move_up`
  - `move_down`
  - `select_line`
  - `duplicate_line`
- 增加快捷键支持
- 改进 GUI 与核心之间的编辑同步策略
- 引入更好的文本 diff / patch 方式

### Mid Term
- 文件树 / 项目视图
- 文件搜索 / 项目搜索
- 基础语法高亮
- 更完整的命令面板

### Long Term
- Python `plugin_host`
- Sublime API shim
- 包加载与生态兼容
- 高价值 Sublime 插件兼容矩阵

---

## Non-Goals (for now)

当前阶段明确 **不追求**：

- 完整 IDE 功能
- 完整 LSP / 调试器 / Git 图形面板
- 100% Sublime Text 行为复刻
- 一次性做完全部插件兼容

这些东西都很香，但一口吞下去通常也很容易噎死项目。

---

## Contributing

欢迎贡献，但当前项目还在快速演进中，建议先：

1. 阅读以下文档：
   - `docs/ARCHITECTURE.md`
   - `docs/DEVELOPMENT_PLAN.md`
   - `docs/TASK_BREAKDOWN.md`
2. 优先从核心命令、测试、GUI 细化、小型重构开始
3. 在大改动前先同步设计方向

如果你想贡献，特别欢迎这些方向：

- 编辑器核心命令
- undo/redo 事务模型
- 文件视图与项目视图
- 测试补充
- 文档改进
- plugin_host 原型

---

## Development Notes

常用命令：

```bash
cargo check
cargo test -p needle-core
cargo run -p needle-desktop
```

开发辅助脚本：

```bash
./scripts/dev.sh
```

---

## License

当前仓库默认使用：

**MIT**

如果后续需要改成双许可证或更严格策略，可以再调整。

---

## Vision

Project Needle 想做的，不只是“一个像 Sublime 的编辑器”。

更准确地说，它想做的是：

> 一个轻量、现代、可持续演进，并且愿意认真面对插件兼容问题的代码编辑器。

如果你也喜欢那种“工具应该快、干净、克制，但关键时刻非常可靠”的味道，那这个项目大概率会对你胃口。