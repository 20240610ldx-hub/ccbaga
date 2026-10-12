// [INPUT]: Windows 当前用户 SID、时区、注册表状态、系统工具与受限进程接口。
// [OUTPUT]: Windows 平台 JSON 桥；支持时区/语言/区域/定位状态、Win32 剪贴板、当前用户文件所有权和安全进程接口。
// [POS]: 仅原生识别与用户触发的系统操作；不含网络客户端，不向 UI 输出进程参数或代理秘密。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde_json::{json, Value};
use std::{
    ffi::{c_char, CStr, CString},
    process::{Command, Stdio},
};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
use windows_sys::Win32::Security::OWNER_SECURITY_INFORMATION;
use windows_sys::Win32::Security::{
    GetLengthSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
#[path = "windows_process.rs"]
mod processes;
pub(crate) use processes::ChildJob;
pub use processes::{claude_processes, kill_process, rf_browser_quit, rf_browser_running};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

#[link(name = "kernel32")]
extern "system" {
    fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> HANDLE;
    fn GlobalLock(hMem: HANDLE) -> *mut std::ffi::c_void;
    fn GlobalUnlock(hMem: HANDLE) -> i32;
    fn GlobalFree(hMem: HANDLE) -> HANDLE;
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
        GlobalFree(h_mem);
        return Err("无法锁定剪贴板内存。".into());
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, bytes);
    GlobalUnlock(h_mem);

    let owner = windows_sys::Win32::UI::WindowsAndMessaging::GetActiveWindow();
    if owner.is_null() || OpenClipboard(owner) == 0 {
        GlobalFree(h_mem);
        return Err("无法打开系统剪贴板。".into());
    }
    if EmptyClipboard() == 0 {
        CloseClipboard();
        GlobalFree(h_mem);
        return Err("剪贴板写入失败。".into());
    }
    let res = SetClipboardData(CF_UNICODETEXT, h_mem);
    CloseClipboard();

    if res.is_null() {
        GlobalFree(h_mem);
        Err("剪贴板写入失败。".into())
    } else {
        Ok(())
    }
}

pub unsafe fn rf_credentials_list() -> *mut c_char {
    response(Err(
        "Windows 凭据管理器不自动读取；文件登录记录可在清理预览中选择。".into(),
    ))
}
pub unsafe fn rf_credentials_remove(_: *const c_char) -> *mut c_char {
    response(Err("Windows 凭据管理器不支持自动删除。".into()))
}

// ----- 所有权：只比较 SID，不收集账户名称或凭据正文 -----
pub(super) unsafe fn process_sid(process: HANDLE) -> Option<Vec<u8>> {
    let mut token = std::ptr::null_mut();
    if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
        return None;
    }
    let mut size = 0;
    GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut size);
    let mut buffer = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
    let ok = size > 0
        && GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        ) != 0;
    let sid = if ok {
        let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        Some(
            std::slice::from_raw_parts(
                user.User.Sid.cast::<u8>(),
                GetLengthSid(user.User.Sid) as usize,
            )
            .to_vec(),
        )
    } else {
        None
    };
    CloseHandle(token);
    sid
}
pub(super) fn current_sid() -> Option<&'static Vec<u8>> {
    static SID: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    SID.get_or_init(|| unsafe { process_sid(GetCurrentProcess()) })
        .as_ref()
}
pub fn owned(path: &std::path::Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let mut owner = std::ptr::null_mut();
        let mut descriptor = std::ptr::null_mut();
        let status = GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut descriptor,
        );
        let matches = status == 0
            && !owner.is_null()
            && current_sid().is_some_and(|sid| {
                sid.as_slice()
                    == std::slice::from_raw_parts(owner.cast::<u8>(), GetLengthSid(owner) as usize)
            });
        if !descriptor.is_null() {
            LocalFree(descriptor.cast());
        }
        matches
    }
}
pub fn linked(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & 0x400 != 0
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

// ----- 共用地区目录所需的 IANA 与 Windows 时区映射 -----
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
    TZ_MAP
        .iter()
        .find(|(_, w)| w.eq_ignore_ascii_case(win_clean))
        .map(|(i, _)| *i)
}

pub fn timezone() -> String {
    let out = output("tzutil", &["/g"]).unwrap_or_default();
    crate::regions::get_regions()
        .iter()
        .find(|region| {
            region.locale == locale() && iana_to_windows_tz(region.timezone) == Some(out.as_str())
        })
        .map(|region| region.timezone)
        .or_else(|| windows_to_iana_tz(&out))
        .unwrap_or(&out)
        .to_string()
}

pub fn raw_windows_timezone() -> String {
    output("tzutil", &["/g"]).unwrap_or_default()
}

pub fn timezones() -> Vec<String> {
    let mut list = Vec::new();
    for (iana, _) in TZ_MAP {
        list.push((*iana).to_string());
    }
    list.sort();
    list.dedup();
    list
}

fn locale() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut value = [0u16; 85];
    let count = unsafe { GetUserDefaultLocaleName(value.as_mut_ptr(), value.len() as i32) };
    if count <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&value[..count as usize - 1]).replace('-', "_")
}
fn session_language() -> String {
    use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    let mut count = 0;
    let mut size = 0;
    unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            std::ptr::null_mut(),
            &mut size,
        );
    }
    let mut buffer = vec![0u16; size as usize];
    if size == 0
        || unsafe {
            GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                buffer.as_mut_ptr(),
                &mut size,
            )
        } == 0
    {
        return String::new();
    }
    let first = buffer.split(|c| *c == 0).next().unwrap_or_default();
    let language = String::from_utf16_lossy(first);
    match language.as_str() {
        "zh-CN" | "zh-SG" => "zh-Hans".into(),
        "zh-TW" | "zh-HK" => "zh-Hant".into(),
        _ => language,
    }
}
pub(super) fn registry_string(
    root: windows_sys::Win32::System::Registry::HKEY,
    key: &str,
    name: &str,
) -> Option<String> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut value = [0u16; 2048];
    let mut bytes = std::mem::size_of_val(&value) as u32;
    let status = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if status != 0 {
        return None;
    }
    Some(
        String::from_utf16_lossy(&value[..bytes as usize / 2])
            .trim_end_matches('\0')
            .into(),
    )
}
fn location_enabled() -> Option<bool> {
    use windows_sys::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    match registry_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location",
        "Value",
    )?
    .as_str()
    {
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
        &[
            "-NoProfile",
            "-Command",
            "[System.Environment]::OSVersion.Version.ToString()",
        ],
    )
    .unwrap_or_default();
    state["version"] = json!(win_ver);
    state["cores"] = json!(std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1));
    state["locale"] = json!(locale());
    let proxy = [
        "ALL_PROXY",
        "all_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ]
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
        let target_win_tz = iana_to_windows_tz(requested).ok_or("无效的时区。")?;
        if output("tzutil", &["/s", target_win_tz]).is_err() {
            open_settings("timezone")?;
            return Err("Windows 未允许修改时区；请在系统设置中完成授权后重试。".into());
        }
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
    current == requested
        || current
            .strip_prefix(requested)
            .is_some_and(|s| s.starts_with('-'))
}
fn region_settings(language: &str, region: Option<&str>) -> Result<Value, String> {
    let supported = crate::regions::get_regions();
    let selected = supported
        .iter()
        .find(|r| r.language == language && region.is_none_or(|v| r.locale == v))
        .ok_or("语言或区域无效。")?;
    let tag = selected.locale.replace('_', "-");
    let script = include_str!("../../scripts/windows-region.ps1").trim_start_matches('\u{feff}');
    let result = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("CC_REGION_LANGUAGE", &tag)
        .env("CC_REGION_LOCALE", region.unwrap_or("").replace('_', "-"))
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "Windows 语言设置无法执行。")?;
    if !result.status.success()
        || !String::from_utf8_lossy(&result.stdout).contains("CC_REGION_DONE=1")
    {
        open_settings("language")?;
        return Err("Windows 语言设置未完成；请检查系统设置中的语言包和权限。".into());
    }
    if region.is_some_and(|r| locale() != r) {
        return Err("Windows 区域格式修改未通过核验。".into());
    }
    if !language_matches(&session_language(), language) {
        open_settings("language")?;
        return Err(
            "首选语言与区域已保存；安装所需语言包并注销、重新登录后再核验显示语言。".into(),
        );
    }
    Ok(json!({"state": snapshot(), "message": "系统语言与区域已应用并核验。"}))
}
pub unsafe fn rf_system_language(language: *const c_char) -> *mut c_char {
    response(region_settings(input(language), None))
}
pub unsafe fn rf_system_region(language: *const c_char, locale: *const c_char) -> *mut c_char {
    response(region_settings(input(language), Some(input(locale))))
}

pub unsafe fn rf_system_location() -> *mut c_char {
    response((|| {
        if location_enabled() == Some(false) {
            Ok(json!({
                "state": snapshot(),
                "message": "Windows 定位权限已成功禁用并核验。"
            }))
        } else {
            open_settings("location")?;
            Err("请在打开的 Windows 设置窗口中手动关闭定位权限。".into())
        }
    })())
}
