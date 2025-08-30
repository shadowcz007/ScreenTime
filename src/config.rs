use clap::Parser;
use std::path::PathBuf;
use std::env;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    /// SiliconFlow API key (or set SILICONFLOW_API_KEY environment variable)
    #[clap(short, long, env = "SILICONFLOW_API_KEY")]
    pub api_key: String,

    /// SiliconFlow API URL (or set SILICONFLOW_API_URL environment variable)
    #[clap(
        long,
        default_value = "https://api.siliconflow.cn/v1/chat/completions",
        env = "SILICONFLOW_API_URL"
    )]
    pub api_url: String,

    /// The model to use for analysis
    #[clap(
        short, long,
        default_value = "THUDM/GLM-4.1V-9B-Thinking",
        env = "SILICONFLOW_MODEL"
    )]
    pub model: String,

    /// The prompt to use for analysis
    #[clap(
        short, long,
        default_value = "请描述这张截图中用户正在使用什么软件，在做什么，并进行分类，严格按照格式输出结果：【类型】【软件】【主要工作摘要】。",
        env = "SCREEN_ANALYSIS_PROMPT"
    )]
    pub prompt: String,

    /// The interval between screenshots in seconds
    #[clap(
        short, long,
        default_value = "60",
        env = "SCREENSHOT_INTERVAL_SECONDS"
    )]
    pub interval: u64,

    /// Directory to save screenshots
    #[clap(
        short, long,
        default_value = "screenshots",
        env = "SCREENSHOT_DIRECTORY"
    )]
    pub screenshot_dir: PathBuf,

    /// Path to save activity log
    #[clap(
        short, long,
        default_value = "activity_log.json",
        env = "ACTIVITY_LOG_PATH"
    )]
    pub log_path: PathBuf,

    /// Path to save service state
    #[clap(
        long,
        env = "SERVICE_STATE_PATH"
    )]
    pub state_path: Option<PathBuf>,

    /// Target width for image processing (None to keep original size)
    #[clap(
        long,
        default_value = "1440",
        env = "IMAGE_TARGET_WIDTH",
        help = "图片处理的目标宽度，设置为0保持原图尺寸"
    )]
    pub image_target_width: u32,

    /// Enable grayscale conversion for image processing
    #[clap(
        long,
        default_value = "true",
        env = "IMAGE_GRAYSCALE",
        help = "是否将图片转换为灰度图",
        action = clap::ArgAction::SetTrue,
        overrides_with = "no_image_grayscale"
    )]
    pub image_grayscale: bool,

    /// Disable grayscale conversion for image processing
    #[clap(
        long,
        help = "禁用灰度转换，保持彩色图片",
        action = clap::ArgAction::SetTrue
    )]
    pub no_image_grayscale: bool,

    /// Enable MCP server mode (default: standalone service mode)
    #[clap(long, help = "启用MCP服务器模式（默认：独立截屏服务模式）")]
    pub mcp: bool,

    /// MCP server port number
    #[clap(
        long,
        default_value = "6672",
        env = "MCP_PORT",
        help = "MCP服务器端口号"
    )]
    pub mcp_port: u16,

    /// Test a new prompt using existing screenshots and context
    #[clap(long, help = "测试新的prompt，使用现有的截图和上下文重新计算")]
    pub test_prompt: Option<String>,

    /// Path to save test results
    #[clap(
        long,
        default_value = "test_log.json",
        env = "TEST_LOG_PATH"
    )]
    pub test_log_path: PathBuf,



    /// Service control socket path
    #[clap(
        long,
        env = "SERVICE_SOCKET_PATH"
    )]
    pub socket_path: Option<PathBuf>,
}

impl Config {
    pub fn from_args() -> Self {
        Self::parse()
    }

    /// 获取系统数据目录
    pub fn get_data_dir() -> PathBuf {
        if let Some(dir) = env::var_os("SCREENTIME_DATA_DIR") {
            return PathBuf::from(dir);
        }

        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("Library/Application Support/ScreenTime")
        }
        #[cfg(target_os = "linux")]
        {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share/screentime")
        }
        #[cfg(target_os = "windows")]
        {
            let appdata = env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("ScreenTime")
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            PathBuf::from(".")
        }
    }

    /// 获取状态文件路径
    pub fn get_state_path(&self) -> PathBuf {
        if let Some(path) = &self.state_path {
            return path.clone();
        }
        
        let data_dir = Self::get_data_dir();
        data_dir.join("service_state.json")
    }

    /// 获取控制socket路径
    pub fn get_socket_path(&self) -> PathBuf {
        if let Some(path) = &self.socket_path {
            return path.clone();
        }
        
        let data_dir = Self::get_data_dir();
        data_dir.join("service.sock")
    }

    /// 生成配置哈希值
    pub fn get_config_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.api_url.hash(&mut hasher);
        self.model.hash(&mut hasher);
        self.prompt.hash(&mut hasher);
        self.interval.hash(&mut hasher);
        self.image_target_width.hash(&mut hasher);
        self.image_grayscale.hash(&mut hasher);
        self.no_image_grayscale.hash(&mut hasher);
        hasher.finish().to_string()
    }
}