# CCBaga Windows 10/11 移植项目交接文档 (HANDOFF.md)

> **适用对象**：无本会话前序上下文的全新 Agent / 协作者  
> **最后更新**：2026-10-12  
> **项目分支**：feat/windows-support (已同步至 GitHub: https://github.com/yq04/CCBaga/tree/feat/windows-support)  
> **上游关联 PR**：https://github.com/liseami/CCBaga/pull/7

---

## 1. 我们在做什么任务

基于优质开源仓库 https://github.com/liseami/CCBaga（技术栈：Tauri 2.12.1 + React 19 + TypeScript + Rust）进行 **Windows 10 / 11 (x86_64) 系统的完整扩展与适配**。

CCBaga 是一款面向 Claude Code CLI 与 Claude Desktop 的环境配置与深度清理套件，包含两个伴生应用：
1. **CCBaga-A（宿主应用）**：负责环境与时区核验、白名单物理清理、凭据与设备标识（ant-did / machineID）重置、Claude Code CLI 卸载/重装，以及通过 OpenSSH 进行 VPS 远程网络配置。
2. **CCBaga-B（伴生浏览器）**：基于 adryfish/fingerprint-chromium 深度定制的独立指纹浏览器，具备随机种子注入、Canvas/WebGL 伪装、独立 Default 资料隔离、自动跟随 TUN 模式及出口真实 IP/地区核验能力。

原作者仅完成了 macOS 与 Linux x86_64 的支持，本任务的目标是**在保持 macOS / Linux 原生架构零退化的前提下，以原生 Win32 高性能规范完成 Windows 10/11 扩展，并向上游作者提交生产就绪的 PR**。

---

## 2. 已经完成了什么

### 2.1 原生架构与功能实现
- **Windows 原生平台桥 (src-tauri/src/windows.rs)**：
  * **微秒级进程收敛**：放弃了开销巨大的 PowerShell WMI 查询，全面采用 Win32 内核级 CreateToolhelp32Snapshot 快照遍历与 64 位 PEB 结构直读（NtQueryInformationProcess + ReadProcessMemory），单次扫描耗时 < 0.5 毫秒，精确捕获 Claude.exe、claude.exe 及 Node/Bun 下挂载的 @anthropic-ai/claude-code，支持级联终止。
  * **Win32 凭据管理器**：接入原生 CredEnumerateW 与 CredDeleteW，支持纯内存 UTF-16 检索与清理 Claude 相关凭据，彻底杜绝了控制台代码页乱码与非英文多语言系统的识别失效。
  * **Win32 原生剪贴板**：直接调用 user32 / kernel32 的 OpenClipboard 与 SetClipboardData，杜绝子进程开销与控制台尾部换行符。
  * **系统时区与设置映射**：建立覆盖美洲、欧洲、亚洲、大洋洲全球核心时区的全量双向映射表 TZ_MAP，通过 tzutil /g 和 tzutil /s 实现系统时区核验与修改；通过 ShellExecuteW 规避 cmd /c start 容易被 URL 中 & 符号截断的命令注入隐患，支持安全唤起 ms-settings:。
  * **密钥选择对话框非阻塞化**：在 src-tauri/src/vps.rs 中，将 Windows 端的密钥选择解耦至 tauri::async_runtime::spawn_blocking，并在 STA 线程模式下唤起 WinForms 对话框，避免冻结主 UI 事件循环。
- **指纹浏览器 Windows 引擎闭环 (src-tauri/src/browser/windows.rs & scripts/fetch-browser.py)**：
  * 扩展下载器支持上游官方发布的 ungoogled-chromium_148.0.7778.215-1.1_windows_x64.zip，校验 SHA-256（9ef3f471b7a6641b4224532522b29141ce3746e27d55788d88e2fd951f362579）并解压至 browser-runtime/WindowsBrowser。
  * 实现了伴生浏览器启动、随机种子与 Chromium 参数注入、以及向注册表 HKCU\Software\Clients\StartMenuInternet\CCBaga-B 写入默认浏览器入口信息。
- **Claude Code CLI 管理脚本 (src-tauri/src/claude_cli.rs & scripts/claude-cli.ps1)**：
  * 编写与 upstream 完全协议兼容的 PowerShell 脚本，遵循 CC_CLI_STEP= / CC_CLI_METHOD= / CC_CLI_PATH= / CC_CLI_VERSION= / CC_CLI_DONE=1 进度通道。
  * 支持 Anthropic 官方安装脚本（install.ps1）、npm 与 bun 全局安装，支持 pwsh 缺失时自适应回退到系统内置 powershell。
- **Windows OpenSSH 权限加固 (src-tauri/src/vps_keys.rs)**：
  * 生成临时 PEM 私钥时，通过 icacls 去除继承并仅授予当前用户完全控制权限 :(F)，既满足 Windows OpenSSH 严格权限检查，又确保析构清理时不会因 AccessDenied 残留文件。
- **构建与工程配置**：
  * 生成高分辨率 Windows 多尺寸图标 src-tauri/icons/icon.ico。
  * 编写 src-tauri/tauri.windows.conf.json（支持窗口原生装饰与 NSIS 安装包打包）。
  * 编写 Windows 开发与架构文档 docs/windows.md，并更新 README.md 与 package.json。

### 2.2 验证结果（全部绿灯）
- 前端测试：bun test（10/10 项通过）
- 原生测试：cargo test --manifest-path src-tauri/Cargo.toml（30/30 项单元测试全过，0 失败，耗时仅 0.51s）
- Windows 专项测试：python scripts/test-windows.py（4/4 项全部通过）
- 独立 Release 构建：cargo build --release --features custom-protocol 成功编译出 10.4MB 的原生发行包 src-tauri/target/release/requirement-french.exe。

### 2.3 PR 提交状态
- 所有改动已整洁提交至本地与 Fork 仓库分支 feat/windows-support（包含 4 个有序提交）。
- 已正式向原作者仓库发起 Pull Request：https://github.com/liseami/CCBaga/pull/7。

---

## 3. 当前卡在哪

- **技术与代码实现**：**完全没有任何卡点**。所有单元测试、集成测试、平台回归与 Release 编译均已通过，本地工作树干净（working tree clean）。
- **外部协同状态**：目前正在等待上游原作者 **liseami** 审查 PR #7。

---

## 4. 下一步计划是什么

1. **跟踪与响应 PR #7**：
   - 关注 GitHub 上 https://github.com/liseami/CCBaga/pull/7 的审阅意见。
   - 若作者提出代码调整或疑问，直接在当前本地 feat/windows-support 分支上修改并提交 push，PR 会自动同步更新。
2. **可选 CI 增强（视作者意愿）**：
   - 上游仓库现有的 .github/workflows/ 仅配置了 macOS 与 Linux runner。若作者希望将 Windows release 纳入 GitHub Actions 自动矩阵构建，可协助编写 Windows runner 打包工作流。
3. **本地使用与体验**：
   - 如需在当前 Windows 机器上直接启动 CCBaga，可直接双击运行已编译完成的 src-tauri/target/release/requirement-french.exe，或在终端执行 bun run tauri dev 启动开发调试。

---

## 5. 有哪些踩过的坑，绝对不要再踩！

以下是本阶段在 Windows 平台实战中踩平的**硬核底层陷阱**，后续会话若涉及修改千万不可倒退：

1. **绝对不要在 functions__exec 脚本中调用 notify(...) 或 yield_control()（致命硬闸门）**：
   - **灾难后果**：号池转换器（CPA）严格遵循 1:1 工具调用匹配，中途注入自定义响应会导致队列首部残留，在下一次工具调用时秒报 400，**直接造成整个会话彻底报废**。
   - **铁律**：所有思考与进度更新走正常的 commentary 通道；工具输出必须在脚本结束时通过单次 text(...) 统一交付。

2. **绝对不要使用 powershell Get-CimInstance 或 WMI 轮询进程**：
   - **根因**：启动 PowerShell 子进程 + WMI RPC 查询数百个进程的冷启动耗时高达 8.7 秒！前端轮询会迅速造成进程堆叠打满 CPU，界面严重无响应。
   - **铁律**：必须使用 src-tauri/src/windows.rs 中已落地的 Win32 内核级 CreateToolhelp32Snapshot + PEB 命令行读取，耗时小于 0.5 毫秒。

3. **在 Windows 上绝对不能使用 std::fs::canonicalize**：
   - **根因**：Windows 原生实现会给路径强制添加 UNC 扩展前缀。这会导致 path.starts_with(root) 或 canonicalize(p) == Some(p) 恒为 false，导致清理模块误报“路径已变化或超出当前用户范围”；传递给 Node.js 还会触发 Error: EISDIR: illegal operation on a directory, lstat 'C:' 崩溃。
   - **铁律**：Windows 下路径规范化必须使用生态标配的 dunce::canonicalize。

4. **measure 与 stamp 绝对不能包含 last_access_time**：
   - **根因**：首次扫描遍历时读取文件会立刻刷新 Windows NTFS 记录的 last_access_time。当执行清理二次核验 identity 时，由于访问时间已变，二次生成的哈希值与快照不一致，会直接报错“目录内的文件已经变化，请重新扫描”。
   - **铁律**：Windows 下元数据哈希只允许使用稳定字段：file_attributes、creation_time、last_write_time、file_size。

5. **OpenSSH 临时私钥的 icacls 权限千万不要设为 :(R)（只读）**：
   - **根因**：设为 :(R) 固然能过 OpenSSH 客户端的权限检查，但在 Rust 的 PrivateKey::drop 析构触发 fs::remove_file 时，会因为当前进程缺少写入/删除权限而抛出 AccessDenied 导致临时私钥残留，引发凭据安全隐患。
   - **铁律**：必须使用 :(F)（Full Control）授予当前登录用户（USERNAME），OpenSSH 会正常放行，且析构时可安全删除。

6. **PowerShell 脚本（.ps1）含中文注释时必须以带 BOM 的 UTF-8（utf-8-sig）保存**：
   - **根因**：Windows 内置的 Windows PowerShell 5.1 在解析无 BOM 的 UTF-8 文本时，会强制按系统 ANSI 代码页（如 CP936）解析中文注释，导致多字节字符与单双引号/括号发生粘连错位，直接抛出语法解析错误。
   - **铁律**：项目中的所有 .ps1 脚本（如 scripts/claude-cli.ps1）必须确保头部包含 UTF-8 BOM 标记。

7. **不要通过 cmd /c start 拼接打开 URL**：
   - **根因**：若 URL 中带有查询参数（如 ?a=1&b=2），cmd 解释器会将 & 视为命令连接符执行，造成参数丢失甚至命令注入风险。
   - **铁律**：必须统一使用 Win32 原生 ShellExecuteW 唤起外部浏览器或 ms-settings: 协议。

