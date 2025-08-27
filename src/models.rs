use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityLog {
    pub timestamp: DateTime<Local>,
    pub description: String,
}