// [INPUT]: Windows Chromium 引擎、资源目录与共用资料/随机令牌会话。
// [OUTPUT]: 本地引擎发现与启动、用户主动设置默认浏览器。
// [POS]: browser 编排的 Windows 平台实现；只接收 URL/文件入口，不执行用户提供的 shell。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
fn register(_: &Path) -> Result<(), String> {
    Ok(())
}

fn binary(engine: &Path) -> PathBuf {
    engine.join("chrome.exe")
}

fn packaged_engine(resources: &Path) -> PathBuf {
    let legacy = resources.join("browser-runtime/WindowsBrowser");
    if binary(&legacy).is_file() {
        return legacy;
    }
    resources.join("WindowsBrowser")
}

fn engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let resources = app.path().resource_dir().map_err(|_| "资源目录不可用。")?;
    let packaged = packaged_engine(&resources);
    if binary(&packaged).is_file() || !cfg!(debug_assertions) {
        return Ok(packaged);
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../browser-runtime/WindowsBrowser"))
}

fn prepare_engine(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let engine = engine(app)?;
    if !binary(&engine).is_file() {
        return Err(
            "Windows 指纹浏览器引擎缺失。请先运行 python scripts/fetch-browser.py 下载并准备引擎。"
                .into(),
        );
    }
    Ok(engine)
}

fn version(engine: &Path) -> String {
    fs::read_to_string(engine.join("version.txt"))
        .unwrap_or_default()
        .trim()
        .into()
}

pub fn open_urls(app: tauri::AppHandle, args: Vec<String>) {
    let urls = args
        .into_iter()
        .skip(1)
        .filter(|s| {
            s.starts_with("https://")
                || s.starts_with("http://")
                || s.starts_with("file://")
                || Path::new(s).is_absolute()
        })
        .collect::<Vec<_>>();
    std::thread::spawn(move || {
        if let Err(error) = start_windows_companion(&app, &urls) {
            eprintln!("CCBaga-B: {error}");
        }
    });
}

fn start_windows_companion(app: &tauri::AppHandle, urls: &[String]) -> Result<(), String> {
    let state = app.state::<Browser>();
    let _guard = state.gate.lock().unwrap();
    let engine = prepare_engine(app)?;
    let root = root(app)?;
    let profile = root.join("profile");
    profile::check_path(&root, &profile)?;
    let config = config::load(&root)?;
    let machine = config::machine()?;
    config.validate(&machine)?;
    if !profile::running(&profile)? {
        profile::preferences(&profile, &config, &machine)?;
    }
    write_launch_args(&root, &config, &machine, &profile, &version(&engine))?;
    let page = open_session(app, &config, &machine, version(&engine))?;
    let mut child = Command::new(binary(&engine))
        .args(config::arguments(
            &config,
            &machine,
            &profile,
            &page,
            &version(&engine),
        ))
        .args(urls)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            "无法启动 Windows Chromium 浏览器。请检查引擎文件与依赖。".to_string()
        })?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn default_read() -> Result<DefaultBrowser, String> {
    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r"(Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice' -Name ProgId -ErrorAction SilentlyContinue).ProgId",
        ])
        .output();
    let prog_id = match out {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => String::new(),
    };
    let is_default = prog_id.contains("CCBaga") || prog_id.contains("RequirementFrench");
    Ok(DefaultBrowser {
        is_default,
        http: prog_id.clone(),
        https: prog_id,
        installed: true,
        app: std::env::current_exe()
            .map_err(|_| "无法定位 CCBaga-A。")?
            .display()
            .to_string(),
    })
}

#[tauri::command]
pub async fn browser_set_default(app: tauri::AppHandle) -> Result<DefaultBrowser, String> {
    run(move || {
        prepare_engine(&app)?;
        let _ = Command::new("cmd")
            .args(["/c", "start", "", "ms-settings:defaultapps"])
            .spawn();
        default_read()
    })
    .await
}

