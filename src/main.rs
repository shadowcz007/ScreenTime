mod screenshot;
mod siliconflow;
mod logger;
mod models;
mod capture;
mod config;
mod context; // 新增
mod permissions; // 新增权限模块

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 ScreenTime 启动中...\n");
    
    // 首先检查并请求必要权限
    println!("第一步：权限检查");
    let _permission_status = permissions::ensure_permissions().await?;
    println!("✅ 权限检查通过！\n");
    
    let config = config::Config::from_args();
    
    println!("📋 配置信息:");
    println!("  - 监控间隔: {} 秒", config.interval);
    println!("  - 使用模型: {}", config.model);
    println!("  - 截图目录: {:?}", config.screenshot_dir);
    println!("  - 日志路径: {:?}", config.log_path);
    println!();
    
    // 确保截图目录存在
    tokio::fs::create_dir_all(&config.screenshot_dir).await?;
    
    // 运行截屏循环
    capture::run_capture_loop(config).await?;
    
    Ok(())
}