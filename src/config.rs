use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    /// SiliconFlow API key (or set SILICONFLOW_API_KEY environment variable)
    #[clap(short, long, env = "SILICONFLOW_API_KEY")]
    pub api_key: String,

    /// The model to use for analysis
    #[clap(
        short,
        long,
        default_value = "THUDM/GLM-4.1V-9B-Thinking",
        env = "SILICONFLOW_MODEL"
    )]
    pub model: String,

    /// The prompt to use for analysis
    #[clap(
        short,
        long,
        default_value = "请描述这张截图中用户正在使用什么软件，在做什么，按照格式：【软件】【主要工作摘要】。",
        env = "SCREEN_ANALYSIS_PROMPT"
    )]
    pub prompt: String,

    /// The interval between screenshots in seconds
    #[clap(
        short,
        long,
        default_value = "60",
        env = "SCREENSHOT_INTERVAL_SECONDS"
    )]
    pub interval: u64,

    /// Directory to save screenshots
    #[clap(
        short,
        long,
        default_value = "screenshots",
        env = "SCREENSHOT_DIRECTORY"
    )]
    pub screenshot_dir: PathBuf,

    /// Path to save activity log
    #[clap(
        short,
        long,
        default_value = "activity_log.json",
        env = "ACTIVITY_LOG_PATH"
    )]
    pub log_path: PathBuf,
}

impl Config {
    pub fn from_args() -> Self {
        Self::parse()
    }
}