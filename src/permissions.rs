use std::process::Command;
use std::error::Error;

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

/// 检查屏幕录制权限（macOS）
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
    
    #[cfg(not(target_os = "macos"))]
    {
        // 非 macOS 系统假设有权限
        true
    }
}

/// 检查辅助功能权限（macOS）
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
    
    #[cfg(not(target_os = "macos"))]
    {
        // 非 macOS 系统假设有权限
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
    
    #[cfg(not(target_os = "macos"))]
    {
        println!("非 macOS 系统，无需打开权限设置");
        Ok(())
    }
}

/// 显示权限请求提示并引导用户
pub fn prompt_for_permissions(status: &PermissionStatus) -> Result<(), Box<dyn Error>> {
    if status.all_granted() {
        println!("✅ 所有权限已授权，可以正常使用！");
        return Ok(());
    }
    
    println!("\n⚠️  缺少必要权限，程序需要以下权限才能正常工作：");
    
    if !status.screen_recording {
        println!("\n📱 屏幕录制权限:");
        println!("   - 用途：截取屏幕截图进行分析");
        println!("   - 操作：请在弹出的系统偏好设置中，找到 'ScreenTime' 并勾选");
        println!("   - 提示：可能需要输入管理员密码");
        
        if cfg!(target_os = "macos") {
            println!("\n正在打开屏幕录制权限设置...");
            if let Err(e) = open_permission_settings("screen_recording") {
                eprintln!("无法自动打开设置页面: {}", e);
                println!("请手动打开：系统偏好设置 -> 安全性与隐私 -> 隐私 -> 屏幕录制");
            }
        }
    }
    
    if !status.accessibility {
        println!("\n🔍 辅助功能权限:");
        println!("   - 用途：获取当前活跃窗口和应用程序信息");
        println!("   - 操作：请在弹出的系统偏好设置中，找到 'ScreenTime' 并勾选");
        println!("   - 注意：这有助于AI更准确地分析您的使用情况");
        
        if cfg!(target_os = "macos") {
            println!("\n正在打开辅助功能权限设置...");
            if let Err(e) = open_permission_settings("accessibility") {
                eprintln!("无法自动打开设置页面: {}", e);
                println!("请手动打开：系统偏好设置 -> 安全性与隐私 -> 隐私 -> 辅助功能");
            }
        }
    }
    
    println!("\n📋 授权步骤:");
    println!("1. 在弹出的系统偏好设置窗口中");
    println!("2. 点击左下角的锁图标解锁（需要管理员密码）");
    println!("3. 找到 'ScreenTime' 或 'screen_time' 应用");
    println!("4. 勾选对应的复选框");
    println!("5. 重新启动本程序");
    
    println!("\n⚠️  注意：授权后请重新启动程序以使权限生效");
    
    Ok(())
}

/// 等待用户授权后重新检查权限
pub fn wait_for_permissions() -> Result<PermissionStatus, Box<dyn Error>> {
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
pub async fn ensure_permissions() -> Result<PermissionStatus, Box<dyn Error>> {
    let status = check_all_permissions();
    
    if status.has_missing_permissions() {
        prompt_for_permissions(&status)?;
        wait_for_permissions()
    } else {
        Ok(status)
    }
}
