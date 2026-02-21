mod screenshot;
mod siliconflow;
mod logger;
mod models;
mod capture;
mod config;
mod context; // 新增
mod permissions; // 新增权限模块
mod mcp_service; // MCP服务模块
mod test_prompt; // 新增测试prompt模块
mod service_state; // 服务状态管理
mod standalone_service; // 独立截屏服务
mod window_tracker; // 窗口追踪模块
mod openclaw; // OpenClaw webhook 上报

use std::error::Error;

use mcp_service::ScreenTimeService;
use standalone_service::{StandaloneService, ServiceController};
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 ScreenTime 启动中...\n");
    
    let config = config::Config::from_args();
    
    // 检查是否为测试prompt模式
    if let Some(_) = &config.test_prompt {
        println!("🧪 启动测试prompt模式");
        return test_prompt::run_test_prompt(config).await;
    }
    
    if config.mcp {
        // MCP 服务器模式
        println!("🔗 启动 MCP 服务器模式");
        return run_mcp_server(config).await;
    }
    
    // 默认启动独立截屏服务模式
    println!("🚀 启动独立截屏服务模式");
    run_standalone_service(config).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_windows_compatibility() {
        // 创建一个测试配置，避免解析命令行参数
        let config = config::Config {
            api_key: "test_key".to_string(),
            api_url: "https://api.siliconflow.cn/v1/chat/completions".to_string(),
            model: "THUDM/GLM-4.1V-9B-Thinking".to_string(),
            prompt: "测试提示".to_string(),
            interval: 60,
            data_dir: None,
            state_path: None,
            image_target_width: 1440,
            image_grayscale: true,
            no_image_grayscale: false,
            mcp: false,
            mcp_port: 6672,
            test_prompt: None,
            test_log_path: PathBuf::from("test_log.json"),
            socket_path: None,
            control_port: 5830,
        };
        
        #[cfg(windows)]
        {
            println!("Windows系统测试:");
            println!("控制端口: {}", config.get_control_port());
            assert!(config.get_control_port() > 0);
        }
        
        #[cfg(unix)]
        {
            println!("Unix系统测试:");
            println!("Socket路径: {:?}", config.get_socket_path());
            assert!(!config.get_socket_path().to_string_lossy().is_empty());
        }
    }
}

async fn run_mcp_server(config: config::Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bind_address = format!("127.0.0.1:{}", config.mcp_port);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".to_string().into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("🌐 启动 MCP SSE 服务器，地址: {}", bind_address);

    // 确保必要的目录存在
    tokio::fs::create_dir_all(&config.get_screenshot_dir()).await?;
    
    // 检查独立服务是否已启动，如果没有则自动启动并确保运行
    let service_controller = ServiceController::new(&config);
    let mut capture_running = false;

    match service_controller
        .send_command(crate::models::ServiceCommand::Status)
        .await
    {
        Ok(response) => {
            capture_running =
                process_service_status_response(response, &service_controller).await;
        }
        Err(_) => {
            println!("🚀 独立截屏服务未运行，正在自动启动...");
            let config_clone = config.clone();
            tokio::spawn(async move {
                if let Err(e) = start_standalone_service_background(config_clone).await {
                    eprintln!("启动独立服务失败: {}", e);
                }
            });

            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            match service_controller
                .send_command(crate::models::ServiceCommand::Status)
                .await
            {
                Ok(response) => {
                    capture_running = process_service_status_response(
                        response,
                        &service_controller,
                    )
                    .await;
                }
                Err(e) => {
                    eprintln!("⚠️ 独立截屏服务启动失败: {}", e);
                    eprintln!(
                        "   MCP服务仍可使用，但截屏功能需要手动启动独立服务"
                    );
                }
            }
        }
    }

    if !capture_running {
        println!(
            "⚠️ 独立截屏服务未能自动启动，截屏功能暂不可用（可使用 monitor.start 手动启动）"
        );
    }

    let server_config = SseServerConfig {
        bind: bind_address.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
        sse_keep_alive: None,
    };

    let (sse_server, router) = SseServer::new(server_config);
    
    // 添加 CORS 中间件
    use tower_http::cors::{Any, CorsLayer};
    use axum::http::HeaderName;
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(vec![
            HeaderName::from_static("content-type"),
            HeaderName::from_static("authorization"),
        ])
        .allow_credentials(false);
    
    let router_with_cors = router.layer(cors);
    
    let listener = tokio::net::TcpListener::bind(sse_server.config.bind).await?;
    let ct = sse_server.config.ct.child_token();

    let http = axum::serve(listener, router_with_cors).with_graceful_shutdown(async move {
        ct.cancelled().await;
        tracing::info!("sse server cancelled");
    });
    tokio::spawn(async move {
        if let Err(e) = http.await {
            tracing::error!(error = %e, "sse server shutdown with error");
        }
    });

    let cfg = config.clone();
    let cancel_token = sse_server.with_service(move || ScreenTimeService::new(cfg.clone()));

    println!("✅ MCP 服务器启动成功！ SSE: /sse, POST: /message");
    println!("🌐 CORS 已启用，支持跨域访问");
    println!("按 Ctrl+C 停止服务器...");

    tokio::signal::ctrl_c().await?;
    cancel_token.cancel();
    Ok(())
}

async fn run_standalone_service(config: config::Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 首先检查并请求必要权限
    println!("第一步：权限检查");
    let _permission_status = permissions::ensure_permissions().await?;
    println!("✅ 权限检查通过！\n");
    
    println!("📋 配置信息:");
    println!("  - 监控间隔: {} 秒", config.interval);
    println!("  - 使用模型: {}", config.model);
    println!("  - 截图目录: {:?}", config.get_screenshot_dir());
    println!("  - 日志目录: {:?}", config.get_logs_dir());
    println!("  - 状态文件: {:?}", config.get_state_path());
    #[cfg(unix)]
    {
        println!("  - Socket路径: {:?}", config.get_socket_path());
    }
    #[cfg(windows)]
    {
        println!("  - 控制端口: {}", config.get_control_port());
    }
    println!("  - 图片处理:");
    println!("    * 目标宽度: {}", if config.image_target_width > 0 { config.image_target_width.to_string() } else { "保持原图".to_string() });
    println!("    * 灰度转换: {}", if config.image_grayscale && !config.no_image_grayscale { "启用" } else { "禁用" });
    println!();
    
    // 确保必要的目录存在
    tokio::fs::create_dir_all(&config.get_screenshot_dir()).await?;
    
    // 创建并启动独立服务
    let service = StandaloneService::new(config).await?;
    
    // 启动服务（包括状态恢复和socket服务器）
    service.start().await?;
    
    Ok(())
}

/// 在后台启动独立服务
async fn start_standalone_service_background(config: config::Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 首先检查并请求必要权限
    let _permission_status = permissions::ensure_permissions().await?;
    
    // 确保必要的目录存在
    tokio::fs::create_dir_all(&config.get_screenshot_dir()).await?;
    
    // 创建并启动独立服务
    let service = StandaloneService::new(config).await?;
    
    // 启动服务（包括状态恢复和socket服务器）
    service.start().await?;
    
    Ok(())
}

async fn process_service_status_response(
    response: crate::models::ServiceResponse,
    controller: &ServiceController,
) -> bool {
    if let Some(state) = response.state {
        if matches!(state.status, crate::models::CaptureServiceStatus::Running) {
            println!("✅ 检测到独立截屏服务已运行");
            return true;
        }

        println!("ℹ️ 独立截屏服务当前为停止状态，正在自动启动...");
    } else {
        println!("⚠️ 未获取到独立截屏服务状态，尝试自动启动...");
    }

    start_capture_service(controller).await
}

async fn start_capture_service(controller: &ServiceController) -> bool {
    match controller
        .send_command(crate::models::ServiceCommand::Start)
        .await
    {
        Ok(response) => {
            if response.success {
                println!("✅ 已自动启动截屏服务");
                true
            } else {
                eprintln!("⚠️ 自动启动截屏服务失败: {}", response.message);
                false
            }
        }
        Err(e) => {
            eprintln!("⚠️ 无法自动启动截屏服务: {}", e);
            false
        }
    }
}