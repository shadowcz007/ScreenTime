use image::{ImageFormat, DynamicImage, GenericImageView};
use screenshots::Screen;
use std::error::Error;
use std::fs::File;
use crate::context::{WindowBounds, ActiveWindowInfo};

/// 处理图片：根据参数进行灰度转换和缩放
pub fn process_image_for_analysis(
    image: DynamicImage, 
    target_width: Option<u32>, 
    grayscale: bool
) -> DynamicImage {
    let mut processed_image = image;
    
    // 转换为灰度图（如果需要）
    if grayscale {
        processed_image = processed_image.grayscale();
    }
    
    // 缩放处理（如果需要）
    if let Some(width) = target_width {
        if width > 0 {
            let (current_width, current_height) = processed_image.dimensions();
            
            if current_width > width {
                // 计算新的高度，保持宽高比
                let scale_factor = width as f32 / current_width as f32;
                let new_height = (current_height as f32 * scale_factor) as u32;
                
                // 缩放到目标尺寸
                processed_image = processed_image.resize(width, new_height, image::imageops::FilterType::Lanczos3);
            }
        }
    }
    
    processed_image
}

// 保留用于向后兼容
#[allow(dead_code)]
fn capture_screenshot(file_path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    capture_screenshot_with_options(file_path, Some(1440), true)
}

// 保留用于向后兼容
#[allow(dead_code)]
pub fn capture_screenshot_with_options(
    file_path: &str, 
    target_width: Option<u32>, 
    grayscale: bool
) -> Result<(), Box<dyn Error + Send + Sync>> {
    capture_screenshot_smart(file_path, target_width, grayscale, None)
}

/// 智能截图：根据活跃窗口信息选择最佳屏幕
pub fn capture_screenshot_smart(
    file_path: &str, 
    target_width: Option<u32>, 
    grayscale: bool,
    active_window: Option<&ActiveWindowInfo>
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let screens = Screen::all()?;
    if screens.is_empty() {
        return Err("未找到屏幕".into());
    }

    // 选择要截图的屏幕
    let target_screen = select_best_screen(&screens, active_window);
    
    // 截取屏幕
    let image = target_screen.capture()?;
    
    // 将screenshots::Image转换为DynamicImage
    let dynamic_image = DynamicImage::ImageRgba8(image);
    
    // 处理图片：根据参数进行灰度转换和缩放
    let processed_image = process_image_for_analysis(dynamic_image, target_width, grayscale);
    
    // 保存处理后的图片
    let file = File::create(file_path)?;
    processed_image.write_to(&mut std::io::BufWriter::new(file), ImageFormat::Png)?;
    
    Ok(())
}

/// 选择最佳屏幕进行截图
fn select_best_screen<'a>(screens: &'a [Screen], active_window: Option<&ActiveWindowInfo>) -> &'a Screen {
    // 如果只有一个屏幕，直接返回
    if screens.len() == 1 {
        return &screens[0];
    }
    
    // 如果有活跃窗口信息且包含位置信息，寻找包含该窗口的屏幕
    if let Some(window) = active_window {
        if let Some(bounds) = &window.bounds {
            if let Some(screen) = find_screen_containing_window(screens, bounds) {
                println!("📍 选择包含活跃窗口的屏幕 (窗口位置: {}x{} at {},{}) ", 
                    bounds.width, bounds.height, bounds.x, bounds.y);
                return screen;
            }
        }
    }
    
    // 如果无法确定活跃窗口所在屏幕，选择主屏幕（通常是第一个）
    println!("🖥️ 使用主屏幕进行截图");
    &screens[0]
}

/// 查找包含指定窗口的屏幕
fn find_screen_containing_window<'a>(screens: &'a [Screen], window_bounds: &WindowBounds) -> Option<&'a Screen> {
    // 计算窗口中心点
    let window_center_x = window_bounds.x + window_bounds.width / 2;
    let window_center_y = window_bounds.y + window_bounds.height / 2;
    
    for screen in screens {
        let display = screen.display_info;
        
        // 检查窗口中心点是否在这个屏幕内
        if window_center_x >= display.x 
            && window_center_x < display.x + display.width as i32
            && window_center_y >= display.y 
            && window_center_y < display.y + display.height as i32 {
            return Some(screen);
        }
    }
    
    None
}