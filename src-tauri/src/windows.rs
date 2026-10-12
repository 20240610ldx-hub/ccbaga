// [INPUT]: Windows 原生凭据、时区、注册表系统设置、Toolhelp32 进程与系统工具。
// [OUTPUT]: Windows 平台 JSON 桥；支持时区/语言/区域/定位状态查询与修改、SSH 密钥对话框、剪贴板及 Claude 进程收敛。
// [POS]: 仅原生识别与用户触发的系统操作；不含网络客户端，不向 UI 输出进程参数或代理秘密。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CStr, CString},
    process::{Command, Stdio},
};

fn response(value: Result<Value, String>) -> *mut c_char {
    let value = value.unwrap_or_else(|error| json!({"error": error}));
    CString::new(value.to_string())
        .expect("JSON escapes NUL")
        .into_raw()
}

unsafe fn input<'a>(value: *const c_char) -> &'a str {
    CStr::from_ptr(value).to_str().unwrap_or("")
}

pub unsafe fn rf_free(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}

fn output(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| format!("Windows: {program} 无法执行。"))?;
    if !out.status.success() {
        let err_msg = String::from_utf8_lossy(&out.stderr);
        return Err(format!("Windows: {program} 执行失败: {err_msg}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

pub unsafe fn rf_copy(text: *const c_char) -> Result<(), String> {
    let val = input(text);
    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-Command", "$input | Set-Clipboard"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "无法启动剪贴板工具。")?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(val.as_bytes());
    }
    let status = child.wait().map_err(|_| "剪贴板写入失败。")?;
    if status.success() {
        Ok(())
    } else {
        Err("剪贴板写入失败。".into())
    }
}

pub unsafe fn rf_choose_ssh_keys(extension: *const c_char) -> *mut c_char {
    let extension = input(extension);
    if !["all", "pem", "ppk", "pub"].contains(&extension) {
        return response(Err("无效的密钥文件类型。".into()));
    }
    let filter = match extension {
        "pem" => "PEM 密钥 (*.pem)|*.pem|所有文件 (*.*)|*.*",
        "ppk" => "PuTTY 密钥 (*.ppk)|*.ppk|所有文件 (*.*)|*.*",
        "pub" => "公钥 (*.pub)|*.pub|所有文件 (*.*)|*.*",
        _ => "SSH 密钥 (*.pem;*.ppk;*.pub)|*.pem;*.ppk;*.pub|所有文件 (*.*)|*.*",
    };
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Multiselect = $true; $d.Filter = '{filter}'; if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ $d.FileNames | ForEach-Object {{ Write-Output $_ }} }}"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output();
    response(match out {
        Ok(out) if out.status.success() => {
            let files: Vec<String> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(json!(files))
        }
        _ => Ok(json!([])),
    })
}

pub unsafe fn rf_credentials_list() -> *mut c_char {
    // 检查 Windows 凭据管理器中与 Claude 相关的凭据
    let out = Command::new("cmdkey").arg("/list").output();
    let mut items = Vec::new();
    if let Ok(out) = out {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let line = line.trim();
            if let Some(target) = line.strip_prefix("Target:") {
                let target = target.trim();
                let lower = target.to_lowercase();
                if lower.contains("claude") || lower.contains("anthropic") {
                    items.push(json!({
                        "service": target,
                        "reference": target,
                    }));
                }
            }
        }
    }
    response(Ok(json!({"items": items})))
}

pub unsafe fn rf_credentials_remove(reference: *const c_char) -> *mut c_char {
    let target = input(reference);
    if target.is_empty() {
        return response(Err("无效的凭据目标。".into()));
    }
    let status = Command::new("cmdkey")
        .arg(format!("/delete:{target}"))
        .status();
    match status {
        Ok(s) if s.success() => response(Ok(json!({"ok": true}))),
        _ => response(Err("凭据删除失败或不存在。".into())),
    }
}

pub unsafe fn rf_keychain_unlock() -> *mut c_char {
    response(Ok(json!({"message": "Windows 凭据管理器无需单独解锁。"})))
}

pub fn open_settings(pane: &str) -> Result<Value, String> {
    let uri = match pane {
        "timezone" => "ms-settings:dateandtime",
        "language" => "ms-settings:regionformatting",
        "location" => "ms-settings:privacy-location",
        _ => return Err("无效的系统设置入口。".into()),
    };
    Command::new("cmd")
        .args(["/c", "start", "", uri])
        .spawn()
        .map_err(|_| "无法打开 Windows 设置。")?;
    Ok(json!({"ok": true}))
}

// Windows 时区转换表与标准时区目录
pub fn iana_to_windows_tz(iana: &str) -> Option<&'static str> {
    match iana {
        "America/Los_Angeles" => Some("Pacific Standard Time"),
        "America/New_York" | "America/Toronto" => Some("Eastern Standard Time"),
        "Europe/London" => Some("GMT Standard Time"),
        "Europe/Paris" => Some("Romance Standard Time"),
        "Europe/Berlin" => Some("W. Europe Standard Time"),
        "Asia/Tokyo" => Some("Tokyo Standard Time"),
        "Asia/Seoul" => Some("Korea Standard Time"),
        "Asia/Singapore" => Some("Singapore Standard Time"),
        "Asia/Taipei" => Some("Taipei Standard Time"),
        "Asia/Hong_Kong" | "Asia/Shanghai" => Some("China Standard Time"),
        "Australia/Sydney" => Some("AUS Eastern Standard Time"),
        "UTC" | "Etc/UTC" => Some("UTC"),
        _ => None,
    }
}

pub fn windows_to_iana_tz(win: &str) -> &'static str {
    match win.trim() {
        "Pacific Standard Time" => "America/Los_Angeles",
        "Eastern Standard Time" => "America/New_York",
        "GMT Standard Time" => "Europe/London",
        "Romance Standard Time" => "Europe/Paris",
        "W. Europe Standard Time" => "Europe/Berlin",
        "Tokyo Standard Time" => "Asia/Tokyo",
        "Korea Standard Time" => "Asia/Seoul",
        "Singapore Standard Time" => "Asia/Singapore",
        "Taipei Standard Time" => "Asia/Taipei",
        "China Standard Time" => "Asia/Shanghai",
        "AUS Eastern Standard Time" => "Australia/Sydney",
        "UTC" => "UTC",
        _ => "Asia/Shanghai",
    }
}

pub fn timezone() -> String {
    let out = output("tzutil", &["/g"]).unwrap_or_else(|_| "China Standard Time".into());
    windows_to_iana_tz(&out).to_string()
}

pub fn raw_windows_timezone() -> String {
    output("tzutil", &["/g"]).unwrap_or_else(|_| "China Standard Time".into())
}

pub fn timezones() -> Vec<String> {
    let mut list = vec![
        "America/Los_Angeles".into(),
        "America/New_York".into(),
        "America/Toronto".into(),
        "Europe/London".into(),
        "Europe/Paris".into(),
        "Europe/Berlin".into(),
        "Asia/Tokyo".into(),
        "Asia/Seoul".into(),
        "Asia/Singapore".into(),
        "Asia/Taipei".into(),
        "Asia/Hong_Kong".into(),
        "Asia/Shanghai".into(),
        "Australia/Sydney".into(),
        "UTC".into(),
        "Etc/UTC".into(),
    ];
    if let Ok(out) = output("tzutil", &["/l"]) {
        for line in out.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('(') {
                list.push(line.to_string());
            }
        }
    }
    list.sort();
    list.dedup();
    list
}

fn locale() -> String {
    let out = output(
        "powershell",
        &["-NoProfile", "-Command", "(Get-Culture).Name"],
    )
    .unwrap_or_else(|_| "zh-CN".into());
    out.replace('-', "_")
}

fn session_language() -> String {
    let loc = locale();
    match loc.as_str() {
        "zh_CN" | "zh_SG" => "zh-Hans".into(),
        "zh_TW" | "zh_HK" => "zh-Hant".into(),
        s if s.starts_with("en_") => "en-US".into(),
        s if s.starts_with("ja_") => "ja".into(),
        s if s.starts_with("ko_") => "ko".into(),
        s if s.starts_with("de_") => "de".into(),
        s if s.starts_with("fr_") => "fr".into(),
        s => s.split('_').next().unwrap_or(s).to_string(),
    }
}

fn location_enabled() -> Option<bool> {
    let out = output(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            r"(Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name Value -ErrorAction SilentlyContinue).Value",
        ],
    ).ok()?;
    match out.trim() {
        "Allow" => Some(true),
        "Deny" => Some(false),
        _ => None,
    }
}

fn snapshot() -> Value {
    json!({
        "timezone": timezone(),
        "timezones": timezones(),
        "automatic_timezone": null,
        "languages": [session_language()],
        "location_enabled": location_enabled(),
        "accessibility": false
    })
}

pub unsafe fn rf_browser_machine() -> *mut c_char {
    let mut state = snapshot();
    state["platform"] = json!("windows");
    let win_ver = output(
        "powershell",
        &["-NoProfile", "-Command", "[System.Environment]::OSVersion.Version.ToString()"],
    )
    .unwrap_or_else(|_| "10.0".into());
    state["version"] = json!(win_ver);
    state["cores"] = json!(std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1));
    state["locale"] = json!(locale());
    let proxy = ["ALL_PROXY", "all_proxy", "HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .unwrap_or_default();
    state["system_proxy"] = json!(proxy);
    response(Ok(state))
}

pub unsafe fn rf_system_read() -> *mut c_char {
    response(Ok(snapshot()))
}

pub unsafe fn rf_system_open(pane: *const c_char) -> *mut c_char {
    response(open_settings(input(pane)))
}

pub unsafe fn rf_system_timezone(zone: *const c_char) -> *mut c_char {
    response((|| {
        let requested = input(zone).trim();
        let target_win_tz = if requested.contains('/') {
            iana_to_windows_tz(requested).ok_or("无效的时区。")?
        } else if timezones().iter().any(|t| t == requested) {
            requested
        } else {
            return Err("无效的时区。".into());
        };
        output("tzutil", &["/s", target_win_tz])?;
        let current_raw = raw_windows_timezone();
        if current_raw.to_lowercase() != target_win_tz.to_lowercase() {
            return Err("时区修改未通过核验。".into());
        }
        Ok(json!({
            "state": snapshot(),
            "message": "时区已成功应用并核验。"
        }))
    })())
}

fn language_matches(current: &str, requested: &str) -> bool {
    !requested.is_empty()
        && (current == requested
            || current
                .strip_prefix(requested)
                .is_some_and(|s| s.starts_with('-')))
}

pub unsafe fn rf_system_language(requested: *const c_char) -> *mut c_char {
    let requested = input(requested);
    if requested.is_empty() || requested.contains(';') || requested.len() < 2 {
        return response(Err("语言无效。".into()));
    }
    let current = session_language();
    if language_matches(&current, requested) {
        response(Ok(json!({
            "state": snapshot(),
            "message": "当前系统显示语言已符合要求。"
        })))
    } else {
        response(Err("请在 Windows 设置中修改显示语言，注销并重新登录后生效。".into()))
    }
}

pub unsafe fn rf_system_region(requested: *const c_char, region: *const c_char) -> *mut c_char {
    let requested_lang = input(requested);
    let requested_loc = input(region);
    let current_lang = session_language();
    let current_loc = locale();
    if (current_lang == requested_lang || current_lang.starts_with(requested_lang)) && current_loc == requested_loc {
        response(Ok(json!({
            "state": snapshot(),
            "message": "当前系统语言与区域格式已核验。"
        })))
    } else {
        response(Err("请在 Windows 区域设置中调整国家和格式，注销并重新登录后生效。".into()))
    }
}

pub unsafe fn rf_system_location() -> *mut c_char {
    response((|| {
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                r"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name Value -Value Deny -ErrorAction SilentlyContinue",
            ])
            .output();
        if location_enabled() == Some(false) {
            Ok(json!({
                "state": snapshot(),
                "message": "Windows 定位权限已成功禁用并核验。"
            }))
        } else {
            let _ = open_settings("location");
            Err("请在打开的 Windows 设置窗口中手动关闭定位权限。".into())
        }
    })())
}

pub fn claude_processes() -> Result<Vec<crate::cleaner::ClaudeProcess>, String> {
    let mut result = Vec::new();
    let script = r#"Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'Claude.exe' -or $_.Name -eq 'claude.exe' -or (($_.Name -eq 'node.exe' -or $_.Name -eq 'bun.exe') -and $_.CommandLine -like '*@anthropic-ai*claude-code*') } | Select-Object ProcessId, Name, ExecutablePath"#;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output();
    if let Ok(out) = out {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("ProcessId") || line.starts_with("---") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            if let Ok(pid) = parts[0].parse::<i32>() {
                let name = if parts.len() > 1 { parts[1] } else { "claude.exe" };
                let display_name = if name.eq_ignore_ascii_case("Claude.exe") {
                    "Claude Desktop".to_string()
                } else {
                    "Claude Code".to_string()
                };
                let exe_path = if parts.len() > 2 {
                    parts[2..].join(" ")
                } else {
                    name.to_string()
                };
                result.push(crate::cleaner::ClaudeProcess {
                    pid,
                    name: display_name,
                    executable: exe_path,
                });
            }
        }
    }
    Ok(result)
}

pub fn kill_process(pid: i32) -> Result<(), String> {
    let status = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => Err(format!("无法终止进程 {pid}。")),
    }
}

pub unsafe fn rf_browser_running(profile: *const c_char) -> *mut c_char {
    let profile_str = input(profile);
    let target = format!("--user-data-dir={profile_str}");
    let script = format!(
        r#"Get-CimInstance Win32_Process -Filter "Name = 'chrome.exe'" | Where-Object {{ $_.CommandLine -like '*{target}*' }} | Measure-Object | Select-Object -ExpandProperty Count"#
    );
    let count: i32 = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or(0);
    response(Ok(json!({"running": count > 0})))
}

pub unsafe fn rf_browser_quit(profile: *const c_char) -> *mut c_char {
    let profile_str = input(profile);
    let target = format!("--user-data-dir={profile_str}");
    let script = format!(
        r#"Get-CimInstance Win32_Process -Filter "Name = 'chrome.exe'" | Where-Object {{ $_.CommandLine -like '*{target}*' }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force }}"#
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output();
    response(Ok(json!({"ok": true})))
}
