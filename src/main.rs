mod screenshot;
mod siliconflow;
mod logger;
mod models;
mod capture;
mod config;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = config::Config::from_args();
    
    println!("开始截屏监控，间隔: {} 秒", config.interval);
    println!("使用模型: {}", config.model);
    println!("截图保存目录: {:?}", config.screenshot_dir);
    println!("日志保存路径: {:?}", config.log_path);
    
    // 确保截图目录存在
    tokio::fs::create_dir_all(&config.screenshot_dir).await?;
    
    // 运行截屏循环
    capture::run_capture_loop(config).await?;
    
    Ok(())
}