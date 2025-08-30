use image::{ImageFormat, DynamicImage, GenericImageView};
use screenshots::Screen;
use std::error::Error;
use std::fs::File;

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

pub fn capture_screenshot(file_path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    capture_screenshot_with_options(file_path, Some(1440), true)
}

pub fn capture_screenshot_with_options(
    file_path: &str, 
    target_width: Option<u32>, 
    grayscale: bool
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 获取主屏幕
    let screens = Screen::all()?;
    if let Some(screen) = screens.first() {
        // 截取屏幕
        let image = screen.capture()?;
        
        // 将screenshots::Image转换为DynamicImage
        let dynamic_image = DynamicImage::ImageRgba8(image);
        
        // 处理图片：根据参数进行灰度转换和缩放
        let processed_image = process_image_for_analysis(dynamic_image, target_width, grayscale);
        
        // 保存处理后的图片
        let file = File::create(file_path)?;
        processed_image.write_to(&mut std::io::BufWriter::new(file), ImageFormat::Png)?;
        
        Ok(())
    } else {
        Err("未找到屏幕".into())
    }
}