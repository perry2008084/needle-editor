# Contributing to Project Needle

感谢你愿意为 Project Needle 做贡献。

当前项目仍处于早期阶段，架构和功能都在快速演进，所以我们更看重：

- 小而清晰的改动
- 先讨论再大改
- 配套测试与文档
- 尊重项目当前的技术方向

---

## Before You Start

建议先阅读：

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/DEVELOPMENT_PLAN.md`
- `docs/TASK_BREAKDOWN.md`
- `docs/API_COMPATIBILITY.md`

如果你的改动涉及较大架构调整，建议先开 issue 或讨论再开始动手。

---

## Ways to Contribute

欢迎以下类型的贡献：

- 修复 bug
- 改进文档
- 补充测试
- 改进核心编辑命令
- 改进 GUI 交互
- 提升性能
- 推进 plugin_host / Sublime 兼容层原型

特别欢迎这几类“高价值、小步快跑”的贡献：

- `needle-core` 单元测试补充
- 基础编辑命令增强
- 打开/保存/编辑链路稳定性改进
- README / docs 改进

---

## Development Setup

### 1. Install Rust

```bash
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
```

### 2. Clone and check

```bash
git clone <your-fork-or-repo-url>
cd needle-editor
cargo check
```

### 3. Run tests

```bash
cargo test -p needle-core
```

### 4. Run the desktop app

```bash
cargo run -p needle-desktop
```

---

## Development Guidelines

### 1. Keep changes focused

请尽量避免把多类问题塞进一个 PR：

- 一个 PR 解决一个主题
- 大改动拆小步
- 先可运行，再逐步变优雅

### 2. Prefer explicit architecture over clever tricks

Project Needle 是编辑器项目，后期复杂度天然会爆炸。

所以我们更偏好：

- 清晰的数据结构
- 清晰的状态流
- 可测试的命令语义
- 可维护的代码组织

而不是一时看起来很帅、半年后谁都不敢碰的技巧代码。

### 3. Tests matter

如果你改了：

- Buffer 行为
- Selection 语义
- 命令逻辑
- Undo / Redo

请尽量补对应测试。

编辑器内核最怕“改一处、坏三处”。测试不是装饰，是保险丝。

### 4. Document meaningful design changes

如果你的改动影响：

- 架构边界
- API 兼容策略
- 插件宿主方向
- 项目路线图

请同步更新 `docs/` 里的相关文档。

---

## Coding Style

### Rust

提交前请至少运行：

```bash
cargo fmt --all
cargo check
cargo test -p needle-core
```

如果适用，也欢迎运行：

```bash
cargo clippy --all-targets --all-features
```

### Commit Messages

不强制某种格式，但建议尽量清晰，例如：

- `core: add delete_forward command`
- `ui: wire up save-as dialog`
- `docs: rewrite README for open source release`

---

## Pull Requests

提交 PR 时，建议说明：

1. **What changed**
2. **Why it changed**
3. **How it was tested**
4. **Anything reviewers should pay attention to**

一个好 PR 描述，能省掉很多来回猜谜。

---

## What Not to Do

当前阶段请尽量避免：

- 未讨论就大规模重写核心架构
- 在一个 PR 里同时改 core、UI、plugin_host 且不附解释
- 引入与项目方向不一致的重量级依赖
- 为了“像 Sublime”而复制一堆历史行为但没有测试

---

## Questions / Discussion

如果你不确定一个方向值不值得做，最好的方式不是闷头写完，而是先提出来讨论。

我们非常欢迎：

- issue
- draft PR
- 设计讨论
- 小型原型验证

先把方向讲明白，能省很多返工。

---

## Final Note

Project Needle 还很早期。

这意味着两件事：

- 这里有很多可以做的事
- 这里也很容易一不小心把事情做复杂

所以最理想的贡献方式是：

> **做小而扎实的改进，让项目一步步更像一个真正可靠的编辑器。**

感谢你的时间和代码。