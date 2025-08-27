mod screenshot;
mod siliconflow;
mod logger;
mod models;
mod capture;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 设置截屏间隔（秒）
    let interval_seconds = 60; // 每分钟截屏一次
    
    println!("开始截屏监控，间隔: {} 秒", interval_seconds);
    
    // 运行截屏循环
    capture::run_capture_loop(interval_seconds).await?;
    
    Ok(())
}