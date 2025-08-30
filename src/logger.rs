use crate::models::ActivityLog;
use crate::config::Config;
use std::error::Error;
use std::fs::{self, File};
use std::io::BufWriter;

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
    
    println!("📝 日志已保存到: {}", daily_log_path.display());
    
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