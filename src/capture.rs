use crate::screenshot;
use crate::siliconflow;
use crate::fastvlm_local;
use crate::logger;
use crate::models::{ActivityLog, SystemContext, SystemInfo};
use crate::config::Config;
use crate::context;
use std::fs;
use std::path::PathBuf;
use crate::service_state::ServiceStateManager;
use chrono::Local;
use std::error::Error;
use std::time::Duration;
use std::sync::Arc;
use tokio::time::{interval, sleep};
use std::sync::Mutex;

/// 全局FastVLM服务实例
static FASTVLM_SERVICE: Mutex<Option<Arc<fastvlm_local::FastVLMService>>> = Mutex::new(None);

/// 初始化FastVLM服务（如果配置了本地模型路径）
pub async fn initialize_fastvlm_if_needed(config: &Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !config.is_using_fastvlm() {
        return Ok(());
    }

    let model_dir = config.get_fastvlm_model_dir().unwrap().clone();
    
    // 检查是否已经初始化
    {
        let service_guard = FASTVLM_SERVICE.lock().unwrap();
        if service_guard.is_some() {
            return Ok(());
        }
    }

    println!("🤖 正在初始化FastVLM本地模型服务...");
    
    // 创建FastVLM服务
    let service = fastvlm_local::create_fastvlm_service(model_dir).await?;
    let service_arc = Arc::new(service);
    
    // 保存到全局变量
    {
        let mut service_guard = FASTVLM_SERVICE.lock().unwrap();
        *service_guard = Some(service_arc);
    }

    println!("✅ FastVLM本地模型服务初始化成功");
    Ok(())
}

/// 生成截图路径并确保目录存在
fn generate_screenshot_path(config: &Config, timestamp: &chrono::DateTime<chrono::Local>) -> Result<PathBuf, std::io::Error> {
    let screenshot_dir = config.get_screenshot_dir();
    
    // 确保截图目录存在
    if !screenshot_dir.exists() {
        fs::create_dir_all(&screenshot_dir)?;
    }
    
    let screenshot_path = screenshot_dir.join(format!("screenshot_{}.png", timestamp.format("%Y%m%d_%H%M%S")));
    Ok(screenshot_path)
}

/// 带状态管理的截屏循环
pub async fn run_capture_loop_with_state(
    config: Config, 
    state_manager: Arc<ServiceStateManager>
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 启动带状态管理的截屏循环...");
    
    // 确保截图目录存在
    tokio::fs::create_dir_all(&config.get_screenshot_dir()).await?;
    
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
    let screenshot_path = match generate_screenshot_path(config, &timestamp) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("创建截图目录失败: {}", e);
            return Err(Box::new(e));
        }
    };
    let screenshot_path_str = screenshot_path.to_str().unwrap_or("screenshot.png");
    
    // 确定图片处理参数
    let target_width = if config.image_target_width > 0 {
        Some(config.image_target_width)
    } else {
        None
    };
    
    // 确定是否启用灰度转换
    let grayscale = config.image_grayscale && !config.no_image_grayscale;
    
    // 获取当前活跃窗口信息，用于智能选择屏幕
    let ctx_for_screenshot = context::collect_system_context().await;
    
    // 截屏 - 使用智能截图功能
    screenshot::capture_screenshot_smart(
        screenshot_path_str, 
        target_width, 
        grayscale, 
        ctx_for_screenshot.active_window.as_ref()
    )?;
    println!("📷 截图已保存: {}", screenshot_path_str);
    
    // 等待一段时间确保文件写入完成
    sleep(Duration::from_millis(500)).await;
    
    // 调用分析API（支持三种计算方式）
    let analysis_result = analyze_screenshot_with_retry(
        config,
        screenshot_path_str,
        &timestamp
    ).await?;
    
    // 创建活动日志
    let ctx_original = context::collect_system_context().await;
    let ctx = convert_context_to_models(&ctx_original);
    
    let log = ActivityLog {
        timestamp,
        description: analysis_result.description,
        context: Some(ctx),
        screenshot_path: Some(screenshot_path_str.to_string()),
        model: Some(config.model.clone()),
        token_usage: analysis_result.token_usage,
    };
    
    // 保存日志
    match logger::save_activity_log(&log, config) {
        Ok(_) => println!("💾 日志已保存"),
        Err(e) => eprintln!("保存日志时出错: {}", e),
    }
    
    // 更新截屏计数
    if let Err(e) = state_manager.increment_capture_count().await {
        eprintln!("更新截屏计数时出错: {}", e);
    }
    
    Ok(())
}

// 使用models模块中的通用AnalysisResult类型

/// 带重试机制的截图分析（支持三种计算方式）
async fn analyze_screenshot_with_retry(
    config: &Config,
    screenshot_path_str: &str,
    _timestamp: &chrono::DateTime<chrono::Local>
) -> Result<crate::models::AnalysisResult, Box<dyn Error + Send + Sync>> {
    // 检查是否使用FastVLM本地模型
    if config.is_using_fastvlm() {
        return analyze_with_fastvlm(config, screenshot_path_str).await;
    }
    
    // 使用API方式（硅基流动或自定义URL）
    analyze_with_api_retry(config, screenshot_path_str).await
}

/// 使用FastVLM本地模型分析
async fn analyze_with_fastvlm(
    config: &Config,
    screenshot_path_str: &str
) -> Result<crate::models::AnalysisResult, Box<dyn Error + Send + Sync>> {
    println!("🤖 使用FastVLM本地模型分析截图...");
    
    // 获取FastVLM服务实例
    let service_opt = {
        let service_guard = FASTVLM_SERVICE.lock().unwrap();
        service_guard.clone()
    };
    
    let service = service_opt.ok_or("FastVLM服务未初始化")?;
    
    // 获取系统上下文和历史记录
    let ctx_original = context::collect_system_context().await;
    let ctx_text = context::format_context_as_text(&ctx_original);
    
    let activity_history = match logger::get_recent_activity_context(config, 5) {
        Ok(history) => Some(history),
        Err(e) => {
            eprintln!("获取历史活动记录时出错: {}", e);
            None
        }
    };
    
    // 使用FastVLM分析
    let fastvlm_result = service.analyze_screenshot_with_prompt(
        screenshot_path_str,
        &config.prompt,
        Some(&ctx_text),
        activity_history.as_deref(),
    ).await?;
    
    println!("✅ FastVLM分析成功: {}", fastvlm_result.description);
    
    Ok(crate::models::AnalysisResult {
        description: fastvlm_result.description,
        token_usage: fastvlm_result.token_usage,
    })
}

/// 使用API方式分析（带重试机制）
async fn analyze_with_api_retry(
    config: &Config,
    screenshot_path_str: &str
) -> Result<crate::models::AnalysisResult, Box<dyn Error + Send + Sync>> {
    println!("🌐 使用API方式分析截图 ({})", if config.api_url.contains("siliconflow") { "硅基流动" } else { "自定义API" });
    
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAYS: [u64; 3] = [5, 15, 30]; // 重试延迟：5秒、15秒、30秒
    
    // 获取系统上下文和历史记录
    let ctx_original = context::collect_system_context().await;
    let ctx_text = context::format_context_as_text(&ctx_original);
    
    // 获取历史活动记录（最近5条）
    let activity_history = match logger::get_recent_activity_context(config, 5) {
        Ok(history) => Some(history),
        Err(e) => {
            eprintln!("获取历史活动记录时出错: {}", e);
            None
        }
    };
    
    let mut last_error = None;
    
    for attempt in 1..=MAX_RETRIES {
        println!("🔍 尝试分析截图 (第 {}/{} 次)", attempt, MAX_RETRIES);
        
        // 检查API密钥是否存在
        let api_key = match &config.api_key {
            Some(key) => key,
            None => {
                eprintln!("❌ 使用API方式需要提供API密钥");
                return Err("API密钥未提供".into());
            }
        };
        
        match siliconflow::analyze_screenshot_with_prompt(
            api_key,
            &config.api_url,
            &config.model,
            screenshot_path_str,
            &config.prompt,
            Some(&ctx_text),
            activity_history.as_deref(),
        ).await {
            Ok(analysis_result) => {
                println!("✅ 分析成功: {}", analysis_result.description);
                if let Some(ref token_usage) = analysis_result.token_usage {
                    println!("Token使用情况 - 输入: {:?}, 输出: {:?}, 总计: {:?}", 
                        token_usage.prompt_tokens, 
                        token_usage.completion_tokens, 
                        token_usage.total_tokens);
                }
                return Ok(crate::models::AnalysisResult {
                    description: analysis_result.description,
                    token_usage: analysis_result.token_usage,
                });
            },
            Err(e) => {
                last_error = Some(e);
                let error_msg = last_error.as_ref().unwrap();
                
                // 判断是否为网络相关错误
                let is_network_error = error_msg.to_string().contains("connection") ||
                                     error_msg.to_string().contains("timeout") ||
                                     error_msg.to_string().contains("network") ||
                                     error_msg.to_string().contains("Connection refused") ||
                                     error_msg.to_string().contains("connection closed");
                
                if is_network_error {
                    eprintln!("🌐 网络错误 (第 {}/{} 次): {}", attempt, MAX_RETRIES, error_msg);
                    
                    if attempt < MAX_RETRIES {
                        let delay = RETRY_DELAYS[attempt as usize - 1];
                        println!("⏳ 等待 {} 秒后重试...", delay);
                        sleep(Duration::from_secs(delay)).await;
                    } else {
                        eprintln!("❌ 达到最大重试次数，分析失败");
                    }
                } else {
                    // 非网络错误，直接失败
                    eprintln!("❌ 非网络错误，停止重试: {}", error_msg);
                    break;
                }
            }
        }
    }
    
    // 所有重试都失败了
    Err(last_error.unwrap_or_else(|| "未知错误".into()))
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