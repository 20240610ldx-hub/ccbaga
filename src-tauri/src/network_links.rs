// [INPUT]: Skill 的 providers.json 固定配置、资源编号与系统 URL 打开工具。
// [OUTPUT]: 两个原始供应商地址的 URL 组装、仓库 Skill 安装入口与四个固定核心域名的并发只读 HTTP 诊断与真实连接对端（不是出口核验）。
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
    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            // 展示真实连接对端，不把独立 DNS 查询误称为这次连接的路由。
            result.ip = response
                .remote_addr()
                .map(|address| address.ip().to_string());
            result.status_code = Some(status);
            // Cloudflare 对无脚本客户端返回可由浏览器完成的质询，说明已到达站点边缘；拦截规则不带此标记。
            let challenge = status == 403
                && response
                    .headers()
                    .get("cf-mitigated")
                    .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"challenge"));
            result.ok = (200..400).contains(&status) || status == 401 || status == 404 || challenge;
            result.message = match status {
                200..=299 => "连接正常",
                401 | 404 => "服务响应正常",
                300..=399 => "服务要求重定向",
                403 if challenge => "服务可达，需浏览器验证",
                403 => "受到安全拦截或限制",
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
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(8))
        .build();
    let client = match client {
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
    // DNS 交由异步 HTTP 客户端；固定探针并发，避免阻塞运行时或串行等待各自的 8 秒超时。
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
pub async fn network_diagnose_domains() -> Result<Vec<DomainCheckResult>, String> {
    Ok(diagnose_urls(&CLAUDE_TARGETS).await)
}

// downloads.claude.ai 是不允许列目录的对象存储，根路径恒为 403；探测官方安装器实际读取的公开版本号。
const CLAUDE_TARGETS: [(&str, &str); 4] = [
    ("claude.ai", "https://claude.ai/"),
    ("api.anthropic.com", "https://api.anthropic.com/"),
    ("platform.claude.com", "https://platform.claude.com/"),
    (
        "downloads.claude.ai",
        "https://downloads.claude.ai/claude-code-releases/latest",
    ),
];

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
    fn fixed_claude_targets_avoid_unlisted_bucket_root() {
        let domains: Vec<_> = CLAUDE_TARGETS.iter().map(|(domain, _)| *domain).collect();
        assert_eq!(
            domains,
            [
                "claude.ai",
                "api.anthropic.com",
                "platform.claude.com",
                "downloads.claude.ai"
            ]
        );
        for (domain, url) in CLAUDE_TARGETS {
            let parsed = reqwest::Url::parse(url).unwrap();
            assert_eq!(parsed.scheme(), "https");
            assert_eq!(parsed.host_str(), Some(domain));
        }
        assert_eq!(
            CLAUDE_TARGETS[3].1,
            "https://downloads.claude.ai/claude-code-releases/latest"
        );
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
            for _ in 0..4 {
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
                } else if request.starts_with("GET /challenge ") {
                    "403 Forbidden\r\ncf-mitigated: challenge"
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
            format!("http://{address}/challenge"),
        ];
        let results = tauri::async_runtime::block_on(diagnose_urls(&[
            ("auth", &urls[0]),
            ("blocked", &urls[1]),
            ("redirect", &urls[2]),
            ("challenge", &urls[3]),
        ]));
        server.join().unwrap();
        assert!(results[0].ok);
        assert!(!results[1].ok);
        assert_eq!(results[2].status_code, Some(302));
        assert_eq!(results[2].message, "服务要求重定向");
        assert!(results[3].ok);
        assert_eq!(results[3].status_code, Some(403));
        assert_eq!(results[3].message, "服务可达，需浏览器验证");
        assert_eq!(results[1].message, "受到安全拦截或限制");
        assert!(results
            .iter()
            .all(|result| result.ip.as_deref() == Some("127.0.0.1")));
    }
}
