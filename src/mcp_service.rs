use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::Parameters,
    handler::server::router::tool::ToolRouter,
    model::*,
    tool, tool_handler, tool_router, schemars,
};
use std::future::Future;
use serde::Deserialize;

use chrono::{DateTime, Local, NaiveDateTime};
use std::sync::Arc;
use crate::logger;
use crate::models::{ActivityLog, ServiceCommand, CaptureServiceStatus};
use crate::standalone_service::ServiceController;
use crate::config::Config;

#[derive(Clone)]
pub struct OpenRecallService {
    config: Config,
    service_controller: Arc<ServiceController>,
    tool_router: ToolRouter<OpenRecallService>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MonitorArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadLogsArgs {
    #[serde(skip_serializing_if = "Option::is_none")] pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub detailed: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClipboardListArgs {
    #[serde(skip_serializing_if = "Option::is_none")] pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClipboardSaveArgs {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub target_dir: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClipboardAutoSaveArgs {
    pub enabled: bool,
}

#[tool_router]
impl OpenRecallService {
    pub fn new(config: Config) -> Self {
        let service_controller = Arc::new(ServiceController::new(&config));
        Self {
            config,
            service_controller,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "监控控制工具 - action参数: start(开始), stop(停止), status(查询状态)")]
    async fn monitor(&self, Parameters(args): Parameters<MonitorArgs>) -> Result<CallToolResult, McpError> {
        let action = args.action.as_deref().unwrap_or("status");
        
        let command = match action {
            "start" => ServiceCommand::Start,
            "stop" => ServiceCommand::Stop,
            "status" => ServiceCommand::Status,
            _ => return Ok(CallToolResult::success(vec![Content::text("invalid action, use: start, stop, status")])),
        };
        
        match self.service_controller.send_command(command).await {
            Ok(response) => {
                let mut message = response.message;
                
                if let Some(state) = response.state {
                    let status_str = match state.status {
                        CaptureServiceStatus::Running => "running",
                        CaptureServiceStatus::Stopped => "stopped",
                    };
                    
                    message = format!("{}\n状态: {}\n总截屏数: {}", 
                        message, status_str, state.total_captures);
                    
                    if let Some(last_start) = state.last_start_time {
                        message = format!("{}\n最后启动: {}", message, last_start.format("%Y-%m-%d %H:%M:%S"));
                    }
                    
                    if let Some(last_capture) = state.last_capture_time {
                        message = format!("{}\n最后截屏: {}", message, last_capture.format("%Y-%m-%d %H:%M:%S"));
                    }
                }

                if let Some(clipboard) = response.clipboard_status {
                    message = format!(
                        "{}\n剪贴板监听: {}\n剪贴板自动保存: {}\n剪贴板记录数: {}",
                        message,
                        if clipboard.enabled { "enabled" } else { "disabled" },
                        if clipboard.auto_save { "enabled" } else { "disabled" },
                        clipboard.total_items
                    );
                }
                
                Ok(CallToolResult::success(vec![Content::text(message)]))
            }
            Err(e) => {
                let error_msg = if e.to_string().contains("No such file or directory") || 
                                   e.to_string().contains("Connection refused") {
                    "截屏服务未运行，请先启动独立服务模式"
                } else {
                    &format!("服务通信错误: {}", e)
                };
                Ok(CallToolResult::success(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(description = "读取活动日志（时间范围、数量、详情，默认不显示详情）")]
    async fn read_logs(&self, Parameters(args): Parameters<ReadLogsArgs>) -> Result<CallToolResult, McpError> {
        let limit = args.limit.unwrap_or(50).max(0) as usize;
        let detailed = args.detailed.unwrap_or(false);

        let logs = match logger::load_recent_daily_logs(&self.config, 30) {
            Ok(v) => v,
            Err(e) => return Ok(CallToolResult::success(vec![Content::text(format!("read logs error: {}", e))]))
        };

        let filtered: Vec<&ActivityLog> = logs.iter().filter(|log| {
            if let Some(ref s) = args.start_time { if let Ok(st) = parse_datetime(s) { if log.timestamp < st { return false; } } }
            if let Some(ref e) = args.end_time { if let Ok(et) = parse_datetime(e) { if log.timestamp > et { return false; } } }
            true
        }).collect();

        let result_logs: Vec<&ActivityLog> = filtered.into_iter().rev().take(limit).collect();
        let mut out = String::new();
        for l in result_logs.into_iter().rev() {
            let line = if detailed {
                let ctx = l.context.as_ref().and_then(|c| serde_json::to_value(c).ok()).unwrap_or(serde_json::Value::Null);
                format!("{} | {} | ctx={}\n", l.timestamp.format("%Y-%m-%d %H:%M:%S"), l.description, ctx)
            } else {
                format!("{} | {}\n", l.timestamp.format("%Y-%m-%d %H:%M:%S"), l.description)
            };
            out.push_str(&line);
        }
        Ok(CallToolResult::success(vec![Content::text(out)]))
    }

    #[tool(description = "查询剪贴板监听状态")]
    async fn clipboard_status(&self) -> Result<CallToolResult, McpError> {
        match self
            .service_controller
            .send_command(ServiceCommand::ClipboardStatus)
            .await
        {
            Ok(response) => {
                if let Some(status) = response.clipboard_status {
                    let message = format!(
                        "enabled: {}\nauto_save: {}\ntotal_items: {}\nlast_capture: {}",
                        status.enabled,
                        status.auto_save,
                        status.total_items,
                        status
                            .last_capture_time
                            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "-".to_string())
                    );
                    Ok(CallToolResult::success(vec![Content::text(message)]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(
                        "未获取到剪贴板状态".to_string(),
                    )]))
                }
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "服务通信错误: {}",
                e
            ))])),
        }
    }

    #[tool(description = "列出最近剪贴板记录，limit 默认 20")]
    async fn clipboard_list(
        &self,
        Parameters(args): Parameters<ClipboardListArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .service_controller
            .send_command(ServiceCommand::ClipboardList { limit: args.limit })
            .await
        {
            Ok(response) => Ok(CallToolResult::success(vec![Content::text(response.message)])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "服务通信错误: {}",
                e
            ))])),
        }
    }

    #[tool(description = "将指定 id 的剪贴板内容保存为 Markdown")]
    async fn clipboard_save(
        &self,
        Parameters(args): Parameters<ClipboardSaveArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .service_controller
            .send_command(ServiceCommand::ClipboardSave {
                id: args.id,
                target_dir: args.target_dir,
            })
            .await
        {
            Ok(response) => Ok(CallToolResult::success(vec![Content::text(response.message)])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "服务通信错误: {}",
                e
            ))])),
        }
    }

    #[tool(description = "开启/关闭剪贴板自动保存")]
    async fn clipboard_auto_save(
        &self,
        Parameters(args): Parameters<ClipboardAutoSaveArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .service_controller
            .send_command(ServiceCommand::ClipboardAutoSave {
                enabled: args.enabled,
            })
            .await
        {
            Ok(response) => Ok(CallToolResult::success(vec![Content::text(response.message)])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "服务通信错误: {}",
                e
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for OpenRecallService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("OpenRecall MCP server: tools=monitor, read_logs, clipboard_status, clipboard_list, clipboard_save, clipboard_auto_save".to_string()),
        }
    }
}

fn parse_datetime(datetime_str: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    let naive = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")?;
    Ok(naive.and_local_timezone(Local).single().unwrap_or_else(|| naive.and_utc().with_timezone(&Local)))
}