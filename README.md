# ScreenTime

这是一个用 Rust 编写的屏幕时间监控工具，它可以定期截取屏幕截图，并使用 SiliconFlow 提供的视觉模型分析用户在特定时间点的活动。

## 功能

- **智能截屏监控**: 定期间隔截屏并分析
- **AI图像分析**: 使用多模态模型分析截图内容
- **丰富系统上下文**: 自动收集系统信息、进程状态、窗口信息等
- **权限自动检查**: 启动时自动检查并引导用户授权必要权限
- **完整活动日志**: 记录分析结果、系统状态和截图路径
- **灵活配置**: 支持命令行参数和环境变量配置

## 安装

1. 确保你已经安装了 Rust 和 Cargo。
2. 克隆此仓库：
   ```bash
   git clone <repository-url>
   cd ScreenTime
   ```
3. 构建项目：
   ```bash
   cargo build --release
   ```

## 权限要求 (macOS)

ScreenTime 需要以下系统权限才能正常工作：

### 📱 屏幕录制权限
- **用途**: 截取屏幕截图进行AI分析
- **设置路径**: 系统偏好设置 → 安全性与隐私 → 隐私 → 屏幕录制
- **说明**: 程序会自动检查此权限并引导您授权

### 🔍 辅助功能权限
- **用途**: 获取当前活跃窗口和应用程序信息
- **设置路径**: 系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能  
- **说明**: 此权限有助于AI更准确地分析您的使用情况

**首次运行时，程序会自动检查权限状态并打开相应的设置页面指导您完成授权。**

## 配置

你可以通过以下方式配置程序：

### 命令行参数

- `-a, --api-key <API_KEY>`: SiliconFlow API 密钥 [环境变量: SILICONFLOW_API_KEY]
- `-m, --model <MODEL>`: 用于分析的模型 [默认: THUDM/GLM-4.1V-9B-Thinking] [环境变量: SILICONFLOW_MODEL]
- `-p, --prompt <PROMPT>`: 用于分析的提示 [默认: 请描述这张图片中用户正在做什么，尽可能详细一些。] [环境变量: SCREEN_ANALYSIS_PROMPT]
- `-i, --interval <INTERVAL>`: 截图间隔（秒） [默认: 60] [环境变量: SCREENSHOT_INTERVAL_SECONDS]
- `-s, --screenshot-dir <SCREENSHOT_DIR>`: 截图保存目录 [默认: screenshots] [环境变量: SCREENSHOT_DIRECTORY]
- `-l, --log-path <LOG_PATH>`: 活动日志保存路径 [默认: activity_log.json] [环境变量: ACTIVITY_LOG_PATH]

### 环境变量

你也可以通过设置环境变量来配置程序：

```bash
export SILICONFLOW_API_KEY=your_api_key_here
export SILICONFLOW_MODEL=Qwen/Qwen2-VL-7B-Instruct
export SCREEN_ANALYSIS_PROMPT="请描述这张图片中用户正在做什么，尽可能详细一些。"
export SCREENSHOT_INTERVAL_SECONDS=60
export SCREENSHOT_DIRECTORY=screenshots
export ACTIVITY_LOG_PATH=activity_log.json
```

## 使用

### 使用命令行参数：

```bash
./target/release/screen_time --api-key your_api_key_here --interval 30 --screenshot-dir ./my_screenshots
```

### 使用环境变量：

```bash
export SILICONFLOW_API_KEY=your_siliconflow_api_key_here
./target/release/screen_time
```

### 混合使用命令行参数和环境变量：

```bash
export SILICONFLOW_API_KEY=your_siliconflow_api_key_here
./target/release/screen_time --interval 30 --screenshot-dir ./my_screenshots
```

程序将按照指定的时间间隔截取屏幕截图，并将分析结果保存到指定的日志文件中。

## 项目结构

- `src/main.rs`: 程序入口点
- `src/screenshot.rs`: 屏幕截图功能
- `src/siliconflow.rs`: SiliconFlow API 调用功能
- `src/logger.rs`: 日志记录功能
- `src/models.rs`: 数据模型
- `src/capture.rs`: 截屏循环控制
- `src/config.rs`: 配置解析

## 依赖

- `tokio`: 异步运行时
- `image`: 图像处理
- `screenshot`: 屏幕截图
- `reqwest`: HTTP 客户端
- `serde`: 序列化/反序列化
- `chrono`: 日期时间处理
- `clap`: 命令行参数解析

## 注意事项

- 请确保你的系统允许屏幕录制权限。
- SiliconFlow API 调用可能会产生费用，请根据你的使用情况进行监控。