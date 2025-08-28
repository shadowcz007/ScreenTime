use crate::models::ActivityLog;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn save_activity_log(log: &ActivityLog, file_path: &str) -> Result<(), Box<dyn Error>> {
    // 读取现有日志
    let mut logs: Vec<ActivityLog> = if Path::new(file_path).exists() {
        let file = File::open(file_path)?;
        serde_json::from_reader(file)?
    } else {
        Vec::new()
    };
    
    // 添加新日志
    logs.push(log.clone());
    
    // 保存日志
    let file = File::create(file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &logs)?;
    
    Ok(())
}

/// 读取活动日志文件
pub fn load_activity_logs(file_path: &str) -> Result<Vec<ActivityLog>, Box<dyn Error>> {
    if !Path::new(file_path).exists() {
        return Ok(Vec::new());
    }
    
    let file = File::open(file_path)?;
    let logs: Vec<ActivityLog> = serde_json::from_reader(file)?;
    Ok(logs)
}

/// 获取最近N条活动日志的timestamp和description，用于AI分析的上下文
pub fn get_recent_activity_context(file_path: &str, count: usize) -> Result<String, Box<dyn Error>> {
    let logs = load_activity_logs(file_path)?;
    
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