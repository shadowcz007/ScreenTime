use crate::screenshot;
use crate::siliconflow;
use crate::logger;
use crate::models::ActivityLog;
use chrono::Local;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time::{interval, sleep};

pub async fn run_capture_loop(interval_seconds: u64) -> Result<(), Box<dyn Error>> {
    let mut interval_timer = interval(Duration::from_secs(interval_seconds));
    
    // 跳过第一次立即触发
    interval_timer.tick().await;
    
    loop {
        // 等待下一个时间点
        interval_timer.tick().await;
        
        // 生成文件名
        let timestamp = Local::now();
        let screenshot_path = format!("screenshots/screenshot_{}.png", timestamp.format("%Y%m%d_%H%M%S"));
        let log_path = "activity_log.json";
        
        // 截屏
        match screenshot::capture_screenshot(&screenshot_path) {
            Ok(_) => {
                println!("截图已保存: {}", screenshot_path);
                
                // 等待一段时间确保文件写入完成
                sleep(Duration::from_millis(500)).await;
                
                // 从环境变量获取SiliconFlow API密钥
                match env::var("SILICONFLOW_API_KEY") {
                    Ok(sf_api_key) => {
                        // 调用SiliconFlow API分析截图
                        match siliconflow::analyze_screenshot(&sf_api_key, &screenshot_path).await {
                            Ok(description) => {
                                println!("分析结果: {}", description);
                                
                                // 创建活动日志
                                let log = ActivityLog {
                                    timestamp,
                                    description,
                                };
                                
                                // 保存日志
                                match logger::save_activity_log(&log, &log_path) {
                                    Ok(_) => println!("日志已保存"),
                                    Err(e) => eprintln!("保存日志时出错: {}", e),
                                }
                            },
                            Err(e) => eprintln!("分析截图时出错: {}", e),
                        }
                    },
                    Err(_) => {
                        eprintln!("请设置SILICONFLOW_API_KEY环境变量以使用SiliconFlow视觉模型");
                    }
                }
            },
            Err(e) => eprintln!("截屏时出错: {}", e),
        }
    }
}