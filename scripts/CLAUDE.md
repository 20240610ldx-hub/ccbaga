# scripts/
> L2 | 父级: ../CLAUDE.md
make-icon.swift: AppKit 将压缩品牌图绘入 macOS 圆角网格，保持原图并添加透明留白与投影。
make-icons.sh: 从 assets/ 品牌图生成各尺寸 iconset，iconutil 输出 src-tauri/icons 与 browser-runtime/icons。
brand-browser.sh: 编译 CCBrowser 入口、写入 CCBaga-B 名称/图标/各语言显示名、清除构建资源扩展属性并调用 sign-browser.py 完整签名（显式固定 ai.local.requirement-french.browser，干净上游引擎也必须与宿主一致）。
sign-browser.py: 内层 Mach-O → Helper 应用 → Framework → 浏览器外壳顺序签名；Developer ID 使用可信时间戳、Hardened Runtime，仅 Renderer/GPU 使用 JIT entitlement；支持显式本机临时签名 -。
test-release-env.py: 伪造凭据环境导入真实发布入口，断言 Apple/更新私钥不进入构建子进程环境，不加载真实密钥。
test-sign-browser.py: 临时伪引擎树验证叶子到容器的签名顺序、最小 JIT 授权与符号链接不重复遍历。
fetch-browser.py: 固定上游引擎 DMG 与 SHA-256，核验版本和 arm64 后恢复引擎；干净 CI 检出不依赖维护者本机 Browser.app。
release-macos.py: 外部凭据与可选 CC_RELEASE_VERSION 驱动三阶段公证装订；明确将原生构建目录 Browser.app 暂存为交付名 CCBaga-B.app，再由 dmgbuild 无头写入 Finder 布局；装订后的宿主单独归档并签名绑定版本，更新私钥仅传给签名子进程；Apple 凭据不进入构建、DMG 或更新签名子进程，公证工具只通过明确参数接收其认证输入；官方更新器回归核验后生成公开 latest.json；产物和哈希记录写入被忽略的 releases/。
dmg-settings.py: 固定 800×500 像素背景、两应用到 Applications 的拖拽坐标；通知文件保留在盘内但默认隐藏，不依赖 Finder 自动化。
verify-dmg.py: 只读挂载实际安装盘，校验双应用名称、包标识与启动入口，再校验严格签名与 Gatekeeper、Applications 链接和像素背景；Finder 元数据不得破坏应用签名。
release-requirements.txt: dmgbuild、ds-store 与 mac-alias 的完整版本及 wheel SHA-256；仅发布阶段安装，不增大应用包。
package-browser.sh: 先品牌化签名，再打包引擎，ditto 保留 Framework 链接与可执行权限；归档进入应用资源。
install-apps.sh: bun install:apps 兼容入口，只转交统一安装层。
install-apps.py: 严格签名与 Gatekeeper 验收后的事务替换、同标识旧名迁移与失败回滚；保留隔离和资料，正常退出后安装、只刷新相关 Dock 项并启动宿主。
setup-vps.sh: 从用户 Downloads 脚本纳入应用的 Debian 12/13 root 设置；Xray Reality 经家宽 SOCKS5 出口、Stripe 直连例外、阻断 UDP、SSH/防火墙加固与自检；仅全部通过输出 CC_SUB_URL，支持 IPv6 订阅地址，失败不打印完整配置，Xray 凭据配置限 root/实际服务组可读。
claude-cli.sh: 原生宿主内嵌的只读安装检查与官方 CLI 卸载/重装，使用登录终端 PATH，发现 nvm/全局 npm、Homebrew 双通道与原生版本；只卸载程序，保护自定义启动器与配置资产，重装沿用已识别包管理器并核验版本。
install-linux.py: Linux 用户前缀安装并保留资料；拒绝自定义启动器、链接父目录与整个安装目录下仍运行的 A/B，失败回滚。
test-linux.py: 临时构建验证安装隔离、B 运行保护、桌面写入回滚、Exec 引用与载荷缺失/引擎篡改拒绝；不访问用户资料。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md

release-linux.py: 无 Apple/更新凭据的 x86_64 三格式构建；真实解包核对宿主、UI、B 入口、图标与引擎字节，生成独立校验清单。
verify-linux-package.py: Linux 安装包只读载荷契约，供 Ubuntu/Arch 共用，拒绝缺失 UI、入口或被改写的引擎。
build-arch.sh: 固定摘要容器准备 Node.js 等构建/测试依赖，以非 root makepkg、原生回归与解包验收，公开源码与 Cargo 缓存挂载；不接触 macOS 凭据。
