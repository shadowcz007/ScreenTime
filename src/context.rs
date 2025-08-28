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
use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::OpenProcess;
#[cfg(target_os = "windows")]
use winapi::um::psapi::GetModuleBaseNameW;
#[cfg(target_os = "windows")]
use winapi::um::handleapi::CloseHandle;
#[cfg(target_os = "windows")]
use winapi::um::winnt::PROCESS_QUERY_INFORMATION;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub memory_mb: u64,
    pub cpu_percent: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveWindowInfo {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub addr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemContext {
    pub username: String,
    pub hostname: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub uptime_secs: u64,
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub processes_top: Vec<ProcessInfo>,
    pub active_window: Option<ActiveWindowInfo>,
    pub interfaces: Vec<NetworkInterfaceInfo>,
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
    let kernel_version = System::kernel_version();
    let uptime_secs = System::uptime();
    let total_memory_mb = sys.total_memory() / 1024;
    let used_memory_mb = sys.used_memory() / 1024;

    // Top N 进程（按内存），并带上当前可得的 CPU 百分比
    let mut procs: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| ProcessInfo {
            name: p.name().to_string(),
            memory_mb: p.memory() / 1024,
            cpu_percent: p.cpu_usage(),
        })
        .collect();
    procs.sort_by_key(|p| std::cmp::Reverse(p.memory_mb));
    procs.truncate(10);

    let interfaces = match get_if_addrs::get_if_addrs() {
        Ok(addrs) => addrs
            .into_iter()
            .map(|i| NetworkInterfaceInfo {
                name: i.name.clone(),
                addr: i.ip().to_string(),
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    let active_window = get_active_window_info().await;

    SystemContext {
        username,
        hostname,
        os_name,
        os_version,
        kernel_version,
        uptime_secs,
        total_memory_mb,
        used_memory_mb,
        processes_top: procs,
        active_window,
        interfaces,
    }
}

/// 获取活跃窗口信息（跨平台支持）
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

        // 如果没有获取到任何信息，返回None；否则返回获取到的信息
        if app_name.is_none() && window_title.is_none() {
            None
        } else {
            Some(ActiveWindowInfo {
                app_name,
                window_title,
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

            // 如果没有获取到任何信息，返回None；否则返回获取到的信息
            if app_name.is_none() && window_title.is_none() {
                None
            } else {
                Some(ActiveWindowInfo {
                    app_name,
                    window_title,
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
        "用户: {}\n主机: {}\nOS: {} {}\n内核: {}\n开机时长: {} 秒\n内存: {}/{} MB\n",
        ctx.username,
        ctx.hostname.clone().unwrap_or_default(),
        ctx.os_name.clone().unwrap_or_default(),
        ctx.os_version.clone().unwrap_or_default(),
        ctx.kernel_version.clone().unwrap_or_default(),
        ctx.uptime_secs,
        ctx.used_memory_mb,
        ctx.total_memory_mb
    ));

    if let Some(w) = &ctx.active_window {
        s.push_str(&format!(
            "前台应用: {}\n窗口标题: {}\n",
            w.app_name.clone().unwrap_or("未知".to_string()),
            w.window_title.clone().unwrap_or("未知".to_string())
        ));
    } else {
        s.push_str("前台应用: [需要辅助功能权限]\n窗口标题: [需要辅助功能权限]\n");
    }

    if !ctx.interfaces.is_empty() {
        s.push_str("网络接口:\n");
        for ni in &ctx.interfaces {
            s.push_str(&format!("  - {}: {}\n", ni.name, ni.addr));
        }
    }

    if !ctx.processes_top.is_empty() {
        s.push_str("Top 进程(按内存):\n");
        for p in &ctx.processes_top {
            s.push_str(&format!(
                "  - {} | mem: {} MB | cpu: {:.1}%\n",
                p.name, p.memory_mb, p.cpu_percent
            ));
        }
    }

    s
}
