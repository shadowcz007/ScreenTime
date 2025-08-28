use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityLog {
    pub timestamp: DateTime<Local>,
    pub description: String,
    // 新增：可选上下文与截图路径，向后兼容旧日志
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<crate::context::SystemContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_path: Option<String>,
}