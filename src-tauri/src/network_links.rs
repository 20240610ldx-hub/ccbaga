// [INPUT]: Skill 的 providers.json 固定配置、资源编号与系统 URL 打开工具。
// [OUTPUT]: 两个原始供应商地址的 URL 组装、仓库 Skill 安装入口与 三个固定核心域名的并发只读 HTTP 诊断与真实连接对端（不是出口核验）。
// [POS]: 应用网络页原生接口；不读取密钥、不执行 SSH、不接收任意 URL，诊断不上传凭据。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::{Deserialize, Serialize};
#[cfg(not(windows))]
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainCheckResult {
    pub domain: String,
    pub ok: bool,
    pub status_code: Option<u16>,
    pub latency_ms: Option<u64>,
    pub ip: Option<String>,
    pub message: String,
}

fn provider_url(provider: &str) -> Result<String, String> {
    if provider == "skill" {
        return Ok(
            "https://github.com/liseami/CCBaga/tree/main/skills/ccbaga-network#installation".into(),
        );
    }
    if !["webshare", "dmit"].contains(&provider) {
        return Err("无效的供应商入口。".into());
    }
    let configured: serde_json::Value =
        serde_json::from_str(include_str!("../../skills/ccbaga-network/providers.json"))
            .map_err(|_| "无效的供应商入口。")?;
    let settings = &configured[provider];
    let mut url = reqwest::Url::parse(settings["base"].as_str().ok_or("无效的供应商入口。")?)
        .map_err(|_| "无效的供应商入口。")?;
    url.query_pairs_mut().append_pair(
        settings["parameter"].as_str().ok_or("无效的供应商入口。")?,
        settings["value"].as_str().ok_or("无效的供应商入口。")?,
    );
    Ok(url.to_string())
}

#[tauri::command]
pub async fn network_open_resource(provider: String) -> Result<(), String> {
    let url = provider_url(&provider)?;
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            return crate::windows::open_external_url(&url);
        }
        #[cfg(not(windows))]
        {
            let status = Command::new(if cfg!(target_os = "linux") {
                "xdg-open"
            } else {
                "/usr/bin/open"
            })
            .arg(url)
            .status()
            .map_err(|_| "无法打开供应商网站。")?;
            if status.success() {
                Ok(())
            } else {
                Err("无法打开供应商网站。".into())
            }
        }
    })
    .await
    .map_err(|_| "无法打开供应商网站。".to_string())?
}

fn valid_redirect_location(request_url: &str, location: Option<&str>) -> bool {
    let Some(loc) = location.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    if loc.starts_with('/') && !loc.starts_with("//") {
        return true;
    }
    let req_host = reqwest::Url::parse(request_url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_ascii_lowercase()));
    let loc_url = if let Some(rest) = loc.strip_prefix("//") {
        reqwest::Url::parse(&format!("https://{rest}")).ok()
    } else {
        reqwest::Url::parse(loc).ok()
    };
    let Some(loc_host) = loc_url.and_then(|u| u.host_str().map(|s| s.to_ascii_lowercase())) else {
        return false;
    };
    req_host.as_deref() == Some(loc_host.as_str())
        || loc_host == "claude.ai"
        || loc_host.ends_with(".claude.ai")
        || loc_host == "claude.com"
        || loc_host.ends_with(".claude.com")
        || loc_host == "anthropic.com"
        || loc_host.ends_with(".anthropic.com")
}

async fn diagnose(client: reqwest::Client, domain: String, url: String) -> DomainCheckResult {
    let started = Instant::now();
    let mut result = DomainCheckResult {
        domain,
        ok: false,
        status_code: None,
        latency_ms: None,
        ip: None,
        message: String::new(),
    };
    match client.get(&url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            // 展示真实连接对端，不把独立 DNS 查询误称为这次连接的路由。
            result.ip = response
                .remote_addr()
                .map(|address| address.ip().to_string());
            result.status_code = Some(status);
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok());
            let redirect_ok =
                !(300..400).contains(&status) || valid_redirect_location(&url, location);
            result.ok =
                ((200..400).contains(&status) && redirect_ok) || status == 401 || status == 404;
            result.message = match status {
                200..=299 => "连接正常",
                401 | 404 => "服务响应正常",
                300..=399 if redirect_ok => "服务要求重定向",
                300..=399 | 403 => "受到安全拦截或限制",
                _ => "服务响应异常",
            }
            .into();
        }
        Err(error) => {
            result.message = if error.is_timeout() {
                "请求超时"
            } else if error.is_connect() {
                "连接失败或代理阻断"
            } else {
                "网络连接失败"
            }
            .into();
        }
    }
    result.latency_ms = Some(started.elapsed().as_millis() as u64);
    result
}
pub(crate) async fn diagnose_urls(targets: &[(&str, &str)]) -> Vec<DomainCheckResult> {
    diagnose_urls_with_proxy(targets, "").await
}
pub(crate) async fn diagnose_urls_with_proxy(
    targets: &[(&str, &str)],
    proxy: &str,
) -> Vec<DomainCheckResult> {
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(8));
    let trimmed = proxy.trim();
    if !trimmed.is_empty() {
        if let Ok(p) = reqwest::Proxy::all(trimmed) {
            builder = builder.proxy(p);
        }
    }
    let client = match builder.build() {
        Ok(client) => client,
        Err(_) => {
            return targets
                .iter()
                .map(|(domain, _)| DomainCheckResult {
                    domain: domain.to_string(),
                    ok: false,
                    status_code: None,
                    latency_ms: None,
                    ip: None,
                    message: "无法创建网络客户端".into(),
                })
                .collect()
        }
    };
    // DNS 交由异步 HTTP 客户端；固定三个探针并发，避免阻塞运行时或串行等待 24 秒。
    let tasks: Vec<_> = targets
        .iter()
        .map(|(domain, url)| {
            tauri::async_runtime::spawn(diagnose(
                client.clone(),
                domain.to_string(),
                url.to_string(),
            ))
        })
        .collect();
    let mut results = Vec::new();
    for ((domain, _), task) in targets.iter().zip(tasks) {
        results.push(task.await.unwrap_or_else(|_| DomainCheckResult {
            domain: domain.to_string(),
            ok: false,
            status_code: None,
            latency_ms: None,
            ip: None,
            message: "网络连接失败".into(),
        }));
    }
    results
}

#[tauri::command]
pub async fn network_diagnose_domains(
    app: tauri::AppHandle,
) -> Result<Vec<DomainCheckResult>, String> {
    let targets = [
        ("claude.ai", "https://claude.ai/"),
        ("api.anthropic.com", "https://api.anthropic.com/"),
        ("downloads.claude.ai", "https://downloads.claude.ai/"),
    ];
    let proxy = tauri::async_runtime::spawn_blocking(move || {
        crate::browser::configured_or_system_proxy(&app)
    })
    .await
    .unwrap_or_default();
    Ok(diagnose_urls_with_proxy(&targets, &proxy).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    #[test]
    fn fixed_resources_preserve_original_destinations() {
        assert_eq!(
            provider_url("dmit").unwrap(),
            "https://www.dmit.io/aff.php?aff=28800"
        );
        assert_eq!(
            provider_url("webshare").unwrap(),
            "https://www.webshare.io/?referral_code=qlm1mt9xibc9"
        );
        assert_eq!(
            provider_url("skill").unwrap(),
            "https://github.com/liseami/CCBaga/tree/main/skills/ccbaga-network#installation"
        );
        assert!(provider_url("https://example.com").is_err());
    }

    #[test]
    fn domain_diagnostics_handles_mock_endpoints() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                loop {
                    line.clear();
                    if reader.read_line(&mut line).unwrap_or(0) == 0
                        || line == "\r\n"
                        || line == "\n"
                    {
                        break;
                    }
                }
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                );
            }
        });
        let target = format!("http://{address}/");
        let results = tauri::async_runtime::block_on(diagnose_urls(&[("127.0.0.1", &target)]));
        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
        assert_eq!(results[0].status_code, Some(200));
        assert_eq!(results[0].message, "连接正常");
    }

    #[test]
    fn probes_never_send_credentials_or_follow_redirects() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut reader = BufReader::new(&stream);
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    request.push_str(&line);
                    if line == "\r\n" {
                        break;
                    }
                }
                let lower = request.to_lowercase();
                assert!(!lower.contains("authorization:"));
                assert!(!lower.contains("cookie:"));
                let status = if request.starts_with("GET /blocked ") {
                    "403 Forbidden"
                } else if request.starts_with("GET /redirect ") {
                    "302 Found"
                } else {
                    "401 Unauthorized"
                };
                write!(stream,"HTTP/1.1 {status}\r\nLocation: http://127.0.0.1:1/never-follow\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            }
        });
        let urls = [
            format!("http://{address}/auth"),
            format!("http://{address}/blocked"),
            format!("http://{address}/redirect"),
        ];
        let results = tauri::async_runtime::block_on(diagnose_urls(&[
            ("auth", &urls[0]),
            ("blocked", &urls[1]),
            ("redirect", &urls[2]),
        ]));
        server.join().unwrap();
        assert!(results[0].ok);
        assert!(!results[1].ok);
        assert_eq!(results[2].status_code, Some(302));
        assert_eq!(results[2].message, "服务要求重定向");
        assert!(results
            .iter()
            .all(|result| result.ip.as_deref() == Some("127.0.0.1")));
        assert!(valid_redirect_location(
            "https://claude.ai/",
            Some("/login")
        ));
        assert!(valid_redirect_location(
            "https://claude.ai/",
            Some("https://claude.ai/login")
        ));
        assert!(valid_redirect_location(
            "https://platform.claude.com/",
            Some("https://console.anthropic.com/login")
        ));
        assert!(!valid_redirect_location(
            "https://claude.ai/",
            Some("http://198.18.0.1/portal")
        ));
    }
}
