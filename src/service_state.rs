use crate::models::{CaptureServiceState, CaptureServiceStatus};
use crate::config::Config;
use chrono::Local;
use std::path::Path;
use std::error::Error;
use tokio::sync::{RwLock};
use std::sync::Arc;
use serde_json;

/// 服务状态管理器
pub struct ServiceStateManager {
    state: Arc<RwLock<CaptureServiceState>>,
    state_file_path: std::path::PathBuf,
}

impl ServiceStateManager {
    /// 创建新的状态管理器
    pub async fn new(config: &Config) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let state_file_path = config.get_state_path();
        
        // 确保状态文件目录存在
        if let Some(parent) = state_file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let state = Self::load_state(&state_file_path, config).await?;
        
        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            state_file_path,
        })
    }
    
    /// 从文件加载状态
    async fn load_state(
        state_file_path: &Path, 
        config: &Config
    ) -> Result<CaptureServiceState, Box<dyn Error + Send + Sync>> {
        if state_file_path.exists() {
            match tokio::fs::read_to_string(state_file_path).await {
                Ok(content) => {
                    match serde_json::from_str::<CaptureServiceState>(&content) {
                        Ok(mut state) => {
                            // 检查配置是否有变更
                            let current_hash = config.get_config_hash();
                            if state.config_hash != current_hash {
                                println!("检测到配置变更，重置服务状态");
                                state.config_hash = current_hash;
                                // 如果配置变更，停止服务
                                if matches!(state.status, CaptureServiceStatus::Running) {
                                    state.status = CaptureServiceStatus::Stopped;
                                    state.last_stop_time = Some(Local::now());
                                }
                            }
                            return Ok(state);
                        }
                        Err(e) => {
                            eprintln!("解析状态文件失败: {}, 使用默认状态", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("读取状态文件失败: {}, 使用默认状态", e);
                }
            }
        }
        
        // 返回默认状态
        let mut default_state = CaptureServiceState::default();
        default_state.config_hash = config.get_config_hash();
        Ok(default_state)
    }
    
    /// 保存状态到文件
    pub async fn save_state(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let state = self.state.read().await;
        let content = serde_json::to_string_pretty(&*state)?;
        tokio::fs::write(&self.state_file_path, content).await?;
        Ok(())
    }
    
    /// 获取当前状态（只读）
    pub async fn get_state(&self) -> CaptureServiceState {
        self.state.read().await.clone()
    }
    
    /// 启动服务
    pub async fn start_service(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let mut state = self.state.write().await;
        match state.status {
            CaptureServiceStatus::Running => {
                return Ok(false); // 已经在运行
            }
            _ => {
                state.status = CaptureServiceStatus::Running;
                state.last_start_time = Some(Local::now());
                drop(state);
                self.save_state().await?;
                Ok(true)
            }
        }
    }
    
    /// 停止服务
    pub async fn stop_service(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let mut state = self.state.write().await;
        match state.status {
            CaptureServiceStatus::Stopped => {
                return Ok(false); // 已经停止
            }
            _ => {
                state.status = CaptureServiceStatus::Stopped;
                state.last_stop_time = Some(Local::now());
                drop(state);
                self.save_state().await?;
                Ok(true)
            }
        }
    }
    

    
    /// 更新截屏计数
    pub async fn increment_capture_count(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut state = self.state.write().await;
        state.total_captures += 1;
        state.last_capture_time = Some(Local::now());
        drop(state);
        self.save_state().await?;
        Ok(())
    }
    
    /// 检查服务是否应该运行
    pub async fn should_capture(&self) -> bool {
        let state = self.state.read().await;
        matches!(state.status, CaptureServiceStatus::Running)
    }
}
