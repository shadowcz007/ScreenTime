use std::process::Command;
use std::error::Error;



#[cfg(target_os = "windows")]
use winapi::um::winuser::{GetDesktopWindow, GetDC, ReleaseDC};
#[cfg(target_os = "windows")]
use winapi::um::wingdi::{CreateCompatibleDC, CreateCompatibleBitmap, DeleteDC, DeleteObject};

#[derive(Debug, Clone)]
pub struct PermissionStatus {
    pub screen_recording: bool,
    pub accessibility: bool,
}

impl PermissionStatus {
    pub fn all_granted(&self) -> bool {
        self.screen_recording && self.accessibility
    }
    
    pub fn has_missing_permissions(&self) -> bool {
        !self.all_granted()
    }
}

/// 检查屏幕录制权限
pub fn check_screen_recording_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        // 尝试使用 screencapture 命令检查权限
        // 如果没有权限，命令会失败
        let output = Command::new("/usr/sbin/screencapture")
            .args(["-t", "png", "-x", "/tmp/permission_test.png"])
            .output();
            
        match output {
            Ok(result) => {
                // 删除测试文件
                let _ = std::fs::remove_file("/tmp/permission_test.png");
                result.status.success()
            }
            Err(_) => false,
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows: 尝试获取桌面设备上下文来检查屏幕录制权限
        unsafe {
            let hwnd = GetDesktopWindow();
            let hdc = GetDC(hwnd);
            if hdc.is_null() {
                return false;
            }
            
            // 尝试创建兼容的设备上下文和位图
            let mem_dc = CreateCompatibleDC(hdc);
            let result = if !mem_dc.is_null() {
                let bitmap = CreateCompatibleBitmap(hdc, 1, 1);
                let has_permission = !bitmap.is_null();
                
                if !bitmap.is_null() {
                    DeleteObject(bitmap as *mut winapi::ctypes::c_void);
                }
                DeleteDC(mem_dc);
                has_permission
            } else {
                false
            };
            
            ReleaseDC(hwnd, hdc);
            result
        }
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // 其他系统假设有权限
        true
    }
}

/// 检查辅助功能权限
pub fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        // 尝试获取前台应用信息来检查辅助功能权限
        let output = Command::new("/usr/bin/osascript")
            .args([
                "-e",
                r#"tell application "System Events" to get name of first process whose frontmost is true"#,
            ])
            .output();
            
        match output {
            Ok(result) => {
                result.status.success() && !result.stdout.is_empty()
            }
            Err(_) => false,
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        // Windows: 尝试使用 PowerShell 获取前台窗口信息来检查权限
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Application]::OpenForms.Count"#,
            ])
            .output();
            
        match output {
            Ok(result) => result.status.success(),
            Err(_) => {
                // 如果 PowerShell 方法失败，尝试简单的 tasklist 命令
                let output = Command::new("tasklist")
                    .args(["/FI", "STATUS eq RUNNING"])
                    .output();
                
                match output {
                    Ok(result) => result.status.success(),
                    Err(_) => true, // 如果都失败则假设有权限
                }
            }
        }
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // 其他系统假设有权限
        true
    }
}

/// 检查所有必需的权限
pub fn check_all_permissions() -> PermissionStatus {
    println!("正在检查系统权限...");
    
    let screen_recording = check_screen_recording_permission();
    let accessibility = check_accessibility_permission();
    
    println!("权限检查结果:");
    println!("  - 屏幕录制权限: {}", if screen_recording { "✅ 已授权" } else { "❌ 未授权" });
    println!("  - 辅助功能权限: {}", if accessibility { "✅ 已授权" } else { "❌ 未授权" });
    
    PermissionStatus {
        screen_recording,
        accessibility,
    }
}

/// 打开系统偏好设置到相应的权限页面
pub fn open_permission_settings(permission_type: &str) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "macos")]
    {
        let url = match permission_type {
            "screen_recording" => "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
            "accessibility" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            _ => return Err("未知的权限类型".into()),
        };
        
        Command::new("/usr/bin/open")
            .arg(url)
            .output()?;
            
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    {
        match permission_type {
            "screen_recording" => {
                // Windows 10/11: 打开隐私设置中的屏幕录制权限
                Command::new("cmd")
                    .args(["/c", "start", "ms-settings:privacy-broadfilesystemaccess"])
                    .output()?;
            },
            "accessibility" => {
                // Windows: 打开辅助功能设置
                Command::new("cmd")
                    .args(["/c", "start", "ms-settings:easeofaccess"])
                    .output()?;
            },
            _ => return Err("未知的权限类型".into()),
        }
        Ok(())
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        println!("当前系统无需打开权限设置");
        Ok(())
    }
}

/// 显示权限请求提示并引导用户
pub fn prompt_for_permissions(status: &PermissionStatus) -> Result<(), Box<dyn Error + Send + Sync>> {
    if status.all_granted() {
        println!("✅ 所有权限已授权，可以正常使用！");
        return Ok(());
    }
    
    println!("\n⚠️  缺少必要权限，程序需要以下权限才能正常工作：");
    
    if !status.screen_recording {
        println!("\n📱 屏幕录制权限:");
        println!("   - 用途：截取屏幕截图进行分析");
        
        if cfg!(target_os = "macos") {
            println!("   - 操作：请在弹出的系统偏好设置中，找到 'ScreenTime' 并勾选");
            println!("   - 提示：可能需要输入管理员密码");
            println!("\n正在打开屏幕录制权限设置...");
            if let Err(e) = open_permission_settings("screen_recording") {
                eprintln!("无法自动打开设置页面: {}", e);
                println!("请手动打开：系统偏好设置 -> 安全性与隐私 -> 隐私 -> 屏幕录制");
            }
        } else if cfg!(target_os = "windows") {
            println!("   - 操作：请在 Windows 设置中允许应用访问屏幕内容");
            println!("   - 提示：可能需要管理员权限");
            println!("\n正在打开 Windows 隐私设置...");
            if let Err(e) = open_permission_settings("screen_recording") {
                eprintln!("无法自动打开设置页面: {}", e);
                println!("请手动打开：设置 -> 隐私 -> 应用权限 -> 屏幕录制");
            }
        }
    }
    
    if !status.accessibility {
        println!("\n🔍 辅助功能权限:");
        println!("   - 用途：获取当前活跃窗口和应用程序信息");
        println!("   - 注意：这有助于AI更准确地分析您的使用情况");
        
        if cfg!(target_os = "macos") {
            println!("   - 操作：请在弹出的系统偏好设置中，找到 'ScreenTime' 并勾选");
            println!("\n正在打开辅助功能权限设置...");
            if let Err(e) = open_permission_settings("accessibility") {
                eprintln!("无法自动打开设置页面: {}", e);
                println!("请手动打开：系统偏好设置 -> 安全性与隐私 -> 隐私 -> 辅助功能");
            }
        } else if cfg!(target_os = "windows") {
            println!("   - 操作：程序将尝试使用 PowerShell 或系统命令获取窗口信息");
            println!("\n正在打开 Windows 辅助功能设置...");
            if let Err(e) = open_permission_settings("accessibility") {
                eprintln!("无法自动打开设置页面: {}", e);
                println!("如需更多权限，请手动打开：设置 -> 轻松使用 -> 其他选项");
            }
        }
    }
    
    if cfg!(target_os = "macos") {
        println!("\n📋 macOS 授权步骤:");
        println!("1. 在弹出的系统偏好设置窗口中");
        println!("2. 点击左下角的锁图标解锁（需要管理员密码）");
        println!("3. 找到 'ScreenTime' 或 'screen_time' 应用");
        println!("4. 勾选对应的复选框");
        println!("5. 重新启动本程序");
    } else if cfg!(target_os = "windows") {
        println!("\n📋 Windows 授权步骤:");
        println!("1. 在弹出的 Windows 设置窗口中");
        println!("2. 找到相关的隐私设置选项");
        println!("3. 允许桌面应用访问相应功能");
        println!("4. 如需要，以管理员身份运行程序");
        println!("5. 重新启动本程序");
    } else {
        println!("\n📋 授权步骤:");
        println!("1. 根据您的操作系统设置相应权限");
        println!("2. 重新启动本程序");
    }
    
    println!("\n⚠️  注意：授权后请重新启动程序以使权限生效");
    
    Ok(())
}

/// 等待用户授权后重新检查权限
pub fn wait_for_permissions() -> Result<PermissionStatus, Box<dyn Error + Send + Sync>> {
    println!("\n按回车键重新检查权限，或输入 'q' 退出程序...");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "q" {
        println!("程序已退出");
        std::process::exit(0);
    }
    
    let status = check_all_permissions();
    
    if status.has_missing_permissions() {
        println!("\n仍有权限未授权，请按照上述步骤完成授权后重新启动程序");
        std::process::exit(1);
    }
    
    Ok(status)
}

/// 完整的权限检查和请求流程
pub async fn ensure_permissions() -> Result<PermissionStatus, Box<dyn Error + Send + Sync>> {
    let status = check_all_permissions();
    
    if status.has_missing_permissions() {
        prompt_for_permissions(&status)?;
        wait_for_permissions()
    } else {
        Ok(status)
    }
}
