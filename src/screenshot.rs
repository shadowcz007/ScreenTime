use image::ImageFormat;
use screenshots::Screen;
use std::error::Error;
use std::fs::File;

pub fn capture_screenshot(file_path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 获取主屏幕
    let screens = Screen::all()?;
    if let Some(screen) = screens.first() {
        // 截取屏幕
        let image = screen.capture()?;
        
        // 保存图片
        let file = File::create(file_path)?;
        image.write_to(&mut std::io::BufWriter::new(file), ImageFormat::Png)?;
        
        Ok(())
    } else {
        Err("未找到屏幕".into())
    }
}