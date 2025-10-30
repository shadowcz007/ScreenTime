use serde::{Deserialize, Serialize};
use sysinfo::System;

use tokio::time::{sleep, Duration};

#[cfg(target_os = "windows")]
use std::ptr;
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStringExt;

#[cfg(target_os = "windows")]
use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, GetWindowRect};
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::OpenProcess;
#[cfg(target_os = "windows")]
use winapi::um::psapi::GetModuleBaseNameW;
#[cfg(target_os = "windows")]
use winapi::um::handleapi::CloseHandle;
#[cfg(target_os = "windows")]
use winapi::um::winnt::PROCESS_QUERY_INFORMATION;
#[cfg(target_os = "windows")]
use winapi::shared::windef::RECT;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub cpu_percent: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveWindowInfo {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub bounds: Option<WindowBounds>,
    pub timestamp: Option<u64>,
    pub process_id: Option<u32>,
    pub switch_stats: Option<crate::window_tracker::WindowSwitchStats>,
    pub recent_switches: Option<Vec<crate::window_tracker::WindowSwitchEvent>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemContext {
    pub username: String,
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub processes_top: Vec<ProcessInfo>,
    pub active_window: Option<ActiveWindowInfo>,
}

pub async fn collect_system_context() -> SystemContext {
    let username = whoami::username();

    let mut sys = System::new_all();
    sys.refresh_all();

    // 为 CPU 使用率做第二次刷新（需要两次采样）
    sys.refresh_processes();
    sleep(Duration::from_millis(200)).await;
    sys.refresh_processes();

    let hostname = System::host_name();
    let os_name = System::name();
    let os_version = System::os_version();

    // Top N 进程（按CPU使用率），并带上当前可得的 CPU 百分比
    let mut procs: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| ProcessInfo {
            name: p.name().to_string(),
            cpu_percent: p.cpu_usage(),
        })
        .collect();
    procs.sort_by_key(|p| std::cmp::Reverse(p.cpu_percent as u64));
    procs.truncate(10);



    let active_window = get_enhanced_active_window_info().await;

    SystemContext {
        username,
        hostname,
        os_name,
        os_version,
        processes_top: procs,
        active_window,
    }
}

/// 获取增强的活跃窗口信息（包含追踪数据）
async fn get_enhanced_active_window_info() -> Option<ActiveWindowInfo> {
    use crate::window_tracker::WINDOW_TRACKER;
    
    // 获取窗口信息和统计数据
    let window_info = WINDOW_TRACKER.get_current_window_info().await?;
    let stats = WINDOW_TRACKER.get_stats().await;
    let recent_switches = WINDOW_TRACKER.get_switch_history(Some(5)).await;
    
    Some(ActiveWindowInfo {
        app_name: window_info.app_name,
        window_title: window_info.window_title,
        bounds: window_info.bounds,
        timestamp: Some(window_info.timestamp),
        process_id: window_info.process_id,
        switch_stats: Some(stats),
        recent_switches: Some(recent_switches),
    })
}

/// 获取活跃窗口信息（跨平台支持）- 保持向后兼容
async fn get_active_window_info() -> Option<ActiveWindowInfo> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let app_name = Command::new("/usr/bin/osascript")
            .args([
                "-e",
                r#"tell application "System Events" to get name of first process whose frontmost is true"#,
            ])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let window_title = if app_name.is_some() {
            Command::new("/usr/bin/osascript")
                .args([
                    "-e",
                    r#"tell application "System Events" to tell (first process whose frontmost is true) to get title of front window"#,
                ])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        };

        // 获取窗口位置和大小
        let bounds = if app_name.is_some() {
            Command::new("/usr/bin/osascript")
                .args([
                    "-e",
                    r#"tell application "System Events" to tell (first process whose frontmost is true) to get position of front window"#,
                ])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .and_then(|pos_str| {
                    // 获取窗口大小
                    Command::new("/usr/bin/osascript")
                        .args([
                            "-e",
                            r#"tell application "System Events" to tell (first process whose frontmost is true) to get size of front window"#,
                        ])
                        .output()
                        .ok()
                        .and_then(|o| {
                            if o.status.success() {
                                String::from_utf8(o.stdout).ok()
                            } else {
                                None
                            }
                        })
                        .and_then(|size_str| {
                            parse_window_bounds(&pos_str.trim(), &size_str.trim())
                        })
                })
        } else {
            None
        };

        // 如果没有获取到任何信息，返回None；否则返回获取到的信息
        if app_name.is_none() && window_title.is_none() {
            None
        } else {
            Some(ActiveWindowInfo {
                app_name,
                window_title,
                bounds,
                timestamp: None,
                process_id: None,
                switch_stats: None,
                recent_switches: None,
            })
        }
    }

    #[cfg(target_os = "windows")]
    {
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
                Some(WindowBounds {
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                })
            } else {
                None
            };

            // 如果没有获取到任何信息，返回None；否则返回获取到的信息
            if app_name.is_none() && window_title.is_none() {
                None
            } else {
                Some(ActiveWindowInfo {
                    app_name,
                    window_title,
                    bounds,
                    timestamp: None,
                    process_id: Some(process_id),
                    switch_stats: None,
                    recent_switches: None,
                })
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

pub fn format_context_as_text(ctx: &SystemContext) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "用户: {}\n主机: {}\nOS: {} {}\n",
        ctx.username,
        ctx.hostname.clone().unwrap_or_default(),
        ctx.os_name.clone().unwrap_or_default(),
        ctx.os_version.clone().unwrap_or_default()
    ));

    if let Some(w) = &ctx.active_window {
        s.push_str(&format!(
            "前台应用: {}\n窗口标题: {}\n",
            w.app_name.clone().unwrap_or("未知".to_string()),
            w.window_title.clone().unwrap_or("未知".to_string())
        ));
        
        // 添加窗口切换统计信息
        if let Some(stats) = &w.switch_stats {
            s.push_str(&format!(
                "窗口切换统计:\n  - 总切换次数: {}\n  - 当前会话时长: {:.1}分钟\n",
                stats.total_switches,
                stats.current_session_duration_ms as f64 / 60000.0
            ));
            
            if !stats.most_used_apps.is_empty() {
                s.push_str("  - 最常用应用:\n");
                for (app, duration) in stats.most_used_apps.iter().take(3) {
                    s.push_str(&format!(
                        "    * {}: {:.1}分钟\n",
                        app,
                        *duration as f64 / 60000.0
                    ));
                }
            }
        }
        
        // 添加最近的窗口切换记录
        if let Some(switches) = &w.recent_switches {
            if !switches.is_empty() {
                s.push_str("最近窗口切换:\n");
                for switch in switches.iter().take(3) {
                    let from_app = switch.from_app.as_deref().unwrap_or("未知");
                    let to_app = switch.to_app.as_deref().unwrap_or("未知");
                    s.push_str(&format!(
                        "  - {} -> {} (停留{:.1}秒)\n",
                        from_app,
                        to_app,
                        switch.duration_ms as f64 / 1000.0
                    ));
                }
            }
        }
    } else {
        s.push_str("前台应用: [需要辅助功能权限]\n窗口标题: [需要辅助功能权限]\n");
    }



    if !ctx.processes_top.is_empty() {
        s.push_str("Top 进程:\n");
        for p in &ctx.processes_top {
            s.push_str(&format!(
                "  - {} | cpu: {:.1}%\n",
                p.name, p.cpu_percent
            ));
        }
    }

    s
}

/// 解析macOS AppleScript返回的窗口位置和大小字符串
fn parse_window_bounds(position_str: &str, size_str: &str) -> Option<WindowBounds> {
    // AppleScript返回的格式通常是 "x, y" 和 "width, height"
    let parse_coords = |s: &str| -> Option<(i32, i32)> {
        let parts: Vec<&str> = s.split(',').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            if let (Ok(first), Ok(second)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                return Some((first, second));
            }
        }
        None
    };
    
    if let (Some((x, y)), Some((width, height))) = (parse_coords(position_str), parse_coords(size_str)) {
        Some(WindowBounds { x, y, width, height })
    } else {
        None
    }
}
