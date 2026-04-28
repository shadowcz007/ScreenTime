use crate::config::Config;
use crate::context;
use crate::logger;
use crate::models::{ActivityLog, SystemContext, SystemInfo};
use crate::screenshot;
use crate::service_state::ServiceStateManager;
use crate::siliconflow;
use chrono::Local;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};

/// 生成截图路径并确保目录存在
fn generate_screenshot_path(
    config: &Config,
    timestamp: &chrono::DateTime<chrono::Local>,
) -> Result<PathBuf, std::io::Error> {
    let screenshot_dir = config.get_screenshot_dir();

    // 确保截图目录存在
    if !screenshot_dir.exists() {
        fs::create_dir_all(&screenshot_dir)?;
    }

    let screenshot_path = screenshot_dir.join(format!(
        "screenshot_{}.png",
        timestamp.format("%Y%m%d_%H%M%S")
    ));
    Ok(screenshot_path)
}

/// 带状态管理的截屏循环
pub async fn run_capture_loop_with_state(
    config: Config,
    state_manager: Arc<ServiceStateManager>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut config = config;
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
    let mut current_interval_secs = config.interval.max(1);
    let mut interval_timer = interval(Duration::from_secs(current_interval_secs));

    loop {
        // 等待下一个时间点
        interval_timer.tick().await;

        // 运行时自动重载 .env 配置
        if let Ok(changed) = config.reload_from_dotenv_and_args() {
            if changed {
                let new_interval_secs = config.interval.max(1);
                if new_interval_secs != current_interval_secs {
                    current_interval_secs = new_interval_secs;
                    interval_timer = interval(Duration::from_secs(current_interval_secs));
                    println!("🔄 检测到 .env 变更，截屏间隔已更新为 {} 秒", current_interval_secs);
                }
            }
        }

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
    state_manager: &Arc<ServiceStateManager>,
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

    println!("────────── {} ──────────", timestamp.format("%H:%M:%S"));

    // 确定图片处理参数
    let target_width = if config.image_target_width > 0 {
        Some(config.image_target_width)
    } else {
        None
    };

    // 确定是否启用灰度转换
    let grayscale = config.image_grayscale && !config.no_image_grayscale;

    // 获取当前活跃窗口信息，用于智能选择屏幕
    let ctx_for_screenshot = context::collect_system_context(config).await;

    // 截屏 - 使用智能截图功能
    screenshot::capture_screenshot_smart(
        screenshot_path_str,
        target_width,
        grayscale,
        ctx_for_screenshot.active_window.as_ref(),
    )?;
    println!("📷 截图已保存: {}", screenshot_path_str);

    // 等待一段时间确保文件写入完成
    sleep(Duration::from_millis(500)).await;

    // 调用SiliconFlow API分析截图（带重试机制）
    let analysis_result =
        analyze_screenshot_with_retry(config, screenshot_path_str, &timestamp).await?;

    // 创建活动日志
    let ctx_original = context::collect_system_context(config).await;
    let ctx = convert_context_to_models(&ctx_original);

    // 是否保留截图：显式开关或 test_prompt 模式强制保留
    let should_keep = config.keep_screenshots || config.test_prompt.is_some();

    let screenshot_path_for_log = if should_keep {
        Some(screenshot_path_str.to_string())
    } else {
        None
    };

    let log = ActivityLog {
        timestamp,
        description: analysis_result.description,
        context: Some(ctx),
        screenshot_path: screenshot_path_for_log,
        model: Some(config.model.clone()),
        token_usage: analysis_result.token_usage,
    };

    // 保存日志
    match logger::save_activity_log(&log, config) {
        Ok(_) => println!("💾 日志已保存"),
        Err(e) => eprintln!("保存日志时出错: {}", e),
    }

    // 非保留模式：删除截图文件（无论分析成功或失败都执行到此）
    if !should_keep {
        if let Err(e) = std::fs::remove_file(screenshot_path_str) {
            eprintln!("删除截图失败: {}", e);
        } else {
            println!("🧹 已删除截图: {}", screenshot_path_str);
        }
    }

    // 更新截屏计数
    if let Err(e) = state_manager.increment_capture_count().await {
        eprintln!("更新截屏计数时出错: {}", e);
    }

    Ok(())
}

/// 带重试机制的截图分析
async fn analyze_screenshot_with_retry(
    config: &Config,
    screenshot_path_str: &str,
    timestamp: &chrono::DateTime<chrono::Local>,
) -> Result<siliconflow::AnalysisResult, Box<dyn Error + Send + Sync>> {
    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAYS: [u64; 5] = [5, 15, 30, 45, 60]; // 重试延迟：5秒、15秒、30秒

    // 获取系统上下文和历史记录
    let ctx_original = context::collect_system_context(config).await;
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

        match siliconflow::analyze_screenshot_with_prompt(
            &config.api_key,
            &config.api_url,
            &config.model,
            screenshot_path_str,
            &config.prompt,
            Some(&ctx_text),
            activity_history.as_deref(),
            config.api_timeout,
        )
        .await
        {
            Ok(analysis_result) => {
                println!("✅ 分析成功:");
                for line in analysis_result.description.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        println!("   {}", trimmed);
                    }
                }
                if let Some(ref token_usage) = analysis_result.token_usage {
                    println!(
                        "   Token: 输入 {}, 输出 {}, 总计 {} · 耗时 {:.2}s · {}",
                        token_usage.prompt_tokens.unwrap_or(0),
                        token_usage.completion_tokens.unwrap_or(0),
                        token_usage.total_tokens.unwrap_or(0),
                        analysis_result.processing_time.as_secs_f64(),
                        timestamp.format("%Y-%m-%d %H:%M:%S")
                    );
                } else {
                    println!(
                        "   耗时 {:.2}s · {}",
                        analysis_result.processing_time.as_secs_f64(),
                        timestamp.format("%Y-%m-%d %H:%M:%S")
                    );
                }
                return Ok(analysis_result);
            }
            Err(e) => {
                last_error = Some(e);
                let error_msg = last_error.as_ref().unwrap();

                // 不管什么错误都重试
                eprintln!(
                    "❌ 分析失败 (第 {}/{} 次): {}",
                    attempt, MAX_RETRIES, error_msg
                );

                if attempt < MAX_RETRIES {
                    let delay = RETRY_DELAYS[attempt as usize - 1];
                    println!("⏳ 等待 {} 秒后重试...", delay);
                    sleep(Duration::from_secs(delay)).await;
                } else {
                    eprintln!("❌ 达到最大重试次数，分析失败");
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
        window_title: ctx
            .active_window
            .as_ref()
            .and_then(|w| w.window_title.clone()),
        system_info: Some(SystemInfo {
            hostname: ctx.hostname.clone(),
            username: Some(ctx.username.clone()),
            platform: ctx.os_name.clone(),
        }),
        timestamp: Local::now(), // 使用当前时间作为时间戳
    }
}
