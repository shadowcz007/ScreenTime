//! 向 OpenClaw Gateway 的 /hooks/wake 上报 ScreenTime 摘要

use crate::config::Config;
use crate::logger;
use chrono::{Duration, Local};
use std::error::Error;
use std::time::Duration as StdDuration;

/// POST /hooks/wake 请求体
#[derive(serde::Serialize)]
struct WakeBody<'a> {
    text: &'a str,
    mode: &'static str,
}

/// 向 OpenClaw 发送 wake 请求（url 为完整 webhook 地址，如 https://host:port/hooks/wake）
pub async fn send_wake(
    url: &str,
    token: &str,
    text: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(15))
        .build()?;
    let body = WakeBody { text, mode: "now" };
    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("OpenClaw wake 请求失败: {} {}", status, body).into());
    }
    Ok(())
}

/// 上报循环：按配置间隔读取近期日志并发送到 OpenClaw
pub async fn run_reporter_loop(config: Config) {
    if !config.openclaw_enabled() {
        return;
    }
    let url = config.openclaw_url.as_ref().unwrap().clone();
    let token = config.openclaw_token.as_ref().unwrap().clone();
    let interval_minutes = config.openclaw_report_interval_minutes;
    let interval_duration = tokio::time::Duration::from_secs(interval_minutes * 60);

    println!(
        "📤 OpenClaw 上报已启用：每 {} 分钟向 {} 发送摘要",
        interval_minutes,
        url
    );

    let mut interval = tokio::time::interval(interval_duration);
    interval.tick().await; // 首次立即跳过，避免启动瞬间就发一条

    loop {
        interval.tick().await;

        let since = Local::now() - Duration::minutes(interval_minutes as i64);
        let logs = match logger::load_activity_logs_since(&config, since) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("⚠️ 读取活动日志失败，跳过本次 OpenClaw 上报: {}", e);
                continue;
            }
        };
        let text = logger::format_logs_for_openclaw(&logs, interval_minutes);

        if let Err(e) = send_wake(&url, &token, &text).await {
            eprintln!("⚠️ OpenClaw 上报失败: {}", e);
        } else {
            println!();
            println!("📤 OpenClaw 上报成功，本周期 {} 条记录", logs.len());
            println!();
        }
    }
}
