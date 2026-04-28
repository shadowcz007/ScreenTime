use clap::Parser;
use std::path::PathBuf;
use std::env;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    /// API key (or set OPENRECALL_API_KEY environment variable)
    #[clap(short, long, default_value = "default", env = "OPENRECALL_API_KEY")]
    pub api_key: String,

    /// API URL (or set OPENRECALL_API_URL environment variable)
    #[clap(
        long,
        default_value = "http://127.0.0.1:1234/v1/chat/completions",
        env = "OPENRECALL_API_URL"
    )]
    pub api_url: String,

    /// The model to use for analysis
    #[clap(
        short, long,
        default_value = "default",
        env = "OPENRECALL_MODEL"
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

    /// Force start capture loop on launch
    #[clap(
        long,
        env = "START_CAPTURE_ON_LAUNCH",
        help = "启动后强制开启截屏服务（忽略上次停止状态）"
    )]
    pub start_capture_on_launch: bool,

    /// Data directory for all OpenRecall files (logs, screenshots, etc.)
    #[clap(
        long,
        env = "SCREENTIME_DATA_DIR",
        help = "数据存储根目录，包含日志、截图等所有文件"
    )]
    pub data_dir: Option<PathBuf>,

    /// Include installed app list in context (macOS)
    #[clap(
        long,
        env = "INSTALLED_APPS_ENABLED",
        help = "在上下文中注入已安装软件清单（macOS）"
    )]
    pub installed_apps_enabled: bool,

    /// Refresh interval for installed app cache in minutes
    #[clap(
        long,
        default_value = "30",
        env = "INSTALLED_APPS_REFRESH_MINUTES",
        help = "已安装软件清单缓存刷新间隔（分钟）"
    )]
    pub installed_apps_refresh_minutes: u64,

    /// Max items of installed app list included in context
    #[clap(
        long,
        default_value = "300",
        env = "INSTALLED_APPS_MAX_ITEMS",
        help = "注入上下文的已安装软件数量上限"
    )]
    pub installed_apps_max_items: usize,

    /// Include ~/Applications when collecting installed apps
    #[clap(
        long,
        env = "INSTALLED_APPS_INCLUDE_USER_DIR",
        default_value = "true",
        help = "是否扫描 ~/Applications（macOS）"
    )]
    pub installed_apps_include_user_dir: bool,

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

    /// 保留截图文件（默认关闭，分析后删除）
    #[clap(
        long,
        env = "KEEP_SCREENSHOTS",
        help = "分析完成后保留截图文件",
        action = clap::ArgAction::SetTrue
    )]
    pub keep_screenshots: bool,

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

    /// API request timeout in seconds
    #[clap(
        long,
        default_value = "120",
        env = "API_TIMEOUT_SECONDS",
        help = "API请求超时时间（秒）"
    )]
    pub api_timeout: u64,

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

    /// Service control port (Windows only)
    #[clap(
        long,
        default_value = "5830",
        env = "SERVICE_CONTROL_PORT"
    )]
    pub control_port: u16,

    /// OpenClaw agent webhook full URL (e.g. http://127.0.0.1:18789/hooks/agent). When set with openclaw-token, OpenRecall will POST summaries to this URL for the agent to summarize.
    #[clap(long, env = "OPENCLAW_URL", help = "OpenClaw agent 完整 URL（如 .../hooks/agent），与 openclaw-token 同时设置时启用上报")]
    pub openclaw_url: Option<String>,

    /// OpenClaw webhook token for /hooks/agent. Required when openclaw-url is set.
    #[clap(long, env = "OPENCLAW_TOKEN", help = "OpenClaw webhook 令牌")]
    pub openclaw_token: Option<String>,

    /// Interval in minutes between OpenClaw report (default 30). Only used when openclaw-url and openclaw-token are set.
    #[clap(
        long,
        default_value = "30",
        env = "OPENCLAW_REPORT_INTERVAL_MINUTES",
        help = "向 OpenClaw 上报的间隔（分钟）"
    )]
    pub openclaw_report_interval_minutes: u64,

    /// Enable clipboard watcher
    #[clap(long, env = "CLIPBOARD_ENABLED", help = "启用剪贴板监听")]
    pub clipboard_enabled: bool,

    /// Clipboard polling interval in milliseconds
    #[clap(
        long,
        default_value = "500",
        env = "CLIPBOARD_INTERVAL_MS",
        help = "剪贴板监听轮询间隔（毫秒）"
    )]
    pub clipboard_interval_ms: u64,

    /// Auto save new clipboard items to markdown
    #[clap(long, env = "CLIPBOARD_AUTO_SAVE", help = "自动保存新剪贴板内容为 Markdown")]
    pub clipboard_auto_save: bool,

    /// Enable AI filter for clipboard save decisions
    #[clap(
        long,
        env = "CLIPBOARD_AI_FILTER_ENABLED",
        help = "启用剪贴板 AI 过滤，仅在判定 save=true 时保存"
    )]
    pub clipboard_ai_filter_enabled: bool,

    /// Prompt for clipboard AI filter
    #[clap(
        long,
        env = "CLIPBOARD_AI_FILTER_PROMPT",
        default_value = "你是剪贴板内容过滤器。任务：判断该内容是否应保存，用于后续 web-search 研究。仅当内容包含可研究 URL、研究主题或检索关键词时 save=true。如果是验证码、密码、token、私钥、无意义噪音则 save=false。只输出一行JSON：{\"save\":true|false,\"reason\":\"简短原因\",\"category\":\"url|topic|keywords|other\"}。"
    )]
    pub clipboard_ai_filter_prompt: String,

    /// Min chars before calling clipboard AI filter
    #[clap(
        long,
        env = "CLIPBOARD_AI_MIN_CHARS",
        default_value = "20",
        help = "触发剪贴板 AI 判定的最小字符数"
    )]
    pub clipboard_ai_min_chars: usize,

    /// Timeout for clipboard AI filter request
    #[clap(
        long,
        env = "CLIPBOARD_AI_TIMEOUT_SECONDS",
        default_value = "10",
        help = "剪贴板 AI 判定请求超时时间（秒）"
    )]
    pub clipboard_ai_timeout_seconds: u64,

    /// Save clipboard content when AI filter errors
    #[clap(
        long,
        env = "CLIPBOARD_AI_SAVE_ON_ERROR",
        help = "AI 判定失败时是否仍保存（默认 false）"
    )]
    pub clipboard_ai_save_on_error: bool,

    /// Clipboard export directory
    #[clap(
        long,
        env = "CLIPBOARD_TARGET_DIR",
        help = "剪贴板 Markdown 导出目录，默认使用数据目录下 clipboards/exports"
    )]
    pub clipboard_target_dir: Option<PathBuf>,

    /// Max bytes for one clipboard content
    #[clap(
        long,
        default_value = "200000",
        env = "CLIPBOARD_MAX_BYTES",
        help = "单条剪贴板内容最大字节数，超出将忽略"
    )]
    pub clipboard_max_bytes: usize,
}

impl Config {
    pub fn from_args() -> Self {
        // 工程化默认行为：自动加载当前目录 .env（若存在）
        let _ = dotenvy::dotenv();
        Self::parse()
    }

    /// 运行时热重载：重新读取 .env 并按当前命令行参数重新解析配置
    /// 返回 true 表示配置发生变化
    pub fn reload_from_dotenv_and_args(&mut self) -> Result<bool, clap::Error> {
        let _ = dotenvy::from_filename_override(".env");
        let new_config = Self::try_parse_from(std::env::args_os())?;
        let changed = self.get_config_hash() != new_config.get_config_hash();
        *self = new_config;
        Ok(changed)
    }

    /// 获取数据存储根目录
    pub fn get_data_dir(&self) -> PathBuf {
        // 优先使用命令行或环境变量指定的目录
        if let Some(ref dir) = self.data_dir {
            return dir.clone();
        }

        // 使用环境变量
        if let Some(dir) = env::var_os("SCREENTIME_DATA_DIR") {
            return PathBuf::from(dir);
        }

        // 使用系统默认目录
        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join("Library/Application Support/OpenRecall")
        }
        #[cfg(target_os = "linux")]
        {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share/screentime")
        }
        #[cfg(target_os = "windows")]
        {
            let appdata = env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("OpenRecall")
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            PathBuf::from(".")
        }
    }

    /// 获取截图保存目录
    pub fn get_screenshot_dir(&self) -> PathBuf {
        self.get_data_dir().join("screenshots")
    }

    /// 获取按日期分类的日志目录
    pub fn get_logs_dir(&self) -> PathBuf {
        self.get_data_dir().join("logs")
    }

    /// 获取剪贴板数据目录
    pub fn get_clipboard_dir(&self) -> PathBuf {
        self.get_data_dir().join("clipboards")
    }

    /// 获取剪贴板存储文件路径
    pub fn get_clipboard_store_path(&self) -> PathBuf {
        self.get_clipboard_dir().join("history.json")
    }

    /// 获取剪贴板索引文件路径
    pub fn get_clipboard_index_path(&self) -> PathBuf {
        self.get_clipboard_dir().join("index.json")
    }

    /// 获取剪贴板导出目录
    pub fn get_clipboard_export_dir(&self) -> PathBuf {
        if let Some(path) = &self.clipboard_target_dir {
            return path.clone();
        }
        self.get_clipboard_dir().join("exports")
    }

    /// 获取指定日期的日志文件路径
    pub fn get_daily_log_path(&self, date: &str) -> PathBuf {
        self.get_logs_dir().join(format!("{}.json", date))
    }

    /// 获取状态文件路径
    pub fn get_state_path(&self) -> PathBuf {
        if let Some(path) = &self.state_path {
            return path.clone();
        }
        
        let data_dir = self.get_data_dir();
        data_dir.join("service_state.json")
    }

    /// 获取控制socket路径
    pub fn get_socket_path(&self) -> PathBuf {
        if let Some(path) = &self.socket_path {
            return path.clone();
        }
        
        let data_dir = self.get_data_dir();
        data_dir.join("service.sock")
    }

    /// 获取控制端口（Windows系统使用）
    pub fn get_control_port(&self) -> u16 {
        self.control_port
    }

    /// 是否启用 OpenClaw 上报（url 与 token 均提供时为 true）
    pub fn openclaw_enabled(&self) -> bool {
        self.openclaw_url.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            && self.openclaw_token.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
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
        self.installed_apps_enabled.hash(&mut hasher);
        self.installed_apps_refresh_minutes.hash(&mut hasher);
        self.installed_apps_max_items.hash(&mut hasher);
        self.installed_apps_include_user_dir.hash(&mut hasher);
        self.image_target_width.hash(&mut hasher);
        self.image_grayscale.hash(&mut hasher);
        self.no_image_grayscale.hash(&mut hasher);
        self.keep_screenshots.hash(&mut hasher);
        self.api_timeout.hash(&mut hasher);
        self.clipboard_enabled.hash(&mut hasher);
        self.clipboard_interval_ms.hash(&mut hasher);
        self.clipboard_auto_save.hash(&mut hasher);
        self.clipboard_ai_filter_enabled.hash(&mut hasher);
        self.clipboard_ai_filter_prompt.hash(&mut hasher);
        self.clipboard_ai_min_chars.hash(&mut hasher);
        self.clipboard_ai_timeout_seconds.hash(&mut hasher);
        self.clipboard_ai_save_on_error.hash(&mut hasher);
        self.clipboard_max_bytes.hash(&mut hasher);
        hasher.finish().to_string()
    }
}