use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::Parameters,
    handler::server::router::tool::ToolRouter,
    model::*,
    tool, tool_handler, tool_router, schemars,
};
use std::future::Future;
use serde::Deserialize;
use tokio::sync::{broadcast, Mutex};
use chrono::{DateTime, Local, NaiveDateTime};
use std::sync::Arc;
use crate::logger;
use crate::models::ActivityLog;
use crate::capture;
use crate::config::Config;

#[derive(Debug, Clone)]
pub enum CaptureStatus {
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct CaptureState {
    pub status: CaptureStatus,
    pub config: Config,
    pub cancel_sender: Option<broadcast::Sender<()>>,
}

impl CaptureState {
    pub fn new(config: Config) -> Self {
        Self { status: CaptureStatus::Stopped, config, cancel_sender: None }
    }
}

#[derive(Clone)]
pub struct ScreenTimeService {
    state: Arc<Mutex<CaptureState>>,
    tool_router: ToolRouter<ScreenTimeService>,
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

#[tool_router]
impl ScreenTimeService {
    pub fn new(config: Config) -> Self {
        Self {
            state: Arc::new(Mutex::new(CaptureState::new(config))),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "监控控制工具 - action参数: start(开始), stop(停止), status(查询状态)")]
    async fn monitor(&self, Parameters(args): Parameters<MonitorArgs>) -> Result<CallToolResult, McpError> {
        let action = args.action.as_deref().unwrap_or("status");
        
        match action {
            "start" => {
                let mut state = self.state.lock().await;
                match state.status {
                    CaptureStatus::Running => Ok(CallToolResult::success(vec![Content::text("already running")])),
                    _ => {
                        let (tx, _) = broadcast::channel(1);
                        let mut rx = tx.subscribe();
                        let config = state.config.clone();
                        state.cancel_sender = Some(tx.clone());
                        state.status = CaptureStatus::Running;
                        tokio::spawn(async move {
                            tokio::select! {
                                r = capture::run_capture_loop(config) => {
                                    if let Err(e) = r { eprintln!("monitor error: {}", e); }
                                }
                                _ = rx.recv() => {}
                            }
                        });
                        Ok(CallToolResult::success(vec![Content::text("started")]))
                    }
                }
            },
            "stop" => {
                let mut state = self.state.lock().await;
                if let Some(tx) = &state.cancel_sender { let _ = tx.send(()); }
                state.cancel_sender = None;
                state.status = CaptureStatus::Stopped;
                Ok(CallToolResult::success(vec![Content::text("stopped")]))
            },
            "status" => {
                let state = self.state.lock().await;
                let s = match state.status { 
                    CaptureStatus::Running => "running", 
                    CaptureStatus::Paused => "paused", 
                    CaptureStatus::Stopped => "stopped" 
                };
                Ok(CallToolResult::success(vec![Content::text(s)]))
            },
            _ => Ok(CallToolResult::success(vec![Content::text("invalid action, use: start, stop, status")])),
        }
    }

    #[tool(description = "读取活动日志（时间范围、数量、详情，默认不显示详情）")]
    async fn read_logs(&self, Parameters(args): Parameters<ReadLogsArgs>) -> Result<CallToolResult, McpError> {
        let limit = args.limit.unwrap_or(50).max(0) as usize;
        let detailed = args.detailed.unwrap_or(false);

        let state = self.state.lock().await;
        let log_path = match state.config.log_path.to_str() { Some(p) => p.to_string(), None => return Ok(CallToolResult::success(vec![Content::text("invalid log path")])) };
        drop(state);

        let logs = match logger::load_activity_logs(&log_path) { Ok(v) => v, Err(e) => return Ok(CallToolResult::success(vec![Content::text(format!("read logs error: {}", e))])) };

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
}

#[tool_handler]
impl ServerHandler for ScreenTimeService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("ScreenTime MCP server: tools=monitor, read_logs".to_string()),
        }
    }
}

fn parse_datetime(datetime_str: &str) -> Result<DateTime<Local>, chrono::ParseError> {
    let naive = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S")?;
    Ok(naive.and_local_timezone(Local).single().unwrap_or_else(|| naive.and_utc().with_timezone(&Local)))
}