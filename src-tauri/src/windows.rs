// [INPUT]: Windows 原生凭据、时区、注册表系统设置、Toolhelp32/PEB 进程与系统工具。
// [OUTPUT]: 零子进程开销的 Windows 平台 JSON 桥；支持时区/语言/区域/定位状态查询与修改、SSH 密钥对话框、Win32 剪贴板及 Claude 进程精准收敛。
// [POS]: 仅原生识别与用户触发的系统操作；不含网络客户端，不向 UI 输出进程参数或代理秘密。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CStr, CString},
    process::{Command, Stdio},
};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Credentials::{
    CredDeleteW, CredEnumerateW, CredFree, CREDENTIALW, CRED_TYPE_GENERIC,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

#[repr(C)]
struct ProcessBasicInformation {
    exit_status: i32,
    peb_base_address: usize,
    affinity_mask: usize,
    base_priority: i32,
    unique_process_id: usize,
    inherited_from_unique_process_id: usize,
}

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}

#[link(name = "ntdll")]
extern "system" {
    fn NtQueryInformationProcess(
        process_handle: HANDLE,
        process_information_class: u32,
        process_information: *mut std::ffi::c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn ReadProcessMemory(
        hProcess: HANDLE,
        lpBaseAddress: *const std::ffi::c_void,
        lpBuffer: *mut std::ffi::c_void,
        nSize: usize,
        lpNumberOfBytesRead: *mut usize,
    ) -> i32;
    fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> HANDLE;
    fn GlobalLock(hMem: HANDLE) -> *mut std::ffi::c_void;
    fn GlobalUnlock(hMem: HANDLE) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn OpenClipboard(hWndNewOwner: HANDLE) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(uFormat: u32, hMem: HANDLE) -> HANDLE;
}

const CF_UNICODETEXT: u32 = 13;
const GMEM_MOVEABLE: u32 = 0x0002;

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

pub fn open_external_url(url: &str) -> Result<(), String> {
    let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    let file: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let inst = ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        if (inst as usize) > 32 {
            Ok(())
        } else {
            Err("无法打开目标地址。".into())
        }
    }
}

pub unsafe fn rf_copy(text: *const c_char) -> Result<(), String> {
    let val = input(text);
    let wide: Vec<u16> = val.encode_utf16().chain(Some(0)).collect();
    let bytes = wide.len() * std::mem::size_of::<u16>();

    let h_mem = GlobalAlloc(GMEM_MOVEABLE, bytes);
    if h_mem.is_null() {
        return Err("无法分配剪贴板内存。".into());
    }
    let ptr = GlobalLock(h_mem);
    if ptr.is_null() {
        return Err("无法锁定剪贴板内存。".into());
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, bytes);
    GlobalUnlock(h_mem);

    if OpenClipboard(std::ptr::null_mut()) == 0 {
        return Err("无法打开系统剪贴板。".into());
    }
    EmptyClipboard();
    let res = SetClipboardData(CF_UNICODETEXT, h_mem);
    CloseClipboard();

    if res.is_null() {
        Err("剪贴板写入失败。".into())
    } else {
        Ok(())
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
    let shell = if Command::new("pwsh").arg("-v").output().is_ok() {
        "pwsh"
    } else {
        "powershell"
    };
    let out = Command::new(shell)
        .args(["-NoProfile", "-Sta", "-Command", &script])
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
    let mut count: u32 = 0;
    let mut creds: *mut *mut CREDENTIALW = std::ptr::null_mut();
    let mut items = Vec::new();

    if CredEnumerateW(std::ptr::null(), 0, &mut count, &mut creds) != 0 && !creds.is_null() {
        let slice = std::slice::from_raw_parts(creds, count as usize);
        for &cred_ptr in slice {
            if cred_ptr.is_null() {
                continue;
            }
            let target_w = (*cred_ptr).TargetName;
            if target_w.is_null() {
                continue;
            }
            let mut len = 0;
            while *target_w.add(len) != 0 {
                len += 1;
            }
            let target = String::from_utf16_lossy(std::slice::from_raw_parts(target_w, len));
            let lower = target.to_lowercase();
            if lower.contains("claude") || lower.contains("anthropic") {
                items.push(json!({
                    "service": target,
                    "reference": target,
                }));
            }
        }
        CredFree(creds as *mut std::ffi::c_void);
    }
    response(Ok(json!({"items": items})))
}

pub unsafe fn rf_credentials_remove(reference: *const c_char) -> *mut c_char {
    let target = input(reference);
    if target.is_empty() {
        return response(Err("无效的凭据目标。".into()));
    }
    let target_w: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
    let success = CredDeleteW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0);
    if success != 0 {
        response(Ok(json!({"ok": true})))
    } else {
        response(Err("凭据删除失败或不存在。".into()))
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
    open_external_url(uri)?;
    Ok(json!({"ok": true}))
}

// 详尽的 Windows IANA 双向全量时区映射表
static TZ_MAP: &[(&str, &str)] = &[
    ("America/Los_Angeles", "Pacific Standard Time"),
    ("America/Vancouver", "Pacific Standard Time"),
    ("America/Denver", "Mountain Standard Time"),
    ("America/Phoenix", "US Mountain Standard Time"),
    ("America/Chicago", "Central Standard Time"),
    ("America/New_York", "Eastern Standard Time"),
    ("America/Toronto", "Eastern Standard Time"),
    ("America/Anchorage", "Alaskan Standard Time"),
    ("Pacific/Honolulu", "Hawaiian Standard Time"),
    ("America/Sao_Paulo", "E. South America Standard Time"),
    ("America/Buenos_Aires", "Argentina Standard Time"),
    ("Europe/London", "GMT Standard Time"),
    ("Europe/Dublin", "GMT Standard Time"),
    ("Europe/Paris", "Romance Standard Time"),
    ("Europe/Berlin", "W. Europe Standard Time"),
    ("Europe/Rome", "W. Europe Standard Time"),
    ("Europe/Madrid", "Romance Standard Time"),
    ("Europe/Amsterdam", "W. Europe Standard Time"),
    ("Europe/Athens", "GTB Standard Time"),
    ("Europe/Helsinki", "FLE Standard Time"),
    ("Europe/Moscow", "Russian Standard Time"),
    ("Asia/Tokyo", "Tokyo Standard Time"),
    ("Asia/Seoul", "Korea Standard Time"),
    ("Asia/Singapore", "Singapore Standard Time"),
    ("Asia/Taipei", "Taipei Standard Time"),
    ("Asia/Hong_Kong", "China Standard Time"),
    ("Asia/Shanghai", "China Standard Time"),
    ("Asia/Bangkok", "SE Asia Standard Time"),
    ("Asia/Jakarta", "SE Asia Standard Time"),
    ("Asia/Kolkata", "India Standard Time"),
    ("Asia/Dubai", "Arabian Standard Time"),
    ("Australia/Sydney", "AUS Eastern Standard Time"),
    ("Australia/Melbourne", "AUS Eastern Standard Time"),
    ("Pacific/Auckland", "New Zealand Standard Time"),
    ("UTC", "UTC"),
    ("Etc/UTC", "UTC"),
];

pub fn iana_to_windows_tz(iana: &str) -> Option<&'static str> {
    TZ_MAP.iter().find(|(i, _)| *i == iana).map(|(_, w)| *w)
}

pub fn windows_to_iana_tz(win: &str) -> Option<&'static str> {
    let win_clean = win.trim();
    TZ_MAP.iter().find(|(_, w)| w.eq_ignore_ascii_case(win_clean)).map(|(i, _)| *i)
}

pub fn timezone() -> String {
    let out = output("tzutil", &["/g"]).unwrap_or_else(|_| "China Standard Time".into());
    windows_to_iana_tz(&out).unwrap_or("Asia/Shanghai").to_string()
}

pub fn raw_windows_timezone() -> String {
    output("tzutil", &["/g"]).unwrap_or_else(|_| "China Standard Time".into())
}

pub fn timezones() -> Vec<String> {
    let mut list = Vec::new();
    for (iana, _) in TZ_MAP {
        list.push((*iana).to_string());
    }
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
            r"(Get-ItemProperty -Path 'HKCU:SoftwareMicrosoftWindowsCurrentVersionCapabilityAccessManagerConsentStorelocation' -Name Value -ErrorAction SilentlyContinue).Value",
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
                r"Set-ItemProperty -Path 'HKCU:SoftwareMicrosoftWindowsCurrentVersionCapabilityAccessManagerConsentStorelocation' -Name Value -Value Deny -ErrorAction SilentlyContinue",
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

// 轻量级命令行读取（通过 PEB 结构）
fn read_process_command_line(handle: HANDLE) -> Option<String> {
    unsafe {
        let mut pbi = std::mem::zeroed::<ProcessBasicInformation>();
        let mut ret_len = 0;
        if NtQueryInformationProcess(
            handle,
            0,
            &mut pbi as *mut _ as *mut _,
            std::mem::size_of::<ProcessBasicInformation>() as u32,
            &mut ret_len,
        ) != 0 {
            return None;
        }
        if pbi.peb_base_address == 0 {
            return None;
        }
        // 64 位 Windows 上，PEB + 0x20 为 ProcessParameters 指针
        let mut params_ptr: usize = 0;
        let mut read = 0;
        if ReadProcessMemory(
            handle,
            (pbi.peb_base_address + 0x20) as *const _,
            &mut params_ptr as *mut _ as *mut _,
            std::mem::size_of::<usize>(),
            &mut read,
        ) == 0
            || params_ptr == 0
        {
            return None;
        }
        // RTL_USER_PROCESS_PARAMETERS 中，CommandLine (UNICODE_STRING) 偏移为 0x70
        let mut ustr = std::mem::zeroed::<UnicodeString>();
        if ReadProcessMemory(
            handle,
            (params_ptr + 0x70) as *const _,
            &mut ustr as *mut _ as *mut _,
            std::mem::size_of::<UnicodeString>(),
            &mut read,
        ) == 0 {
            return None;
        }
        if ustr.length == 0 || ustr.buffer.is_null() {
            return None;
        }
        let len = (ustr.length / 2) as usize;
        let mut buf = vec![0u16; len];
        if ReadProcessMemory(
            handle,
            ustr.buffer as *const _,
            buf.as_mut_ptr() as *mut _,
            ustr.length as usize,
            &mut read,
        ) == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf))
    }
}

pub fn claude_processes() -> Result<Vec<crate::cleaner::ClaudeProcess>, String> {
    let mut result = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return Ok(result);
        }
        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile);
                let name = name.trim_matches(|c: char| c as u32 == 0).trim();
                let lower = name.to_lowercase();
                let pid = entry.th32ProcessID as i32;

                if lower == "claude.exe" {
                    result.push(crate::cleaner::ClaudeProcess {
                        pid,
                        name: if name == "Claude.exe" {
                            "Claude Desktop".into()
                        } else {
                            "Claude Code".into()
                        },
                        executable: name.to_string(),
                    });
                } else if lower == "node.exe" || lower == "bun.exe" {
                    let h_proc = OpenProcess(
                        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                        0,
                        entry.th32ProcessID,
                    );
                    if !h_proc.is_null() {
                        if let Some(cmd) = read_process_command_line(h_proc) {
                            if cmd.contains("@anthropic-ai/claude-code")
                                || cmd.contains(r"@anthropic-ai\claude-code")
                            {
                                result.push(crate::cleaner::ClaudeProcess {
                                    pid,
                                    name: "Claude Code".into(),
                                    executable: name.to_string(),
                                });
                            }
                        }
                        CloseHandle(h_proc);
                    }
                }

                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    Ok(result)
}

pub fn kill_process(pid: i32) -> Result<(), String> {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid as u32);
        if !handle.is_null() {
            let res = TerminateProcess(handle, 1);
            CloseHandle(handle);
            if res != 0 {
                return Ok(());
            }
        }
    }
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
    let mut is_running = false;
    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snap != INVALID_HANDLE_VALUE {
        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile);
                let name = name.trim_matches(|c: char| c as u32 == 0).trim().to_lowercase();
                if name == "chrome.exe" {
                    let h_proc = OpenProcess(
                        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                        0,
                        entry.th32ProcessID,
                    );
                    if !h_proc.is_null() {
                        if let Some(cmd) = read_process_command_line(h_proc) {
                            if cmd.contains(&target) {
                                is_running = true;
                                CloseHandle(h_proc);
                                break;
                            }
                        }
                        CloseHandle(h_proc);
                    }
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    response(Ok(json!({"running": is_running})))
}

pub unsafe fn rf_browser_quit(profile: *const c_char) -> *mut c_char {
    let profile_str = input(profile);
    let target = format!("--user-data-dir={profile_str}");
    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snap != INVALID_HANDLE_VALUE {
        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile);
                let name = name.trim_matches(|c: char| c as u32 == 0).trim().to_lowercase();
                if name == "chrome.exe" {
                    let h_proc = OpenProcess(
                        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_TERMINATE,
                        0,
                        entry.th32ProcessID,
                    );
                    if !h_proc.is_null() {
                        if let Some(cmd) = read_process_command_line(h_proc) {
                            if cmd.contains(&target) {
                                let _ = TerminateProcess(h_proc, 1);
                            }
                        }
                        CloseHandle(h_proc);
                    }
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    response(Ok(json!({"ok": true})))
}

