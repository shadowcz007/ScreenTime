use crate::models::ActivityLog;
use crate::config::Config;
use chrono::Local;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};

/// 保存活动日志（按日期分类存储）
pub fn save_activity_log(log: &ActivityLog, config: &Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 获取当前日期（YYYY-MM-DD格式）
    let date = log.timestamp.format("%Y-%m-%d").to_string();
    
    // 确保日志目录存在
    let logs_dir = config.get_logs_dir();
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }
    
    // 获取当日日志文件路径
    let daily_log_path = config.get_daily_log_path(&date);
    
    // 读取当日已有日志
    let mut logs: Vec<ActivityLog> = if daily_log_path.exists() {
        let file = File::open(&daily_log_path)?;
        serde_json::from_reader(file)?
    } else {
        Vec::new()
    };
    
    // 添加新日志
    logs.push(log.clone());
    
    // 保存日志
    let file = File::create(&daily_log_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &logs)?;

    // 同步保存可读 Markdown 日志
    save_activity_log_markdown(log, config)?;
    
    println!("📝 日志已保存到: {}", daily_log_path.display());
    
    Ok(())
}

/// 保存可读的 Markdown 活动日志（按日期追加）
fn save_activity_log_markdown(
    log: &ActivityLog,
    config: &Config,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let date = log.timestamp.format("%Y-%m-%d").to_string();
    let logs_md_dir = config.get_data_dir().join("logs_md");
    if !logs_md_dir.exists() {
        fs::create_dir_all(&logs_md_dir)?;
    }

    let daily_md_path = logs_md_dir.join(format!("{}.md", date));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&daily_md_path)?;

    let status_line = if log.description.trim().is_empty() {
        "失败/空结果".to_string()
    } else {
        "成功".to_string()
    };

    let app = log
        .context
        .as_ref()
        .and_then(|ctx| ctx.active_app.clone())
        .unwrap_or_else(|| "未知软件".to_string());

    let title = log
        .context
        .as_ref()
        .and_then(|ctx| ctx.window_title.clone())
        .unwrap_or_else(|| "-".to_string());

    let token_line = match &log.token_usage {
        Some(token) => format!(
            "输入 {} / 输出 {} / 总计 {}",
            token.prompt_tokens.unwrap_or(0),
            token.completion_tokens.unwrap_or(0),
            token.total_tokens.unwrap_or(0)
        ),
        None => "-".to_string(),
    };

    let screenshot_line = match &log.screenshot_path {
        Some(path) => path.clone(),
        None => "已删除".to_string(),
    };

    let md = format!(
        "## {}\n\n- 状态: {}\n- 软件: {}\n- 窗口: {}\n- 模型: {}\n- Token: {}\n- 截图: {}\n\n### AI 输出\n> {}\n\n---\n\n",
        log.timestamp.format("%H:%M:%S"),
        status_line,
        app,
        title,
        log.model.clone().unwrap_or_else(|| "-".to_string()),
        token_line,
        screenshot_line,
        log.description.replace('\n', "\n> ")
    );

    file.write_all(md.as_bytes())?;
    Ok(())
}



/// 读取指定日期的活动日志
pub fn load_daily_activity_logs(config: &Config, date: &str) -> Result<Vec<ActivityLog>, Box<dyn Error + Send + Sync>> {
    let daily_log_path = config.get_daily_log_path(date);
    
    if !daily_log_path.exists() {
        return Ok(Vec::new());
    }
    
    let file = File::open(daily_log_path)?;
    let logs: Vec<ActivityLog> = serde_json::from_reader(file)?;
    Ok(logs)
}

/// 读取最近N天的日志
pub fn load_recent_daily_logs(config: &Config, days: u32) -> Result<Vec<ActivityLog>, Box<dyn Error + Send + Sync>> {
    use chrono::{Local, Duration};
    
    let mut all_logs = Vec::new();
    let today = Local::now().date_naive();
    
    for i in 0..days {
        let date = today - Duration::days(i as i64);
        let date_str = date.format("%Y-%m-%d").to_string();
        
        match load_daily_activity_logs(config, &date_str) {
            Ok(mut logs) => all_logs.append(&mut logs),
            Err(_) => continue, // 忽略不存在的日志文件
        }
    }
    
    // 按时间排序
    all_logs.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    
    Ok(all_logs)
}

/// 读取指定时间点以来的活动日志（用于 OpenClaw 上报）
pub fn load_activity_logs_since(
    config: &Config,
    since: chrono::DateTime<Local>,
) -> Result<Vec<ActivityLog>, Box<dyn Error + Send + Sync>> {
    let today = Local::now().date_naive();
    let date_str = today.format("%Y-%m-%d").to_string();
    let logs = load_daily_activity_logs(config, &date_str)?;
    Ok(logs
        .into_iter()
        .filter(|log| log.timestamp >= since)
        .collect())
}

/// 将活动日志格式化为 OpenClaw /hooks/agent 的 message 内容
pub fn format_logs_for_openclaw(logs: &[ActivityLog], interval_minutes: u64) -> String {
    if logs.is_empty() {
        return format!(
            "用户电脑设备在过去{}分钟内的活动无新记录。",
            interval_minutes
        );
    }
    let mut s = format!("用户电脑设备在过去{}分钟内的活动摘要（共{}条）：\n", interval_minutes, logs.len());
    for (i, log) in logs.iter().enumerate() {
        s.push_str(&format!(
            "{}. {} {}\n",
            i + 1,
            log.timestamp.format("%Y-%m-%d %H:%M:%S"),
            log.description.trim()
        ));
    }
    s
}

/// 获取最近N条活动日志的timestamp和description，用于AI分析的上下文
pub fn get_recent_activity_context(config: &Config, count: usize) -> Result<String, Box<dyn Error + Send + Sync>> {
    // 读取最近3天的日志
    let logs = load_recent_daily_logs(config, 3)?;
    
    if logs.is_empty() {
        return Ok("暂无历史活动记录".to_string());
    }
    
    // 获取最后N条记录（最新的记录在最后）
    let recent_logs: Vec<&ActivityLog> = logs.iter().rev().take(count).collect();
    
    let mut context = String::new();
    context.push_str("【用户最近的活动历史】\n");
    
    for (index, log) in recent_logs.iter().rev().enumerate() {
        context.push_str(&format!(
            "{}. 时间: {}\n   描述: {}\n\n",
            index + 1,
            log.timestamp.format("%Y-%m-%d %H:%M:%S"),
            log.description.trim()
        ));
    }
    
    Ok(context)
}