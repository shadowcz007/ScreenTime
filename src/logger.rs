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