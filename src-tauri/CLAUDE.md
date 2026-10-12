# src-tauri/
> L2 | 父级: ../CLAUDE.md
SECURITY.md: PR #6 原生安全审阅与数据流地图，列出本地读取、外部连接、确认后修改、代理不序列化及 Linux A/B 安装运行保护；验证范围按实际环境陈述。
宿主保持网络检测、SSH 与系统权限在 Rust 中；剪贴板复制与密钥文件选择只在主线程执行。清理与电脑设置使用后台任务；清理不读取钥匙串正文。
src/ - 清理、浏览器、地区与 VPS 事务的独立边界；见其 CLAUDE.md。
native/ - macOS SSH 文件选择、剪贴板、授权、直接钥匙串删除与电脑环境设置；见其 CLAUDE.md。
capabilities/ - 主窗口能力边界；见其 CLAUDE.md。
icons/ - 应用包图标；见其 CLAUDE.md。
Cargo.toml: Tauri 宿主、官方 updater/process 插件与原生桥依赖；custom-protocol 映射 tauri/custom-protocol，由正式打包启用以读取内嵌前端，开发模式默认不启用。
Cargo.lock: 锁定 Rust 与平台库的实际版本。
providers.json: 编译内嵌的供应商主地址与查询配置；vps.rs 按入口白名单组装，回归固定完整目标地址，WebView 不提供 URL 或配置。
build.rs: 编译 Objective-C 桥并生成 Tauri 构建元数据。
tauri.conf.json: 产品名 CCBaga-A、窗口、应用包标识、内容安全策略、公开 HTTPS 更新清单/公钥/强制签名版本与开发默认临时签名；发布脚本显式覆盖 Developer ID 签名身份。
tauri.macos.conf.json / tauri.linux.conf.json: 分平台资源与打包配置；macOS 保持双应用归档，Linux 使用 PNG 和内置 x86_64 引擎；默认 Tauri 构建 deb/AppImage，RPM 配置的依赖由原生 rpmbuild 消费；build.rs 只在 macOS 编译 Objective-C。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
