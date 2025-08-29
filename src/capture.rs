use crate::screenshot;
use crate::siliconflow;
use crate::logger;
use crate::models::ActivityLog;
use crate::config::Config;
use crate::context; // 新增
use chrono::Local;
use std::error::Error;
use std::time::Duration;
use tokio::time::{interval, sleep};

pub async fn run_capture_loop(config: Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("启动后5秒开始第一次截屏...");
    
    // 等待5秒后开始第一次截屏
    sleep(Duration::from_secs(5)).await;
    
    // 执行第一次截屏
    let timestamp = Local::now();
    let screenshot_path = config.screenshot_dir.join(format!("screenshot_{}.png", timestamp.format("%Y%m%d_%H%M%S")));
    let screenshot_path_str = screenshot_path.to_str().unwrap_or("screenshot.png");
    
    // 截屏
    match screenshot::capture_screenshot(screenshot_path_str) {
        Ok(_) => {
            println!("第一次截图已保存: {}", screenshot_path_str);
            
            // 等待一段时间确保文件写入完成
            sleep(Duration::from_millis(500)).await;
            
            // 调用SiliconFlow API分析截图
            let ctx = context::collect_system_context().await;
            let ctx_text = context::format_context_as_text(&ctx);
            
            // 获取历史活动记录（最近5条）
            let log_path_str = config.log_path.to_str().unwrap_or("activity_log.json");
            let activity_history = match logger::get_recent_activity_context(log_path_str, 5) {
                Ok(history) => Some(history),
                Err(e) => {
                    eprintln!("获取历史活动记录时出错: {}", e);
                    None
                }
            };

            match siliconflow::analyze_screenshot_with_prompt(
                &config.api_key,
                &config.api_url,
                &config.model,
                screenshot_path_str,
                &config.prompt,
                Some(&ctx_text), // 系统上下文
                activity_history.as_deref(), // 用户活动历史
            ).await {
                Ok(description) => {
                    println!("第一次分析结果: {}", description);
                    
                    // 创建活动日志
                    let log = ActivityLog {
                        timestamp,
                        description,
                        context: Some(ctx), // 记录上下文
                        screenshot_path: Some(screenshot_path_str.to_string()),
                    };
                    
                    // 保存日志
                    let log_path_str = config.log_path.to_str().unwrap_or("activity_log.json");
                    match logger::save_activity_log(&log, log_path_str) {
                        Ok(_) => println!("第一次日志已保存"),
                        Err(e) => eprintln!("保存日志时出错: {}", e),
                    }
                },
                Err(e) => eprintln!("分析截图时出错: {}", e),
            }
        },
        Err(e) => eprintln!("截屏时出错: {}", e),
    }
    
    println!("开始间隔循环，间隔: {} 秒", config.interval);
    
    // 开始间隔循环
    let mut interval_timer = interval(Duration::from_secs(config.interval));
    
    loop {
        // 等待下一个时间点
        interval_timer.tick().await;
        
        // 生成文件名
        let timestamp = Local::now();
        let screenshot_path = config.screenshot_dir.join(format!("screenshot_{}.png", timestamp.format("%Y%m%d_%H%M%S")));
        let screenshot_path_str = screenshot_path.to_str().unwrap_or("screenshot.png");
        
        // 截屏
        match screenshot::capture_screenshot(screenshot_path_str) {
            Ok(_) => {
                println!("截图已保存: {}", screenshot_path_str);
                
                // 等待一段时间确保文件写入完成
                sleep(Duration::from_millis(500)).await;
                
                // 调用SiliconFlow API分析截图
                let ctx = context::collect_system_context().await;
                let ctx_text = context::format_context_as_text(&ctx);
                
                // 获取历史活动记录（最近5条）
                let log_path_str = config.log_path.to_str().unwrap_or("activity_log.json");
                let activity_history = match logger::get_recent_activity_context(log_path_str, 5) {
                    Ok(history) => Some(history),
                    Err(e) => {
                        eprintln!("获取历史活动记录时出错: {}", e);
                        None
                    }
                };

                match siliconflow::analyze_screenshot_with_prompt(
                    &config.api_key,
                    &config.api_url,
                    &config.model,
                    screenshot_path_str,
                    &config.prompt,
                    Some(&ctx_text), // 系统上下文
                    activity_history.as_deref(), // 用户活动历史
                ).await {
                    Ok(description) => {
                        println!("分析结果: {}", description);
                        
                        // 创建活动日志
                        let log = ActivityLog {
                            timestamp,
                            description,
                            context: Some(ctx), // 记录上下文
                            screenshot_path: Some(screenshot_path_str.to_string()),
                        };
                        
                        // 保存日志
                        let log_path_str = config.log_path.to_str().unwrap_or("activity_log.json");
                        match logger::save_activity_log(&log, log_path_str) {
                            Ok(_) => println!("日志已保存"),
                            Err(e) => eprintln!("保存日志时出错: {}", e),
                        }
                    },
                    Err(e) => eprintln!("分析截图时出错: {}", e),
                }
            },
            Err(e) => eprintln!("截屏时出错: {}", e),
        }
    }
}