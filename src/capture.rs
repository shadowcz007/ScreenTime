use crate::screenshot;
use crate::siliconflow;
use crate::logger;
use crate::models::{ActivityLog, SystemContext, SystemInfo};
use crate::config::Config;
use crate::context; // 新增
use crate::service_state::ServiceStateManager;
use chrono::Local;
use std::error::Error;
use std::time::Duration;
use std::sync::Arc;
use tokio::time::{interval, sleep};

/// 原有的截屏循环（已废弃，保留用于内部使用）
async fn run_capture_loop(config: Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("启动后5秒开始第一次截屏...");
    
    // 等待5秒后开始第一次截屏
    sleep(Duration::from_secs(5)).await;
    
    // 执行第一次截屏
    let timestamp = Local::now();
    let screenshot_path = config.screenshot_dir.join(format!("screenshot_{}.png", timestamp.format("%Y%m%d_%H%M%S")));
    let screenshot_path_str = screenshot_path.to_str().unwrap_or("screenshot.png");
    
    // 确定图片处理参数
    let target_width = if config.image_target_width > 0 {
        Some(config.image_target_width)
    } else {
        None
    };
    
    // 确定是否启用灰度转换
    let grayscale = config.image_grayscale && !config.no_image_grayscale;
    
    // 截屏
    match screenshot::capture_screenshot_with_options(screenshot_path_str, target_width, grayscale) {
        Ok(_) => {
            println!("第一次截图已保存: {}", screenshot_path_str);
            
            // 等待一段时间确保文件写入完成
            sleep(Duration::from_millis(500)).await;
            
            // 调用SiliconFlow API分析截图
            let ctx_original = context::collect_system_context().await;
            let ctx_text = context::format_context_as_text(&ctx_original);
            
            // 转换context格式到models格式
            let ctx = convert_context_to_models(&ctx_original);
            
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
        match screenshot::capture_screenshot_with_options(screenshot_path_str, target_width, grayscale) {
            Ok(_) => {
                println!("截图已保存: {}", screenshot_path_str);
                
                // 等待一段时间确保文件写入完成
                sleep(Duration::from_millis(500)).await;
                
                // 调用SiliconFlow API分析截图
                let ctx_original = context::collect_system_context().await;
                let ctx_text = context::format_context_as_text(&ctx_original);
                
                // 转换context格式到models格式
                let ctx = convert_context_to_models(&ctx_original);
                
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

/// 带状态管理的截屏循环
pub async fn run_capture_loop_with_state(
    config: Config, 
    state_manager: Arc<ServiceStateManager>
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 启动带状态管理的截屏循环...");
    
    // 确保截图目录存在
    tokio::fs::create_dir_all(&config.screenshot_dir).await?;
    
    // 等待5秒后开始第一次截屏
    println!("启动后5秒开始第一次截屏...");
    sleep(Duration::from_secs(5)).await;
    
    // 检查是否应该开始截屏
    if !state_manager.should_capture().await {
        println!("⏹️ 服务未启动，截屏循环退出");
        return Ok(());
    }
    
    // 执行第一次截屏
    if let Err(e) = perform_capture(&config, &state_manager).await {
        eprintln!("第一次截屏失败: {}", e);
    }
    
    println!("开始间隔循环，间隔: {} 秒", config.interval);
    
    // 开始间隔循环
    let mut interval_timer = interval(Duration::from_secs(config.interval));
    
    loop {
        // 等待下一个时间点
        interval_timer.tick().await;
        
        // 检查服务状态
        if !state_manager.should_capture().await {
            println!("⏹️ 服务已停止，截屏循环退出");
            break;
        }
        
        // 执行截屏
        if let Err(e) = perform_capture(&config, &state_manager).await {
            eprintln!("截屏失败: {}", e);
            // 截屏失败时短暂休眠再继续
            sleep(Duration::from_secs(5)).await;
        }
    }
    
    println!("✅ 截屏循环正常退出");
    Ok(())
}

/// 执行单次截屏操作
async fn perform_capture(
    config: &Config, 
    state_manager: &Arc<ServiceStateManager>
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let timestamp = Local::now();
    let screenshot_path = config.screenshot_dir.join(format!("screenshot_{}.png", timestamp.format("%Y%m%d_%H%M%S")));
    let screenshot_path_str = screenshot_path.to_str().unwrap_or("screenshot.png");
    
    // 确定图片处理参数
    let target_width = if config.image_target_width > 0 {
        Some(config.image_target_width)
    } else {
        None
    };
    
    // 确定是否启用灰度转换
    let grayscale = config.image_grayscale && !config.no_image_grayscale;
    
    // 截屏
    screenshot::capture_screenshot_with_options(screenshot_path_str, target_width, grayscale)?;
    println!("📷 截图已保存: {}", screenshot_path_str);
    
    // 等待一段时间确保文件写入完成
    sleep(Duration::from_millis(500)).await;
    
    // 调用SiliconFlow API分析截图
    let ctx_original = context::collect_system_context().await;
    let ctx_text = context::format_context_as_text(&ctx_original);
    
    // 转换context格式到models格式
    let ctx = convert_context_to_models(&ctx_original);
    
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
            println!("🔍 分析结果: {}", description);
            
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
                Ok(_) => println!("💾 日志已保存"),
                Err(e) => eprintln!("保存日志时出错: {}", e),
            }
            
            // 更新截屏计数
            if let Err(e) = state_manager.increment_capture_count().await {
                eprintln!("更新截屏计数时出错: {}", e);
            }
        },
        Err(e) => {
            eprintln!("分析截图时出错: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// 将context模块的SystemContext转换为models模块的SystemContext
fn convert_context_to_models(ctx: &context::SystemContext) -> SystemContext {
    SystemContext {
        active_app: ctx.active_window.as_ref().and_then(|w| w.app_name.clone()),
        window_title: ctx.active_window.as_ref().and_then(|w| w.window_title.clone()),
        system_info: Some(SystemInfo {
            hostname: ctx.hostname.clone(),
            username: Some(ctx.username.clone()),
            platform: ctx.os_name.clone(),
        }),
        timestamp: Local::now(), // 使用当前时间作为时间戳
    }
}