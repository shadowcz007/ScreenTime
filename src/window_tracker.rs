use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

// 窗口切换事件
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowSwitchEvent {
    pub from_app: Option<String>,
    pub to_app: Option<String>,
    pub from_title: Option<String>, 
    pub to_title: Option<String>,
    pub timestamp: u64, // Unix timestamp in milliseconds
    pub duration_ms: u64, // 上一个窗口的持续时间
}

// 窗口会话信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowSession {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_ms: u64,
}

// 窗口统计信息 - 简化版本
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowSwitchStats {
    pub total_switches: u32,
    pub most_used_apps: Vec<(String, u64)>, // (app_name, total_duration_ms)
    pub current_session_duration_ms: u64,
    pub last_switch_time: Option<u64>,
}

// 增强的窗口信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnhancedWindowInfo {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub bounds: Option<crate::context::WindowBounds>,
    pub timestamp: u64,
    pub process_id: Option<u32>,
    pub is_focus_changed: bool, // 是否是焦点变化
}

// 窗口追踪器
pub struct WindowTracker {
    // 当前窗口信息
    current_window: Arc<RwLock<Option<EnhancedWindowInfo>>>,
    
    // 窗口切换历史 (最近100个事件)
    switch_history: Arc<Mutex<VecDeque<WindowSwitchEvent>>>,
    
    // 窗口会话历史
    session_history: Arc<Mutex<VecDeque<WindowSession>>>,
    
    // 应用使用时间统计
    app_usage_stats: Arc<Mutex<HashMap<String, u64>>>,
    
    // 统计信息
    stats: Arc<Mutex<WindowSwitchStats>>,
    
    // 缓存信息（避免频繁查询相同状态）
    last_query_time: Arc<Mutex<Instant>>,
    cached_info: Arc<Mutex<Option<EnhancedWindowInfo>>>,
    cache_duration: Duration,
}

impl Default for WindowTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowTracker {
    pub fn new() -> Self {
        Self {
            current_window: Arc::new(RwLock::new(None)),
            switch_history: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            session_history: Arc::new(Mutex::new(VecDeque::with_capacity(50))),
            app_usage_stats: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(WindowSwitchStats {
                total_switches: 0,
                most_used_apps: Vec::new(),
                current_session_duration_ms: 0,
                last_switch_time: None,
            })),
            last_query_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10))),
            cached_info: Arc::new(Mutex::new(None)),
            cache_duration: Duration::from_millis(500), // 500ms缓存
        }
    }
    
    /// 获取当前窗口信息（带缓存）
    pub async fn get_current_window_info(&self) -> Option<EnhancedWindowInfo> {
        // 检查缓存
        {
            let last_query = self.last_query_time.lock().unwrap();
            if last_query.elapsed() < self.cache_duration {
                if let Some(cached) = self.cached_info.lock().unwrap().clone() {
                    return Some(cached);
                }
            }
        }
        
        // 获取新的窗口信息
        let new_info = self.fetch_window_info().await;
        
        // 更新缓存
        {
            *self.last_query_time.lock().unwrap() = Instant::now();
            *self.cached_info.lock().unwrap() = new_info.clone();
        }
        
        // 如果窗口发生变化，记录切换事件
        if let Some(ref new_window) = new_info {
            self.handle_window_change(new_window.clone()).await;
        }
        
        new_info
    }
    
    /// 处理窗口变化
    async fn handle_window_change(&self, new_window: EnhancedWindowInfo) {
        let current = self.current_window.read().await;
        let is_different = match &*current {
            Some(old) => {
                old.app_name != new_window.app_name || 
                old.window_title != new_window.window_title
            }
            None => true,
        };
        
        if is_different {
            drop(current);
            
            let mut current_write = self.current_window.write().await;
            let old_window = current_write.clone();
            *current_write = Some(new_window.clone());
            drop(current_write);
            
            // 记录切换事件
            self.record_switch_event(old_window, new_window).await;
        }
    }
    
    /// 记录窗口切换事件
    async fn record_switch_event(&self, old_window: Option<EnhancedWindowInfo>, new_window: EnhancedWindowInfo) {
        let now = get_current_timestamp();
        let duration = if let Some(ref old) = old_window {
            now.saturating_sub(old.timestamp)
        } else {
            0
        };
        
        // 创建切换事件
        let switch_event = WindowSwitchEvent {
            from_app: old_window.as_ref().and_then(|w| w.app_name.clone()),
            to_app: new_window.app_name.clone(),
            from_title: old_window.as_ref().and_then(|w| w.window_title.clone()),
            to_title: new_window.window_title.clone(),
            timestamp: now,
            duration_ms: duration,
        };
        
        // 添加到历史记录
        {
            let mut history = self.switch_history.lock().unwrap();
            history.push_back(switch_event);
            if history.len() > 100 {
                history.pop_front();
            }
        }
        
        // 结束旧会话，开始新会话
        if let Some(old) = old_window {
            self.end_session(old, now).await;
        }
        self.start_session(new_window, now).await;
        
        // 更新统计信息
        self.update_stats().await;
    }
    
    /// 开始新会话
    async fn start_session(&self, window: EnhancedWindowInfo, start_time: u64) {
        let session = WindowSession {
            app_name: window.app_name,
            window_title: window.window_title,
            start_time,
            end_time: None,
            duration_ms: 0,
        };
        
        let mut sessions = self.session_history.lock().unwrap();
        sessions.push_back(session);
        if sessions.len() > 50 {
            sessions.pop_front();
        }
    }
    
    /// 结束会话
    async fn end_session(&self, old_window: EnhancedWindowInfo, end_time: u64) {
        let mut sessions = self.session_history.lock().unwrap();
        if let Some(last_session) = sessions.back_mut() {
            if last_session.app_name == old_window.app_name && 
               last_session.window_title == old_window.window_title {
                last_session.end_time = Some(end_time);
                last_session.duration_ms = end_time.saturating_sub(last_session.start_time);
                
                // 更新应用使用统计
                if let Some(ref app_name) = last_session.app_name {
                    let mut stats = self.app_usage_stats.lock().unwrap();
                    *stats.entry(app_name.clone()).or_insert(0) += last_session.duration_ms;
                }
            }
        }
    }
    
    /// 更新统计信息 - 简化版本
    async fn update_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        let history = self.switch_history.lock().unwrap();
        let sessions = self.session_history.lock().unwrap();
        let app_stats = self.app_usage_stats.lock().unwrap();
        
        stats.total_switches = history.len() as u32;
        
        // 最常用应用
        let mut app_usage: Vec<(String, u64)> = app_stats.iter()
            .map(|(name, duration)| (name.clone(), *duration))
            .collect();
        app_usage.sort_by(|a, b| b.1.cmp(&a.1));
        app_usage.truncate(5);
        stats.most_used_apps = app_usage;
        
        // 当前会话时长
        if let Some(last_session) = sessions.back() {
            if last_session.end_time.is_none() {
                stats.current_session_duration_ms = 
                    get_current_timestamp().saturating_sub(last_session.start_time);
            }
        }
        
        stats.last_switch_time = history.back().map(|event| event.timestamp);
    }
    
    /// 获取统计信息
    pub async fn get_stats(&self) -> WindowSwitchStats {
        self.update_stats().await;
        self.stats.lock().unwrap().clone()
    }
    
    /// 获取切换历史
    pub async fn get_switch_history(&self, limit: Option<usize>) -> Vec<WindowSwitchEvent> {
        let history = self.switch_history.lock().unwrap();
        let limit = limit.unwrap_or(20);
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// 跨平台获取窗口信息
    async fn fetch_window_info(&self) -> Option<EnhancedWindowInfo> {
        #[cfg(target_os = "macos")]
        {
            self.fetch_macos_window_info().await
        }
        
        #[cfg(target_os = "windows")]
        {
            self.fetch_windows_window_info().await
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            None
        }
    }
    
    /// macOS 窗口信息获取（优化版 AppleScript）
    #[cfg(target_os = "macos")]
    async fn fetch_macos_window_info(&self) -> Option<EnhancedWindowInfo> {
        use std::process::Command;
        
        let script = r#"
            tell application "System Events"
                set frontApp to first process whose frontmost is true
                set appName to name of frontApp
                set processId to unix id of frontApp
                try
                    set windowTitle to title of front window of frontApp
                on error
                    set windowTitle to ""
                end try
                try
                    set windowPos to position of front window of frontApp
                    set windowSize to size of front window of frontApp
                    return appName & "|" & windowTitle & "|" & processId & "|" & (item 1 of windowPos as string) & "," & (item 2 of windowPos as string) & "|" & (item 1 of windowSize as string) & "," & (item 2 of windowSize as string)
                on error
                    return appName & "|" & windowTitle & "|" & processId & "||"
                end try
            end tell
        "#;
        
        let output = Command::new("/usr/bin/osascript")
            .args(["-e", script])
            .output()
            .ok()?;
        
        if !output.status.success() {
            return None;
        }
        
        let output_str = String::from_utf8(output.stdout).ok()?;
        let parts: Vec<&str> = output_str.trim().split('|').collect();
        
        if parts.len() < 5 {
            return None;
        }
        
        let app_name = if !parts[0].is_empty() { 
            Some(parts[0].to_string()) 
        } else { 
            None 
        };
        
        let window_title = if !parts[1].is_empty() { 
            Some(parts[1].to_string()) 
        } else { 
            None 
        };
        
        let process_id = parts[2].parse::<u32>().ok();
        
        let bounds = if parts.len() >= 5 && parts[3].contains(',') && parts[4].contains(',') {
            parse_window_bounds(parts[3], parts[4])
        } else {
            None
        };
        
        Some(EnhancedWindowInfo {
            app_name,
            window_title,
            bounds,
            timestamp: get_current_timestamp(),
            process_id,
            is_focus_changed: true,
        })
    }
    
    /// Windows 窗口信息获取（使用 Windows API）
    #[cfg(target_os = "windows")]
    async fn fetch_windows_window_info(&self) -> Option<EnhancedWindowInfo> {
        use std::ptr;
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, GetWindowRect};
        use winapi::um::processthreadsapi::OpenProcess;
        use winapi::um::psapi::GetModuleBaseNameW;
        use winapi::um::handleapi::CloseHandle;
        use winapi::um::winnt::PROCESS_QUERY_INFORMATION;
        use winapi::shared::windef::RECT;
        
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            
            // 获取窗口标题
            let mut window_title_buf = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, window_title_buf.as_mut_ptr(), window_title_buf.len() as i32);
            let window_title = if title_len > 0 {
                let title_slice = &window_title_buf[..title_len as usize];
                Some(OsString::from_wide(title_slice).to_string_lossy().into_owned())
            } else {
                None
            };
            
            // 获取进程 ID 和应用程序名称
            let mut process_id = 0;
            GetWindowThreadProcessId(hwnd, &mut process_id);
            
            let app_name = if process_id != 0 {
                let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, process_id);
                if !process_handle.is_null() {
                    let mut app_name_buf = [0u16; 512];
                    let name_len = GetModuleBaseNameW(
                        process_handle,
                        ptr::null_mut(),
                        app_name_buf.as_mut_ptr(),
                        app_name_buf.len() as u32,
                    );
                    CloseHandle(process_handle);
                    
                    if name_len > 0 {
                        let name_slice = &app_name_buf[..name_len as usize];
                        Some(OsString::from_wide(name_slice).to_string_lossy().into_owned())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            
            // 获取窗口位置和大小
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            
            let bounds = if GetWindowRect(hwnd, &mut rect) != 0 {
                Some(crate::context::WindowBounds {
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                })
            } else {
                None
            };
            
            Some(EnhancedWindowInfo {
                app_name,
                window_title,
                bounds,
                timestamp: get_current_timestamp(),
                process_id: Some(process_id),
                is_focus_changed: true,
            })
        }
    }
}

// 辅助函数
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn parse_window_bounds(pos_str: &str, size_str: &str) -> Option<crate::context::WindowBounds> {
    let pos_parts: Vec<&str> = pos_str.split(',').collect();
    let size_parts: Vec<&str> = size_str.split(',').collect();
    
    if pos_parts.len() >= 2 && size_parts.len() >= 2 {
        let x = pos_parts[0].trim().parse::<i32>().ok()?;
        let y = pos_parts[1].trim().parse::<i32>().ok()?;
        let width = size_parts[0].trim().parse::<i32>().ok()?;
        let height = size_parts[1].trim().parse::<i32>().ok()?;
        
        Some(crate::context::WindowBounds { x, y, width, height })
    } else {
        None
    }
}

// 全局窗口追踪器实例
lazy_static::lazy_static! {
    pub static ref WINDOW_TRACKER: WindowTracker = WindowTracker::new();
} 