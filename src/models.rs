use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityLog {
    pub timestamp: DateTime<Local>,
    pub description: String,
    pub context: Option<SystemContext>,
    pub screenshot_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemContext {
    pub active_app: Option<String>,
    pub window_title: Option<String>,
    pub system_info: Option<SystemInfo>,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub platform: Option<String>,
}

// 新增：截屏服务状态
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CaptureServiceStatus {
    Running,
    Stopped,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaptureServiceState {
    pub status: CaptureServiceStatus,
    pub last_start_time: Option<DateTime<Local>>,
    pub last_stop_time: Option<DateTime<Local>>,
    pub total_captures: u64,
    pub last_capture_time: Option<DateTime<Local>>,
    pub config_hash: String, // 用于检测配置变更
}

impl Default for CaptureServiceState {
    fn default() -> Self {
        Self {
            status: CaptureServiceStatus::Stopped,
            last_start_time: None,
            last_stop_time: None,
            total_captures: 0,
            last_capture_time: None,
            config_hash: String::new(),
        }
    }
}

// 新增：服务控制命令
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServiceCommand {
    Start,
    Stop,
    Status,
}

// 新增：服务响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceResponse {
    pub success: bool,
    pub message: String,
    pub state: Option<CaptureServiceState>,
}