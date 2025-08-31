use crate::config::Config;
use crate::service_state::ServiceStateManager;
use crate::capture;
use crate::models::{CaptureServiceStatus, ServiceCommand, ServiceResponse};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json;

/// 独立截屏服务
pub struct StandaloneService {
    config: Config,
    state_manager: Arc<ServiceStateManager>,
    shutdown_tx: broadcast::Sender<()>,
    capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl StandaloneService {
    /// 创建新的独立服务
    pub async fn new(config: Config) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let state_manager = Arc::new(ServiceStateManager::new(&config).await?);
        let (shutdown_tx, _) = broadcast::channel(16);
        
        Ok(Self {
            config,
            state_manager,
            shutdown_tx,
            capture_handle: Arc::new(Mutex::new(None)),
        })
    }
    
    /// 启动服务（包括恢复之前的状态）
    pub async fn start(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("🚀 启动独立截屏服务...");
        
        // 初始化FastVLM服务（如果配置了本地模型）
        if let Err(e) = capture::initialize_fastvlm_if_needed(&self.config).await {
            eprintln!("⚠️ FastVLM初始化失败: {}", e);
            eprintln!("   将继续使用API方式进行分析");
        }
        
        // 检查之前的状态并自动恢复
        let current_state = self.state_manager.get_state().await;
        match current_state.status {
            CaptureServiceStatus::Running => {
                println!("🔄 检测到之前服务正在运行，自动恢复截屏...");
                self.start_capture_loop().await?;
            }
            CaptureServiceStatus::Stopped => {
                println!("⏹️ 服务处于停止状态");
            }
        }
        
        // 启动控制socket服务器
        let socket_path = self.config.get_socket_path();
        if socket_path.exists() {
            let _ = std::fs::remove_file(&socket_path);
        }
        
        // 确保socket目录存在
        if let Some(parent) = socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let listener = UnixListener::bind(&socket_path)?;
        println!("🔌 控制socket启动: {:?}", socket_path);
        
        let state_manager = self.state_manager.clone();
        let config = self.config.clone();
        let shutdown_tx = self.shutdown_tx.clone();
        let capture_handle = self.capture_handle.clone();
        
        tokio::spawn(async move {
            Self::handle_socket_connections(
                listener, 
                state_manager, 
                config, 
                shutdown_tx,
                capture_handle
            ).await;
        });
        
        println!("✅ 独立截屏服务启动完成！");
        
        // 等待关闭信号
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        shutdown_rx.recv().await.ok();
        
        // 清理socket文件
        let _ = std::fs::remove_file(&socket_path);
        
        Ok(())
    }
    
    /// 处理socket连接
    async fn handle_socket_connections(
        listener: UnixListener,
        state_manager: Arc<ServiceStateManager>,
        config: Config,
        _shutdown_tx: broadcast::Sender<()>,
        capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>
    ) {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let state_manager = state_manager.clone();
                    let config = config.clone();
                    let shutdown_tx = _shutdown_tx.clone();
                    let capture_handle = capture_handle.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client_connection(
                            stream, 
                            state_manager, 
                            config, 
                            shutdown_tx,
                            capture_handle
                        ).await {
                            eprintln!("处理客户端连接时出错: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("接受socket连接时出错: {}", e);
                    break;
                }
            }
        }
    }
    
    /// 处理单个客户端连接
    async fn handle_client_connection(
        mut stream: UnixStream,
        state_manager: Arc<ServiceStateManager>,
        config: Config,
        shutdown_tx: broadcast::Sender<()>,
        capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let command_str = String::from_utf8_lossy(&buffer[..n]);
        
        let response = match serde_json::from_str::<ServiceCommand>(&command_str) {
            Ok(command) => {
                Self::handle_command(
                    command, 
                    state_manager, 
                    config, 
                    shutdown_tx,
                    capture_handle
                ).await
            }
            Err(e) => ServiceResponse {
                success: false,
                message: format!("无效命令: {}", e),
                state: None,
            }
        };
        
        let response_str = serde_json::to_string(&response)?;
        stream.write_all(response_str.as_bytes()).await?;
        
        Ok(())
    }
    
    /// 处理服务命令
    async fn handle_command(
        command: ServiceCommand,
        state_manager: Arc<ServiceStateManager>,
        config: Config,
        _shutdown_tx: broadcast::Sender<()>,
        capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>
    ) -> ServiceResponse {
        match command {
            ServiceCommand::Start => {
                match state_manager.start_service().await {
                    Ok(true) => {
                        // 启动截屏循环
                        if let Err(e) = Self::start_capture_task(&state_manager, &config, &capture_handle).await {
                            let _ = state_manager.stop_service().await;
                            ServiceResponse {
                                success: false,
                                message: format!("启动截屏失败: {}", e),
                                state: Some(state_manager.get_state().await),
                            }
                        } else {
                            ServiceResponse {
                                success: true,
                                message: "服务已启动".to_string(),
                                state: Some(state_manager.get_state().await),
                            }
                        }
                    }
                    Ok(false) => ServiceResponse {
                        success: true,
                        message: "服务已在运行".to_string(),
                        state: Some(state_manager.get_state().await),
                    },
                    Err(e) => ServiceResponse {
                        success: false,
                        message: format!("启动失败: {}", e),
                        state: Some(state_manager.get_state().await),
                    }
                }
            }
            ServiceCommand::Stop => {
                match state_manager.stop_service().await {
                    Ok(_) => {
                        // 停止截屏循环
                        Self::stop_capture_task(&capture_handle).await;
                        ServiceResponse {
                            success: true,
                            message: "服务已停止".to_string(),
                            state: Some(state_manager.get_state().await),
                        }
                    }
                    Err(e) => ServiceResponse {
                        success: false,
                        message: format!("停止失败: {}", e),
                        state: Some(state_manager.get_state().await),
                    }
                }
            }

            ServiceCommand::Status => ServiceResponse {
                success: true,
                message: "状态查询成功".to_string(),
                state: Some(state_manager.get_state().await),
            }
        }
    }
    
    /// 启动截屏任务
    async fn start_capture_task(
        state_manager: &Arc<ServiceStateManager>,
        config: &Config,
        capture_handle: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut handle_guard = capture_handle.lock().await;
        
        // 如果已有任务在运行，先停止
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        
        let state_manager_clone = state_manager.clone();
        let config_clone = config.clone();
        
        let handle = tokio::spawn(async move {
            if let Err(e) = capture::run_capture_loop_with_state(config_clone, state_manager_clone).await {
                eprintln!("截屏循环出错: {}", e);
            }
        });
        
        *handle_guard = Some(handle);
        Ok(())
    }
    
    /// 停止截屏任务
    async fn stop_capture_task(capture_handle: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>) {
        let mut handle_guard = capture_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
    }
    
    /// 启动截屏循环（内部使用）
    async fn start_capture_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Self::start_capture_task(&self.state_manager, &self.config, &self.capture_handle).await
    }
    
}

/// 服务控制客户端
pub struct ServiceController {
    socket_path: std::path::PathBuf,
}

impl ServiceController {
    pub fn new(config: &Config) -> Self {
        Self {
            socket_path: config.get_socket_path(),
        }
    }
    
    /// 发送命令到服务
    pub async fn send_command(&self, command: ServiceCommand) -> Result<ServiceResponse, Box<dyn Error + Send + Sync>> {
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        
        let command_str = serde_json::to_string(&command)?;
        stream.write_all(command_str.as_bytes()).await?;
        
        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).await?;
        let response_str = String::from_utf8_lossy(&buffer[..n]);
        
        let response: ServiceResponse = serde_json::from_str(&response_str)?;
        Ok(response)
    }
}
