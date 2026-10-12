# CCBaga Windows 10/11 Port Implementation Plan

日期：2026-10-12。规划者：Codex（software mode）。

**Goal:** 将 CCBaga（双应用套件：CCBaga-A 宿主与 CCBaga-B 指纹浏览器）完整扩展至 Windows 10/11，在功能完备性、安全沙箱、进程收敛及操作人体工学上达到或超越已有的 macOS 与 Linux 版本，并符合向原作者仓库提交高质量 PR 的标准。

**Architecture:** 采用与 Linux 版本对齐的解耦设计：
1. 平台原生抽象（`src-tauri/src/windows.rs`）：提供统一的 C-ABI 兼容平台桥，实现 Windows 凭据管理、时区/语言/区域/定位状态查询与配置修改（依托 `tzutil`、Windows 注册表及 `ms-settings` 原生入口）、剪贴板复制、SSH 密钥对话框及基于 Windows Toolhelp32 Snapshot 的进程发现与树状收敛终止。
2. 独立指纹浏览器适配（`src-tauri/src/browser/windows.rs`）：下载并打包官方发布的 `adryfish/fingerprint-chromium` Windows x64 构件（v148.0.7778.215），实现独立的 Default 资料隔离、命令行参数注入（种子、画布指纹、WebRTC 与 TUN 跟随）、以及 Windows 10/11 系统的默认浏览器注册与核验。
3. Claude Code CLI 管理（`claude_cli.rs` 与 `scripts/claude-cli.ps1`）：支持 Windows 原生官方安装器（`irm https://claude.ai/install.ps1 | iex`）、npm 全局安装及 bun 安装；严格遵循 `CC_CLI_STEP=` / `CC_CLI_METHOD=` / `CC_CLI_PATH=` / `CC_CLI_VERSION=` / `CC_CLI_DONE=1` 的进度通道协议。
4. Windows OpenSSH 权限加固（`vps.rs` 与 `vps_keys.rs`）：针对 Windows 版 OpenSSH 对私钥权限的强校验（拒绝宽松 ACL 权限），在生成临时 PEM 私钥时通过 Windows ACL（`icacls` 严格限制仅当前用户完全控制，移除所有继承权限），防止触发 OpenSSH "UNPROTECTED PRIVATE KEY FILE" 错误。
5. 构建与打包系统（`tauri.windows.conf.json`、`scripts/fetch-browser.py`、`src-tauri/icons/icon.ico`）：支持 NSIS 安装包打包，生成 Windows 独立可执行文件与分发包。

**Hard constraints:**
- 遵循 `CLAUDE.md` 与 `SECURITY.md` 安全契约：私钥只在原生临时受控权限中使用，随事务立即销毁；家宽凭据仅在加密 stdin 中传递；绝对禁止向前端 WebView 序列化系统代理凭据。
- 遵循项目清理契约：有效快照核验后物理删除，不创建备份、不进回收站、不提供撤回。
- 遵循单窗口规范：发布构建必须启用 `custom-protocol`，防止连接开发服务器导致白屏。
- 终端环境偏好：Windows 环境下统一使用 PowerShell 7 (`pwsh`)，严禁调用 Windows PowerShell 5.1。
- 原生代码规范：不引入外部不可控的第三方 C 动态库，全部依赖 Rust 标准库、轻量 `windows-sys` 或系统内置命令闭环。

---

## 明确不做

1. 不篡改或劫持 Windows 底层 TCP/IP 协议栈或系统全局网络代理（网络环境继续严格遵循官方 Clash/Sing-box TUN 模式或应用级指纹代理）。
2. 不触碰用户的个人资产（包括 skills、plugins、hooks、commands、CLAUDE.md、MCP 配置及个人项目源码）。
3. 不支持 32 位系统或已淘汰的 Windows 7/8 系统（明确面向主流 Windows 10 64位 与 Windows 11）。
4. 不在非用户显式操作下修改系统默认浏览器或破坏既有浏览器快捷方式。

---

## 阶段规划

### Phase 1: 平台抽象与交叉编译错误解耦 (Cross-Platform Gating)

**Status:** pending

**Files:**
- Modify: `src-tauri/Cargo.toml`（添加 `[target.'cfg(windows)'.dependencies]` 引入 `windows-sys` 必要的 win32 特性）
- Modify: `src-tauri/src/main.rs`（引入 `#[cfg(target_os = "windows")] mod windows;`，在 setup 与 invoke_handler 中接入 Windows 平台）
- Modify: `src-tauri/src/native.rs`（补充 Windows 下的 `rf_*` 桥接声明与条件编译分支）
- Modify: `src-tauri/src/cleaner_io.rs`（解除 `std::os::unix::fs::OpenOptionsExt` 限制，Windows 使用标准原子写入）
- Modify: `src-tauri/src/cleaner_transaction.rs`（权限模式获取区分 Unix 与 Windows，Windows 默认安全掩码）
- Modify: `src-tauri/src/cleaner_targets.rs`（解除 Unix `MetadataExt`，引入跨平台元数据度量；补充 Windows 平台路径：%USERPROFILE%\.claude、%APPDATA%\Claude、%LOCALAPPDATA%\claude-cli-nodejs 等）
- Modify: `src-tauri/src/cleaner.rs`（跨平台 `Stamp` 时间戳与设备标识、解除 `getuid()` 限制、接入 Windows 进程收敛与时区读取）
- Modify: `src-tauri/src/cleaner_folder.rs`（去除 Unix `uid()` 硬编码，Windows 上通过规范路径验证）
- Modify: `src-tauri/src/browser/mod.rs`（引入 `#[cfg(target_os = "windows")] include!("windows.rs");`，解除 `mode(0o600)` 的 unix 限制）
- Modify: `src-tauri/src/browser/profile.rs`（解除 `PermissionsExt::from_mode` 限制，Windows 下采用标准权限与只读控制）
- Modify: `src-tauri/src/vps_keys.rs`（去除 Unix `OpenOptionsExt`，Windows 采用临时文件并结合 ACL 控制）
- Modify: `src-tauri/src/vps.rs`（Windows 下使用 `cmd /c start` 打开供应商网站，OpenSSH 调用标准 `ssh.exe`）

**验证:**
`cargo check --manifest-path src-tauri/Cargo.toml`
预期：编译错误由当前的 72 个迅速收敛到 0 个，基础模块在 Windows 上顺利通过类型检查与语法检查。

**反模式:**
- 不要使用 `#[cfg(not(target_os = "macos"))]` 粗暴把 Linux 和 Windows 混在一起。Linux 与 Windows 系统调用差异巨大，必须保持 `windows.rs` 独立清晰。

---

### Phase 2: Windows 原生平台桥与系统设置实现 (Windows Native Bridge: windows.rs)

**Status:** pending

**Files:**
- Create: `src-tauri/src/windows.rs`
  - 实现 C-ABI 兼容函数：
    * `rf_free(value: *mut c_char)`：释放 CString。
    * `rf_choose_ssh_keys(extension: *const c_char)`：通过原生 PowerShell / Forms / Win32 对话框选取 `.pem`、`.ppk`、`.pub` 密钥文件。
    * `rf_copy(text: *const c_char)`：将文本写入 Windows 系统剪贴板（Win32 剪贴板 API 或 `pwsh -Command Set-Clipboard`）。
    * `rf_credentials_list()`, `rf_credentials_remove()`, `rf_keychain_unlock()`：针对 Windows Credential Manager 的安全读取与脱敏清理，无相关凭据时返回友好状态，不伪造空成功。
    * `processes()` & `node_cli()`：利用 Toolhelp32 遍历系统进程，精确识别 `Claude.exe`、`claude.exe` 及运行 `@anthropic-ai/claude-code/cli.js` 的 Node/Bun 进程。
    * `signal_process(pid, term)`：通过 `OpenProcess(PROCESS_TERMINATE)` + `TerminateProcess` 或 `taskkill /F /T /PID` 彻底级联关闭目标进程。
    * `rf_browser_machine()`：采集 Windows 机器特征（Windows 版本号、CPU 核心数、系统区域语言与 TUN/Proxy 环境变量）。
    * `rf_browser_running(profile)` & `rf_browser_quit(profile)`：检查与退出指定的指纹浏览器进程实例。
    * `rf_system_read()`：读取 Windows 当前时区（`tzutil /g`）、可用时区列表（`tzutil /l`）、会话显示语言与定位服务开启状态。
    * `rf_system_open(pane)`：调用 `ms-settings:dateandtime`、`ms-settings:regionformatting`、`ms-settings:privacy-location`。
    * `rf_system_timezone(zone)`：通过 `tzutil /s <zone>` 修改并二次核验时区。
    * `rf_system_language(requested)` & `rf_system_region(requested, locale)`：核验或引导用户在系统设置中切换。
    * `rf_system_location()`：修改 Windows 定位权限注册表项并验证。

**验证:**
`cargo test --manifest-path src-tauri/Cargo.toml windows`
预期：Windows 平台桥的机器特征读取、时区检测与有效性校验单元测试全部通过。

**反模式:**
- 严禁硬编码注册表未知项；修改时区与设置后必须进行读取核验（read-after-write verification）。

---

### Phase 3: 指纹浏览器 Windows 运行时准备与集成 (Browser Engine Integration)

**Status:** pending

**Files:**
- Create: `src-tauri/src/browser/windows.rs`
  - 实现 Windows 浏览器启动与默认浏览器管理：
    * `engine(app)`：检索开发目录（`browser-runtime/WindowsBrowser`）与打包资源目录（`resources/browser-runtime/WindowsBrowser`）下的 `chrome.exe`。
    * `prepare_engine(app)`：验证 `chrome.exe` 与 `version.txt` 是否就绪。
    * `binary(engine)`：返回 `engine.join("chrome.exe")`。
    * `version(engine)`：读取 `version.txt` 中的固定版本号。
    * `register(engine)`：向 Windows 注册表当前用户键（`HKCU\Software\Clients\StartMenuInternet\CCBaga-B`）注册浏览器入口。
    * `start_windows_companion(app, urls)`：以独立 Default profile、指纹随机种子、Canvas 随机种子及隔离端口启动 Companion 浏览器。
    * `default_read()` & `browser_set_default(app)`：查询当前 HTTP/HTTPS 默认处理程序，若未设置则弹出系统应用选择或引导打开设置。
- Modify: `scripts/fetch-browser.py`
  - 增加 Windows 平台下载支持：从 `https://github.com/adryfish/fingerprint-chromium/releases/download/148.0.7778.215/ungoogled-chromium_148.0.7778.215-1.1_windows_x64.zip` 下载，校验 SHA-256（`9ef3f471b7a6641b4224532522b29141ce3746e27d55788d88e2fd951f362579`），解压至 `browser-runtime/WindowsBrowser` 并写入 `version.txt`。

**验证:**
`python scripts/fetch-browser.py`
预期：正确下载、核对 SHA256 并解压出包含 `chrome.exe` 的 `browser-runtime/WindowsBrowser`，版本匹配 148.0.7778.215。

**反模式:**
- 不要解压未经验证的 upstream latest；必须严格哈希校验。

---

### Phase 4: Claude Code CLI 在 Windows 上的生命周期管理 (claude_cli.rs & scripts/claude-cli.ps1)

**Status:** pending

**Files:**
- Create: `scripts/claude-cli.ps1`
  - 编写与 `claude-cli.sh` 100% 协议兼容的 PowerShell 脚本：
    * 支持参数：`read`、`uninstall`、`reinstall`。
    * 输出标准化通道标记：`CC_CLI_STEP=`、`CC_CLI_METHOD=`、`CC_CLI_PATH=`、`CC_CLI_VERSION=`、`CC_CLI_DONE=1`。
    * 检测方法涵盖：
      1. 原生安装：`~/.local/share/claude` 或 `%LOCALAPPDATA%\Programs\claude\claude.exe`。
      2. npm 全局安装：`npm root -g` 下的 `@anthropic-ai/claude-code`。
      3. bun 全局安装：`bun pm ls -g`。
    * 重装逻辑：沿用已识别的安装方式；若无旧安装，则使用官方 Windows 原生脚本（`irm https://claude.ai/install.ps1 | iex`）。
- Modify: `src-tauri/src/claude_cli.rs`
  - 在 Windows 下通过 `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/claude-cli.ps1 <action>` 触发任务；
  - 移除 Unix `process_group(0)` 与 `kill(-(pid), 9)`，Windows 下利用 `taskkill /F /T /PID` 确保超时时子进程树被完全收敛。

**验证:**
`pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/claude-cli.ps1 read`
预期：输出标准的 CC_CLI_METHOD、CC_CLI_PATH、CC_CLI_VERSION 并以 CC_CLI_DONE=1 结束。

**反模式:**
- 严禁直接在命令中注入未经转义的用户路径；变量通过参数或环境传递。

---

### Phase 5: VPS 远程执行与 OpenSSH 权限加固 (Windows OpenSSH & ACL Hardening)

**Status:** pending

**Files:**
- Modify: `src-tauri/src/vps_keys.rs`
  - 在生成临时 PEM 私钥文件时，Windows 下调用系统 `icacls` 命令严格限制 ACL 权限：
    * `icacls <key_path> /inheritance:r /grant:r "$($env:USERNAME):(R)"`
    * 阻断任何其他用户或继承组的访问，确保 Windows OpenSSH 不会报 "bad permissions" 拒绝连接。
  - Windows 下调用系统自带的 `C:\Windows\System32\OpenSSH\ssh-keygen.exe` 进行私钥配对与格式核验。
- Modify: `src-tauri/src/vps.rs`
  - OpenSSH 客户端路径自适应：若 `ssh` 不在常规路径，优先使用 `C:\Windows\System32\OpenSSH\ssh.exe`。

**验证:**
`cargo test --manifest-path src-tauri/Cargo.toml vps_keys`
预期：临时密钥生成、配对验证、权限收敛及 Drop 自动删除单元测试通过。

**反模式:**
- 不要使用全局临时目录而不做 ACL 权限锁定，否则 Windows OpenSSH 会直接报错并退出。

---

### Phase 6: 构建配置、打包、文档与 PR 准备 (Packaging, Docs & PR Baseline)

**Status:** pending

**Files:**
- Create: `src-tauri/tauri.windows.conf.json`（Windows 专属窗口装饰与 NSIS 打包配置）
- Create: `docs/windows.md`（Windows 10/11 架构、运行环境、安装依赖与已知限制说明文档）
- Modify: `README.md`（在文档中增加 Windows 10/11 的支持说明及开发指南链接）
- Modify: `package.json`（补充 `build:windows` 与 `test:windows` 脚本便捷入口）
- Create: `scripts/package-browser.ps1`（Windows 对应的打包与资源嵌入脚本）

**验证:**
1. `bun run build`
2. `bun test`
3. `cargo test --manifest-path src-tauri/Cargo.toml`
4. `bun run tauri build --config src-tauri/tauri.windows.conf.json --features custom-protocol`
预期：前端测试全过，Rust 测试全过，Tauri 成功构建出 Windows NSIS 安装包并内嵌自定义协议资源。

**反模式:**
- 严禁在 release 构建中忽略 `custom-protocol`；发布构建必须自包含前端。

---

## 验证总入口

依次执行以下命令，全部返回 Exit 0 即代表 Windows 移植与功能扩展圆满成功：
```powershell
# 1. 前端构建与测试
bun run build
bun test

# 2. 原生 Rust 核心测试
cargo test --manifest-path src-tauri/Cargo.toml

# 3. 完整 Tauri Windows 构建
bun run tauri build --config src-tauri/tauri.windows.conf.json --features custom-protocol
```
通过即止。

