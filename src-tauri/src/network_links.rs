// [INPUT]: Skill 的 providers.json 固定配置、资源编号与系统 URL 打开工具。
// [OUTPUT]: 两个原始供应商地址的 URL 组装、仓库 Skill 安装入口与 Claude 核心域名只读连通性诊断。
// [POS]: 应用网络页原生接口；不读取密钥、不执行 SSH、不接收任意 URL，诊断不上传凭据。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
#[cfg(not(windows))]
use std::process::Command;

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
        return Ok("https://github.com/liseami/CCBaga/tree/main/skills/ccbaga-network#installation".into());
    }
    if !["webshare", "dmit"].contains(&provider) {
        return Err("无效的供应商入口。".into());
    }
    let configured: serde_json::Value = serde_json::from_str(include_str!("../../skills/ccbaga-network/providers.json"))
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
        { return crate::windows::open_external_url(&url); }
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

pub(crate) async fn diagnose_urls(targets: &[(&str, &str)]) -> Vec<DomainCheckResult> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(2))
        .timeout(Duration::from_secs(8))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => {
            return targets.iter().map(|(domain, _)| DomainCheckResult {
                domain: domain.to_string(),
                ok: false,
                status_code: None,
                latency_ms: None,
                ip: None,
                message: "无法创建网络客户端".into(),
            }).collect();
        }
    };

    let mut results = Vec::new();
    for (domain, url) in targets {
        let started = Instant::now();
        let ip = match std::net::ToSocketAddrs::to_socket_addrs(&(*domain, 443)) {
            Ok(mut addrs) => addrs.next().map(|a| a.ip().to_string()),
            Err(_) => None,
        };

        match client.get(*url).send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let latency = started.elapsed().as_millis() as u64;
                let (ok, msg) = if (200..400).contains(&status) {
                    (true, "连接正常".into())
                } else if status == 401 || status == 404 {
                    (true, "服务响应正常".into())
                } else if status == 403 {
                    (false, "受到安全拦截或限制".into())
                } else {
                    (false, "服务响应异常".into())
                };
                results.push(DomainCheckResult {
                    domain: domain.to_string(),
                    ok,
                    status_code: Some(status),
                    latency_ms: Some(latency),
                    ip,
                    message: msg,
                });
            }
            Err(err) => {
                let latency = started.elapsed().as_millis() as u64;
                let msg = if err.is_timeout() {
                    "请求超时"
                } else if err.is_connect() {
                    "连接失败或代理阻断"
                } else {
                    "网络连接失败"
                };
                results.push(DomainCheckResult {
                    domain: domain.to_string(),
                    ok: false,
                    status_code: None,
                    latency_ms: Some(latency),
                    ip,
                    message: msg.into(),
                });
            }
        }
    }
    results
}

#[tauri::command]
pub async fn network_diagnose_domains() -> Result<Vec<DomainCheckResult>, String> {
    let targets = [
        ("claude.ai", "https://claude.ai/"),
        ("api.anthropic.com", "https://api.anthropic.com/"),
        ("downloads.claude.ai", "https://downloads.claude.ai/"),
    ];
    Ok(diagnose_urls(&targets).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    #[test]
    fn fixed_resources_preserve_original_destinations() {
        assert_eq!(provider_url("dmit").unwrap(), "https://www.dmit.io/aff.php?aff=28800");
        assert_eq!(provider_url("webshare").unwrap(), "https://www.webshare.io/?referral_code=qlm1mt9xibc9");
        assert_eq!(provider_url("skill").unwrap(), "https://github.com/liseami/CCBaga/tree/main/skills/ccbaga-network#installation");
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
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" || line == "\n" {
                        break;
                    }
                }
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK");
            }
        });
        let target = format!("http://{address}/");
        let results = tauri::async_runtime::block_on(diagnose_urls(&[("127.0.0.1", &target)]));
        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
        assert_eq!(results[0].status_code, Some(200));
        assert_eq!(results[0].message, "连接正常");
    }
}
