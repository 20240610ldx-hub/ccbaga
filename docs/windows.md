<!--
[INPUT]: Windows 原生实现、构建配方与实际验证边界。
[OUTPUT]: 社区开发依赖、源码构建和平台特性与限制。
[POS]: README 的 Windows 开发入口，与原生平台边界对应。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# Windows 10 / 11

CCBaga-A builds natively on Windows 10 and Windows 11 (x86_64). The pinned Fingerprint Chromium 148.0.7778.215 Windows engine is bundled with A; CCBaga-B is an isolated companion browser launcher into the same dedicated browser profile. Both launch paths start the host and its authenticated local measurement session.

## Installation

Download the Windows x64 NSIS setup installer or standalone release archive from [GitHub Releases](https://github.com/liseami/CCBaga/releases/latest). A and its companion B engine are included in the package.

Windows updates use the downloaded setup installer or package manager. The application supports standard Windows 10 (version 1809+) and Windows 11 64-bit systems.

## Dependencies

Use stable Rust (with MSVC toolchain), Bun (or Node.js 20+), Python 3.12+, and PowerShell 7 (`pwsh`).

Windows 10/11 includes OpenSSH Client by default (`C:\Windows\System32\OpenSSH\ssh.exe` and `ssh-keygen.exe`). If OpenSSH Client is not installed, enable it via:

```powershell
Add-WindowsCapability -Online -Name OpenSSH.Client~~~~0.0.1.0
```

## Build and install

```powershell
# 1. Install frontend dependencies
bun install

# 2. Download and verify pinned Fingerprint Chromium Windows x64 engine
python scripts/fetch-browser.py

# 3. Build frontend assets and execute tests
bun run build
bun test

# 4. Run native Rust verification suite
cargo test --manifest-path src-tauri/Cargo.toml

# 5. Run Windows platform regression tests
python scripts/test-windows.py

# 6. Build the complete Windows installer
bun run tauri build --config src-tauri/tauri.windows.conf.json --features custom-protocol
```

The fetcher checks the pinned publisher SHA-256 (`9ef3f471b7a6641b4224532522b29141ce3746e27d55788d88e2fd951f362579`), PE architecture, and actual product version before preparing `browser-runtime/WindowsBrowser`.

Local Tauri builds produce the NSIS setup installer under `src-tauri/target/release/bundle/nsis/`.

## Features and limits

| Feature | Windows 10 / 11 behavior |
| --- | --- |
| Claude Code cleanup | Same snapshot, hash verification, and process convergence checks; preserves skills, plugins, and project source. |
| Paths | Target paths in `%USERPROFILE%\.claude`, Claude CLI cache in `%LOCALAPPDATA%\claude-cli-nodejs`, Claude Desktop cache and logs in `%APPDATA%\Claude` and `%LOCALAPPDATA%\Claude`. |
| Claude Desktop | Cache, GPU cache, Code cache, session storage, and crash dumps are identified and cleanly removed upon snapshot confirmation. |
| Credentials | File credentials in `%USERPROFILE%\.claude\.credentials.json` and Claude-related entries in Windows Credential Manager (`cmdkey`) are supported. |
| CLI install management | Native Anthropic Windows installer (`irm https://claude.ai/install.ps1 | iex`), npm global, and bun global installations are supported via `scripts/claude-cli.ps1`. |
| Browser | Pinned Windows fingerprint engine (`chrome.exe`), isolated Default profile, cache/site-data separation, seed rotation, and real session verification. |
| Network route | Follows system TUN (Clash / Sing-box) or explicit browser proxy. Proxy credentials stay in native memory. |
| VPS | Built-in Windows OpenSSH (`ssh.exe` and `ssh-keygen.exe`) with three-key validation. Temporary private key ACL permissions are locked down via `icacls` to prevent OpenSSH permission warnings. |
| Time zone | Queried via `tzutil /g`, set and verified via `tzutil /s`. Standard IANA-to-Windows timezone mapping is built-in. |
| Language/region | Queried via `Get-Culture` / Windows API. Deep adjustments open Windows Settings (`ms-settings:regionformatting`). |
| Location | Windows location service state is checked via registry (`CapabilityAccessManager\ConsentStore\location`), can be disabled or managed via `ms-settings:privacy-location`. |
| Settings launcher | Uses Windows `ms-settings:` protocol handlers for date/time, region, and privacy. |
| Default browser | Registers `CCBaga-B` under `HKCU\Software\Clients\StartMenuInternet` and launches `ms-settings:defaultapps` for confirmation. |

