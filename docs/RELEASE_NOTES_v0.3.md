# Project Needle v0.3.0

第三个公开发布版本，重点从“基础项目工作流可用”推进到“P0 可用性主线基本收口”。

## 包含内容
- Windows x64 发布包：
  - `needle-desktop.exe`
  - `install.ps1`
  - `uninstall.ps1`
- macOS Apple Silicon 发布包：
  - `Needle Editor.app`

## 本版新增能力
- 顶部菜单分组（替代原先一整排按钮）
- GUI → core 最小 diff 文本同步
- core → GUI selection 回写基础版
- 程序化选区更新后的 UI 回写抑制
- 用户级 settings 文件基础版
- 用户级 keymap 文件基础版
- 默认 / 用户 / 项目配置合并规则基础版
- settings / keymap 基础校验与错误提示
- `crates/search` 搜索模块基础版
- Quick Open 改接搜索模块
- Find in Project 后台任务化
- Find in Project 批次结果回流
- Find in Project 取消机制 + `Cancel Search` 按钮
- 当前文件 Find / Replace 匹配计数增强
- 当前文件匹配高亮 + 当前命中项强化高亮

## 验证情况
- `cargo test -p needle-core` 通过
- `cargo test -p needle-search` 通过
- `cargo test -p needle-ui` 通过
- `cargo check -p needle-ui` 通过
- Linux 本地 release 二进制可构建
- Windows / macOS 包通过 GitHub Actions 跨平台 runner 构建并打包

## 已知限制
- 仍处于早期 MVP 阶段
- 多标签生命周期仍未完全收口
- 语法高亮尚未接入
- 更完整的项目模型 / Sidebar 状态装饰仍待推进
- macOS 包未进行代码签名与 notarization，首次打开可能需要绕过 Gatekeeper
- plugin_host 与 Sublime 插件兼容层尚未接入

## 一句话总结

`v0.3.0` 不是“功能爆炸式扩张”的版本，而是一个很实在的版本：

> 它把编辑手感、配置层、搜索链路和查找体验这几条最影响日常试用的 P0 主线，基本拉到了能认真继续往下做产品的水平。
