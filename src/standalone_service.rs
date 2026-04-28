use crate::config::Config;
use crate::openclaw;
use crate::clipboard::{self, ClipboardManager};
use crate::service_state::ServiceStateManager;
use crate::capture;
use crate::models::{CaptureServiceStatus, ServiceCommand, ServiceResponse};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
#[cfg(windows)]
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json;

/// 独立截屏服务
pub struct StandaloneService {
    config: Config,
    state_manager: Arc<ServiceStateManager>,
    shutdown_tx: broadcast::Sender<()>,
    capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    clipboard_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    clipboard_manager: Arc<Mutex<ClipboardManager>>,
}

impl StandaloneService {
    /// 创建新的独立服务
    pub async fn new(config: Config) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let state_manager = Arc::new(ServiceStateManager::new(&config).await?);
        let clipboard_manager = Arc::new(Mutex::new(ClipboardManager::new(config.clone()).await?));
        let (shutdown_tx, _) = broadcast::channel(16);
        
        Ok(Self {
            config,
            state_manager,
            shutdown_tx,
            capture_handle: Arc::new(Mutex::new(None)),
            clipboard_handle: Arc::new(Mutex::new(None)),
            clipboard_manager,
        })
    }
    
    /// 启动服务（包括恢复之前的状态）
    pub async fn start(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("🚀 启动独立截屏服务...");
        
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

        if self.config.start_capture_on_launch
            && matches!(current_state.status, CaptureServiceStatus::Stopped)
        {
            println!("⚡ 检测到 --start-capture-on-launch，强制开启截屏服务...");
            match self.state_manager.start_service().await {
                Ok(true) => {
                    self.start_capture_loop().await?;
                    println!("✅ 启动时已强制开启截屏服务");
                }
                Ok(false) => {
                    println!("ℹ️ 截屏服务已在运行状态");
                }
                Err(e) => {
                    eprintln!("⚠️ 启动时强制开启截屏失败: {}", e);
                }
            }
        }

        if self.config.clipboard_enabled {
            self.start_clipboard_loop().await?;
        }
        
        // 启动控制socket服务器
        #[cfg(unix)]
        {
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
            let clipboard_handle = self.clipboard_handle.clone();
            let clipboard_manager = self.clipboard_manager.clone();
            
            tokio::spawn(async move {
                Self::handle_unix_socket_connections(
                    listener, 
                    state_manager, 
                    config, 
                    shutdown_tx,
                    capture_handle,
                    clipboard_handle,
                    clipboard_manager,
                ).await;
            });
        }
        
        #[cfg(windows)]
        {
            let port = self.config.get_control_port();
            let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
            println!("🔌 控制TCP socket启动: 127.0.0.1:{}", port);
            
            let state_manager = self.state_manager.clone();
            let config = self.config.clone();
            let shutdown_tx = self.shutdown_tx.clone();
            let capture_handle = self.capture_handle.clone();
            let clipboard_handle = self.clipboard_handle.clone();
            let clipboard_manager = self.clipboard_manager.clone();
            
            tokio::spawn(async move {
                Self::handle_tcp_socket_connections(
                    listener, 
                    state_manager, 
                    config, 
                    shutdown_tx,
                    capture_handle,
                    clipboard_handle,
                    clipboard_manager,
                ).await;
            });
        }

        // 若配置了 OpenClaw，启动定期上报任务
        if self.config.openclaw_enabled() {
            let config = self.config.clone();
            tokio::spawn(async move {
                openclaw::run_reporter_loop(config).await;
            });
        }
        
        println!("✅ 独立截屏服务启动完成！");
        
        // 等待关闭信号
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        shutdown_rx.recv().await.ok();
        
        // 清理socket文件（仅Unix系统）
        #[cfg(unix)]
        {
            let socket_path = self.config.get_socket_path();
            let _ = std::fs::remove_file(&socket_path);
        }
        
        Ok(())
    }
    
    /// 处理Unix socket连接
    #[cfg(unix)]
    async fn handle_unix_socket_connections(
        listener: UnixListener,
        state_manager: Arc<ServiceStateManager>,
        config: Config,
        _shutdown_tx: broadcast::Sender<()>,
        capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_manager: Arc<Mutex<ClipboardManager>>,
    ) {
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let state_manager = state_manager.clone();
                    let config = config.clone();
                    let capture_handle = capture_handle.clone();
                    let clipboard_handle = clipboard_handle.clone();
                    let clipboard_manager = clipboard_manager.clone();
                    
                    tokio::spawn(async move {
                        Self::handle_unix_stream(stream, state_manager, config, capture_handle, clipboard_handle, clipboard_manager).await;
                    });
                }
                Err(e) => {
                    eprintln!("接受Unix socket连接失败: {}", e);
                    break;
                }
            }
        }
    }
    
    /// 处理TCP socket连接
    #[cfg(windows)]
    async fn handle_tcp_socket_connections(
        listener: TcpListener,
        state_manager: Arc<ServiceStateManager>,
        config: Config,
        _shutdown_tx: broadcast::Sender<()>,
        capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_manager: Arc<Mutex<ClipboardManager>>,
    ) {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("接受TCP连接: {}", addr);
                    let state_manager = state_manager.clone();
                    let config = config.clone();
                    let capture_handle = capture_handle.clone();
                    let clipboard_handle = clipboard_handle.clone();
                    let clipboard_manager = clipboard_manager.clone();
                    
                    tokio::spawn(async move {
                        Self::handle_tcp_stream(stream, state_manager, config, capture_handle, clipboard_handle, clipboard_manager).await;
                    });
                }
                Err(e) => {
                    eprintln!("接受TCP socket连接失败: {}", e);
                    break;
                }
            }
        }
    }
    
    /// 处理Unix stream
    #[cfg(unix)]
    async fn handle_unix_stream(
        mut stream: UnixStream,
        state_manager: Arc<ServiceStateManager>,
        config: Config,
        capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_manager: Arc<Mutex<ClipboardManager>>,
    ) {
        let mut buffer = Vec::new();
        let mut temp_buffer = [0; 1024];
        
        loop {
            match stream.read(&mut temp_buffer).await {
                Ok(0) => break, // 连接关闭
                Ok(n) => {
                    buffer.extend_from_slice(&temp_buffer[..n]);
                    
                    // 尝试解析JSON命令
                    if let Ok(command) = serde_json::from_slice::<ServiceCommand>(&buffer) {
                        let response = Self::handle_command(command, &state_manager, &config, &capture_handle, &clipboard_handle, &clipboard_manager).await;
                        
                        if let Ok(response_json) = serde_json::to_string(&response) {
                            if let Err(e) = stream.write_all(response_json.as_bytes()).await {
                                eprintln!("写入Unix socket响应失败: {}", e);
                                break;
                            }
                        }
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("读取Unix socket失败: {}", e);
                    break;
                }
            }
        }
    }
    
    /// 处理TCP stream
    #[cfg(windows)]
    async fn handle_tcp_stream(
        mut stream: TcpStream,
        state_manager: Arc<ServiceStateManager>,
        config: Config,
        capture_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_manager: Arc<Mutex<ClipboardManager>>,
    ) {
        let mut buffer = Vec::new();
        let mut temp_buffer = [0; 1024];
        
        loop {
            match stream.read(&mut temp_buffer).await {
                Ok(0) => break, // 连接关闭
                Ok(n) => {
                    buffer.extend_from_slice(&temp_buffer[..n]);
                    
                    // 尝试解析JSON命令
                    if let Ok(command) = serde_json::from_slice::<ServiceCommand>(&buffer) {
                        let response = Self::handle_command(command, &state_manager, &config, &capture_handle, &clipboard_handle, &clipboard_manager).await;
                        
                        if let Ok(response_json) = serde_json::to_string(&response) {
                            if let Err(e) = stream.write_all(response_json.as_bytes()).await {
                                eprintln!("写入TCP socket响应失败: {}", e);
                                break;
                            }
                        }
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("读取TCP socket失败: {}", e);
                    break;
                }
            }
        }
    }
    
    /// 处理服务命令
    async fn handle_command(
        command: ServiceCommand,
        state_manager: &Arc<ServiceStateManager>,
        config: &Config,
        capture_handle: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_handle: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_manager: &Arc<Mutex<ClipboardManager>>,
    ) -> ServiceResponse {
        match command {
            ServiceCommand::Start => {
                match state_manager.start_service().await {
                    Ok(true) => {
                        // 启动截屏循环
                        if let Err(e) = Self::start_capture_task(state_manager, config, capture_handle).await {
                            let _ = state_manager.stop_service().await;
                            ServiceResponse {
                                success: false,
                                message: format!("启动截屏失败: {}", e),
                                state: Some(state_manager.get_state().await),
                                clipboard_status: Some(clipboard_manager.lock().await.status()),
                            }
                        } else {
                            if config.clipboard_enabled {
                                if let Err(e) = Self::start_clipboard_task(config, clipboard_handle, clipboard_manager).await {
                                    eprintln!("启动剪贴板监听失败: {}", e);
                                }
                            }
                            ServiceResponse {
                                success: true,
                                message: "服务已启动".to_string(),
                                state: Some(state_manager.get_state().await),
                                clipboard_status: Some(clipboard_manager.lock().await.status()),
                            }
                        }
                    }
                    Ok(false) => ServiceResponse {
                        success: true,
                        message: "服务已在运行".to_string(),
                        state: Some(state_manager.get_state().await),
                        clipboard_status: Some(clipboard_manager.lock().await.status()),
                    },
                    Err(e) => ServiceResponse {
                        success: false,
                        message: format!("启动失败: {}", e),
                        state: Some(state_manager.get_state().await),
                        clipboard_status: Some(clipboard_manager.lock().await.status()),
                    }
                }
            }
            ServiceCommand::Stop => {
                match state_manager.stop_service().await {
                    Ok(_) => {
                        // 停止截屏循环
                        Self::stop_capture_task(capture_handle).await;
                        Self::stop_clipboard_task(clipboard_handle).await;
                        ServiceResponse {
                            success: true,
                            message: "服务已停止".to_string(),
                            state: Some(state_manager.get_state().await),
                            clipboard_status: Some(clipboard_manager.lock().await.status()),
                        }
                    }
                    Err(e) => ServiceResponse {
                        success: false,
                        message: format!("停止失败: {}", e),
                        state: Some(state_manager.get_state().await),
                        clipboard_status: Some(clipboard_manager.lock().await.status()),
                    }
                }
            }

            ServiceCommand::Status => ServiceResponse {
                success: true,
                message: "状态查询成功".to_string(),
                state: Some(state_manager.get_state().await),
                clipboard_status: Some(clipboard_manager.lock().await.status()),
            },
            ServiceCommand::ClipboardStatus => ServiceResponse {
                success: true,
                message: "剪贴板状态查询成功".to_string(),
                state: Some(state_manager.get_state().await),
                clipboard_status: Some(clipboard_manager.lock().await.status()),
            },
            ServiceCommand::ClipboardList { limit } => {
                let guard = clipboard_manager.lock().await;
                let items = guard.list_recent(limit.unwrap_or(20));
                let mut message = format!("最近 {} 条剪贴板记录：", items.len());
                for item in items {
                    let preview = item.content.lines().next().unwrap_or("").chars().take(60).collect::<String>();
                    message.push_str(&format!(
                        "\n- {} | {} | seen={} | {}",
                        item.id,
                        item.last_seen.format("%Y-%m-%d %H:%M:%S"),
                        item.seen_count,
                        preview
                    ));
                }
                ServiceResponse {
                    success: true,
                    message,
                    state: Some(state_manager.get_state().await),
                    clipboard_status: Some(guard.status()),
                }
            }
            ServiceCommand::ClipboardSave { id, target_dir } => {
                let mut guard = clipboard_manager.lock().await;
                let path = guard
                    .save_item_to_markdown(&id, target_dir.map(std::path::PathBuf::from))
                    .await;
                match path {
                    Ok(Some(path)) => ServiceResponse {
                        success: true,
                        message: format!("已保存到 {}", path.to_string_lossy()),
                        state: Some(state_manager.get_state().await),
                        clipboard_status: Some(guard.status()),
                    },
                    Ok(None) => ServiceResponse {
                        success: false,
                        message: "未找到对应剪贴板记录".to_string(),
                        state: Some(state_manager.get_state().await),
                        clipboard_status: Some(guard.status()),
                    },
                    Err(e) => ServiceResponse {
                        success: false,
                        message: format!("保存失败: {}", e),
                        state: Some(state_manager.get_state().await),
                        clipboard_status: Some(guard.status()),
                    },
                }
            }
            ServiceCommand::ClipboardAutoSave { enabled } => {
                let mut guard = clipboard_manager.lock().await;
                guard.set_auto_save(enabled);
                ServiceResponse {
                    success: true,
                    message: format!("自动保存已{}", if enabled { "开启" } else { "关闭" }),
                    state: Some(state_manager.get_state().await),
                    clipboard_status: Some(guard.status()),
                }
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

    async fn start_clipboard_task(
        config: &Config,
        clipboard_handle: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        clipboard_manager: &Arc<Mutex<ClipboardManager>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut handle_guard = clipboard_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }

        let config_clone = config.clone();
        let manager_clone = clipboard_manager.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = clipboard::run_clipboard_loop(config_clone, manager_clone).await {
                eprintln!("剪贴板监听循环出错: {}", e);
            }
        });
        *handle_guard = Some(handle);
        Ok(())
    }

    async fn stop_clipboard_task(
        clipboard_handle: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ) {
        let mut handle_guard = clipboard_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
    }
    
    /// 启动截屏循环（内部使用）
    async fn start_capture_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Self::start_capture_task(&self.state_manager, &self.config, &self.capture_handle).await
    }

    async fn start_clipboard_loop(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Self::start_clipboard_task(&self.config, &self.clipboard_handle, &self.clipboard_manager).await
    }
    

}

/// 服务控制客户端
pub struct ServiceController {
    #[cfg(unix)]
    socket_path: std::path::PathBuf,
    #[cfg(windows)]
    port: u16,
}

impl ServiceController {
    pub fn new(config: &Config) -> Self {
        #[cfg(unix)]
        {
            Self {
                socket_path: config.get_socket_path(),
            }
        }
        #[cfg(windows)]
        {
            Self {
                port: config.get_control_port(),
            }
        }
    }
    
    /// 发送命令到服务
    pub async fn send_command(&self, command: ServiceCommand) -> Result<ServiceResponse, Box<dyn Error + Send + Sync>> {
        use tokio::time::{timeout, Duration};
        
        // 设置30秒的连接和通信超时
        let timeout_duration = Duration::from_secs(30);
        
        let result = timeout(timeout_duration, async {
            #[cfg(unix)]
            {
                let mut stream = UnixStream::connect(&self.socket_path).await?;
                
                let command_str = serde_json::to_string(&command)?;
                stream.write_all(command_str.as_bytes()).await?;
                
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).await?;
                let response_str = String::from_utf8_lossy(&buffer[..n]);
                
                let response: ServiceResponse = serde_json::from_str(&response_str)?;
                Ok(response)
            }
            #[cfg(windows)]
            {
                let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.port)).await?;
                
                let command_str = serde_json::to_string(&command)?;
                stream.write_all(command_str.as_bytes()).await?;
                
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).await?;
                let response_str = String::from_utf8_lossy(&buffer[..n]);
                
                let response: ServiceResponse = serde_json::from_str(&response_str)?;
                Ok(response)
            }
        }).await;
        
        match result {
            Ok(response) => response,
            Err(_) => Err("操作超时：TCP连接或通信超过30秒".into()),
        }
    }
}
