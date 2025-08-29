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

use std::error::Error;

use mcp_service::ScreenTimeService;
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
    
    // 首先检查并请求必要权限
    println!("第一步：权限检查");
    let _permission_status = permissions::ensure_permissions().await?;
    println!("✅ 权限检查通过！\n");
    
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

async fn run_mcp_server(config: config::Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    const BIND_ADDRESS: &str = "127.0.0.1:8000";

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".to_string().into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("🌐 启动 MCP SSE 服务器，地址: {}", BIND_ADDRESS);

    // 确保必要的目录存在
    tokio::fs::create_dir_all(&config.screenshot_dir).await?;

    let server_config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
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