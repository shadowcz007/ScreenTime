# ScreenTime

一个功能强大的屏幕时间监控工具，使用 Rust 编写，集成了 AI 图像分析和 MCP (Model Context Protocol) 服务功能。它可以定期截取屏幕截图，使用 SiliconFlow 提供的视觉模型分析用户活动，并提供丰富的系统上下文信息。

## ✨ 主要功能

- **🤖 AI 智能分析**: 使用多模态模型分析截图内容，理解用户活动
- **📊 丰富系统上下文**: 自动收集系统信息、进程状态、窗口信息、网络接口等
- **🔗 MCP 服务支持**: 提供 Model Context Protocol 服务，支持远程控制
- **🛡️ 权限自动检查**: 启动时自动检查并引导用户授权必要权限
- **📝 完整活动日志**: 记录分析结果、系统状态和截图路径
- **⚙️ 灵活配置**: 支持命令行参数和环境变量配置
- **🌐 Web 服务**: 内置 SSE (Server-Sent Events) 服务器，支持实时数据推送

## 🚀 快速开始

### 安装

1. 确保你已经安装了 Rust 和 Cargo
2. 克隆此仓库：
   ```bash
   git clone <repository-url>
   cd ScreenTime
   ```
3. 构建项目：
   ```bash
   cargo build --release
   ```

### 基本使用

#### 1. 标准监控模式

```bash
# 使用命令行参数
./target/release/screen_time --api-key your_api_key_here --interval 30

# 使用环境变量
export SILICONFLOW_API_KEY=your_api_key_here
./target/release/screen_time
```

#### 2. MCP 服务器模式

```bash
# 启动 MCP 服务器
./target/release/screen_time --mcp --api-key your_api_key_here
```

MCP 服务器将在 `127.0.0.1:8000` 启动，提供以下工具：
- `monitor`: 控制监控状态 (start/stop/status)
- `read_logs`: 读取活动日志
- `take_screenshot`: 手动截取屏幕截图

## 🔐 权限要求

ScreenTime 需要以下系统权限才能正常工作：

### macOS 系统

#### 📱 屏幕录制权限
- **用途**: 截取屏幕截图进行AI分析
- **设置路径**: 系统偏好设置 → 安全性与隐私 → 隐私 → 屏幕录制
- **说明**: 程序会自动检查此权限并引导您授权

#### 🔍 辅助功能权限
- **用途**: 获取当前活跃窗口和应用程序信息
- **设置路径**: 系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能  
- **说明**: 此权限有助于AI更准确地分析您的使用情况

### Windows 系统

#### 📱 屏幕录制权限
- **用途**: 截取屏幕截图进行AI分析
- **设置路径**: 设置 → 隐私 → 应用权限 → 屏幕录制
- **说明**: Windows 10/11 需要允许应用访问屏幕内容

#### 🔍 活跃窗口信息
- **用途**: 获取当前活跃窗口和应用程序信息
- **说明**: 程序使用 Windows API 和 PowerShell 获取窗口信息
- **注意**: 某些情况下可能需要管理员权限

**首次运行时，程序会自动检查权限状态并打开相应的设置页面指导您完成授权。**

## ⚙️ 配置选项

### 命令行参数

| 参数 | 环境变量 | 默认值 | 说明 |
|------|----------|--------|------|
| `-a, --api-key <API_KEY>` | `SILICONFLOW_API_KEY` | - | SiliconFlow API 密钥 |
| `-m, --model <MODEL>` | `SILICONFLOW_MODEL` | `THUDM/GLM-4.1V-9B-Thinking` | 用于分析的模型 |
| `-p, --prompt <PROMPT>` | `SCREEN_ANALYSIS_PROMPT` | `请描述这张图片中用户正在做什么，尽可能详细一些。` | 用于分析的提示 |
| `-i, --interval <INTERVAL>` | `SCREENSHOT_INTERVAL_SECONDS` | `60` | 截图间隔（秒） |
| `-s, --screenshot-dir <SCREENSHOT_DIR>` | `SCREENSHOT_DIRECTORY` | `screenshots` | 截图保存目录 |
| `-l, --log-path <LOG_PATH>` | `ACTIVITY_LOG_PATH` | `activity_log.json` | 活动日志保存路径 |
| `--mcp` | - | `false` | 启动 MCP 服务器模式 |

### 环境变量配置示例

```bash
export SILICONFLOW_API_KEY=your_api_key_here
export SILICONFLOW_MODEL=Qwen/Qwen2-VL-7B-Instruct
export SCREEN_ANALYSIS_PROMPT="请描述这张图片中用户正在做什么，尽可能详细一些。"
export SCREENSHOT_INTERVAL_SECONDS=60
export SCREENSHOT_DIRECTORY=screenshots
export ACTIVITY_LOG_PATH=activity_log.json
```

## 📊 系统上下文收集

ScreenTime 会自动收集以下系统信息，为 AI 分析提供更丰富的上下文：

### 🔧 系统信息
- 用户名和主机名
- 操作系统名称和版本
- 内核版本
- 系统运行时间

### 💾 资源使用情况
- 内存使用情况（总内存/已用内存）
- Top 10 进程（按内存使用量排序）
- 进程 CPU 使用率

### 🖥️ 窗口信息
- 当前活跃应用程序
- 前台窗口标题

### 🌐 网络信息
- 网络接口列表
- IP 地址信息

## 📁 项目结构

```
ScreenTime/
├── src/
│   ├── main.rs              # 程序入口点
│   ├── config.rs            # 配置解析
│   ├── screenshot.rs        # 屏幕截图功能
│   ├── siliconflow.rs       # SiliconFlow API 调用
│   ├── logger.rs            # 日志记录功能
│   ├── models.rs            # 数据模型定义
│   ├── capture.rs           # 截屏循环控制
│   ├── context.rs           # 系统上下文收集
│   ├── permissions.rs       # 权限检查和请求
│   └── mcp_service.rs       # MCP 服务实现
├── examples/                # 示例代码
├── Cargo.toml              # 项目配置和依赖
└── README.md               # 项目文档
```

## 🔧 依赖库

### 核心依赖
- `tokio`: 异步运行时
- `image`: 图像处理
- `screenshots`: 屏幕截图
- `reqwest`: HTTP 客户端
- `serde`: 序列化/反序列化
- `chrono`: 日期时间处理
- `clap`: 命令行参数解析

### 系统信息收集
- `sysinfo`: 系统信息获取
- `whoami`: 用户信息获取
- `get_if_addrs`: 网络接口信息

### Web 服务
- `axum`: Web 框架
- `tower-http`: HTTP 中间件
- `rmcp`: Model Context Protocol 实现

### 日志和追踪
- `tracing`: 结构化日志
- `tracing-subscriber`: 日志订阅器

## 📝 日志格式

活动日志以 JSON 格式保存，包含以下信息：

```json
{
  "timestamp": "2024-01-01T12:00:00+08:00",
  "description": "AI 分析结果描述",
  "context": {
    "username": "用户名",
    "hostname": "主机名",
    "os_name": "macOS",
    "uptime_secs": 3600,
    "total_memory_mb": 16384,
    "used_memory_mb": 8192,
    "processes_top": [...],
    "active_window": {
      "app_name": "应用程序名",
      "window_title": "窗口标题"
    },
    "interfaces": [...]
  },
  "screenshot_path": "screenshots/2024-01-01_12-00-00.png"
}
```

## 🌐 MCP 服务 API

当以 MCP 模式运行时，服务提供以下工具：

### monitor
控制监控状态
- `action`: "start" | "stop" | "status"

### read_logs
读取活动日志
- `start_time`: 开始时间（可选）
- `end_time`: 结束时间（可选）
- `limit`: 限制返回条数（可选）
- `detailed`: 是否包含详细信息（可选）

### take_screenshot
手动截取屏幕截图

## ⚠️ 注意事项

1. **权限要求**: 请确保你的系统允许屏幕录制权限
2. **API 费用**: SiliconFlow API 调用可能会产生费用，请根据你的使用情况进行监控
3. **隐私保护**: 截图和分析结果会保存在本地，请注意保护个人隐私
4. **系统兼容性**: 支持 macOS 和 Windows 系统，Linux 系统的支持正在开发中
5. **网络连接**: 需要稳定的网络连接以调用 AI 分析服务
6. **管理员权限**: Windows 系统可能需要管理员权限来获取完整的窗口信息

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目！

## 📄 许可证

本项目采用 MIT 许可证。