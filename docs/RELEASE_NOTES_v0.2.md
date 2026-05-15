# Project Needle v0.2.0

第二个公开发布版本，重点从“单文件 MVP”推进到“基础项目工作流可用”。

## 包含内容
- Windows x64 发布包：
  - `needle-desktop.exe`
  - `install.ps1`
  - `uninstall.ps1`
- macOS Apple Silicon 发布包：
  - `Needle Editor.app`

## 本版新增能力
- Open Folder
- Sidebar 文件树（基础版）
- Recent Projects（基础版）
- Quick Open（增强基础版：模糊匹配 + 项目索引缓存）
- Find in Project（增强基础版：结果缓存 / 大小写选项）
- 项目级 `.needle/settings.json` 热重载（基础版）
- 项目级 keymap（基础版）
- 项目文件系统 watcher 基础版
- 项目索引轮询回退刷新基础版
- Goto Line（基础版）
- 多标签页关闭 dirty 提示（基础版）

## 验证情况
- `cargo check` 通过
- `cargo test -p needle-core` 通过
- 核心测试 16/16 通过
- Linux release 二进制可成功构建
- Windows / macOS 包由 GitHub Actions 跨平台 runner 构建并打包

## 已知限制
- 仍处于早期 MVP 阶段
- GUI 文本同步仍较粗，后续会继续细化
- Quick Open 模糊排序目前还是基础版
- 项目搜索尚未后台化 / 异步化
- macOS 包未进行代码签名与 notarization，首次打开可能需要绕过 Gatekeeper
- plugin_host 与 Sublime 插件兼容层尚未接入
