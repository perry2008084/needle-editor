# plugin_host

这里用于承载 Project Needle 的 Python 插件宿主。

目标：
- 提供 `sublime` / `sublime_plugin` shim
- 扫描与加载 package
- 通过 JSON-RPC 与主进程通信
- 隔离插件崩溃与异常

当前仅完成目录初始化，后续将补充：
- bootstrap 启动入口
- shim 模块
- loader
- API 兼容测试
