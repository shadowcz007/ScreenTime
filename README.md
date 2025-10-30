# ScreenTime

一个功能强大的屏幕时间监控工具，使用 Rust 编写，集成了 AI 图像分析和 MCP (Model Context Protocol) 服务功能。它可以定期截取屏幕截图，使用 SiliconFlow 提供的视觉模型分析用户活动，并提供丰富的系统上下文信息。

## 📋 更新日志

### v0.3.0 (2024-12-19)
- 🔧 **重大重构**: 配置简化优化，移除复杂的路径配置参数
- 💰 **新增**: Token使用统计功能，实时显示AI分析的token消耗
- 📅 **优化**: 日志系统按日期分类存储，扩展日志结构支持模型和token信息
- 🗂️ **统一**: 采用标准数据目录结构，简化用户配置体验
- 🚮 **移除**: 向后兼容代码，采用约定大于配置的设计理念

### v0.2.2 (2024-12-19)
- 🖼️ **新增**: 智能图片处理系统
  - **专门的图片处理功能**: 自动将截图转换为灰度图，减少颜色信息干扰
  - **智能缩放**: 支持自定义目标宽度，默认1440像素，保持原始宽高比
  - **高质量算法**: 使用Lanczos3算法进行缩放，确保图片质量
  - **参数化控制**: 添加 `--image-target-width` 和 `--image-grayscale` 命令行参数
  - **灵活配置**: 支持设置为0保持原图尺寸，支持环境变量配置
  - **性能优化**: 减少图片文件大小，提高AI分析效率和API响应速度
- 🔧 **改进**: 图片处理流程完全重构，支持参数化控制，提升整体分析性能

### v0.2.0 (2024-12-19)
- ✨ **新增**: 支持自定义 SiliconFlow API URL 配置
  - 添加 `--api-url` 命令行参数
  - 支持 `SILICONFLOW_API_URL` 环境变量
  - 保持向后兼容，默认使用官方API端点
- 🔧 **改进**: 增强配置灵活性，支持私有部署的API服务
- 📚 **文档**: 更新README文档，添加新功能使用说明

### v0.1.0 (2024-12-18)
- 🎉 **初始版本**: 基础屏幕时间监控功能
- 🤖 **AI分析**: 集成SiliconFlow多模态模型
- 🔗 **MCP服务**: 支持Model Context Protocol
- 📊 **系统上下文**: 自动收集系统信息
- 🛡️ **权限管理**: 自动检查和请求必要权限
- 🧪 **测试功能**: 支持使用新prompt重新分析现有截图

![](assets\6fdf331f-390c-4493-a29f-eebcd0e393af.png)

## ✨ 主要功能

- **🤖 AI 智能分析**: 使用多模态模型分析截图内容，理解用户活动
- **💰 Token 使用统计**: 实时显示AI分析的token消耗，包含输入、输出和总token数量
- **🖼️ 智能图片处理**: 自动灰度转换、智能缩放、高质量算法，优化AI分析效果
- **📊 丰富系统上下文**: 自动收集系统信息、进程状态、窗口信息、网络接口等
- **🗂️ 统一数据管理**: 采用标准目录结构，所有数据统一存储管理
- **📅 按日期分类日志**: 日志文件按天分别保存，便于数据管理和分析
- **🔗 MCP 服务支持**: 提供 Model Context Protocol 服务，支持远程控制
- **🛡️ 权限自动检查**: 启动时自动检查并引导用户授权必要权限
- **📝 完整活动日志**: 记录分析结果、系统状态、截图路径、模型信息和token使用
- **⚙️ 简化配置**: 约定大于配置，只需设置数据根目录即可
- **🌐 Web 服务**: 内置 SSE (Server-Sent Events) 服务器，支持实时数据推送
- **🔧 自定义API**: 支持配置自定义API端点，包括SiliconFlow和Ollama等本地模型
- **🧪 测试功能**: 支持使用新prompt重新分析现有截图，便于优化分析效果

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
# 基本使用（使用默认数据目录）
./target/release/screen_time --api-key your_api_key_here

# 自定义数据目录
./target/release/screen_time --api-key your_api_key_here --data-dir /path/to/your/data

# 使用自定义API URL
./target/release/screen_time --api-key your_api_key_here --api-url https://your-custom-endpoint.com/v1/chat/completions

# 使用环境变量
export SILICONFLOW_API_KEY=your_api_key_here
export SCREENTIME_DATA_DIR=/path/to/your/data
./target/release/screen_time

# 使用 Ollama 本地模型
./target/release/screen_time \
  --api-key ollama \
  --api-url "http://localhost:11434/v1/chat/completions" \
  --model "llava:7b" \
  --data-dir /path/to/ollama/data
```

#### 截图保留与删除

- 默认：分析完成后删除截图，控制台会显示 `已删除截图: <路径>`。
- 保留截图：
  ```bash
  ./target/release/screen_time --api-key your_api_key_here --keep-screenshots
  ```
- 使用环境变量：
  ```bash
  export KEEP_SCREENSHOTS=1
  ./target/release/screen_time --api-key your_api_key_here
  ```
- 特例：`--test-prompt` 模式会强制保留当次截图。

**数据目录结构**（自动创建）：
```
你的数据目录/
├── screenshots/              # 截图文件
├── logs/                    # 按日期分类的日志
│   ├── 2024-01-01.json
│   ├── 2024-01-02.json
│   └── ...
├── service_state.json       # 服务状态
└── service.sock            # 服务控制Socket
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

#### 3. 测试新Prompt模式

```bash
# 使用新prompt重新分析现有截图
./target/release/screen_time \
  --api-key your_api_key_here \
  --test-prompt "请详细描述这张截图中用户的工作状态和专注程度" \
  --test-log-path new_analysis_results.json
```

这个功能允许您：
- 使用新的prompt重新分析现有的截图
- 对比不同prompt的分析效果
- 优化AI分析的质量和准确性
- 保存测试结果到指定文件

#### 4. 图片处理配置示例

```bash
# 使用默认设置（宽度1440，灰度转换）
./target/release/screen_time --api-key your_key

# 自定义宽度，保持灰度转换
./target/release/screen_time --api-key your_key --image-target-width 1024

# 保持原图尺寸，启用灰度转换
./target/release/screen_time --api-key your_key --image-target-width 0

# 自定义宽度，禁用灰度转换（保持彩色）
./target/release/screen_time --api-key your_key --image-target-width 800 --no-image-grayscale

# 使用环境变量配置图片处理
export IMAGE_TARGET_WIDTH=1200
export IMAGE_GRAYSCALE=false
./target/release/screen_time --api-key your_key
```

**图片处理功能说明**:
- **灰度转换**: 默认启用，减少颜色信息干扰，提高AI分析准确性
- **智能缩放**: 默认宽度1440像素，保持原始宽高比，使用高质量Lanczos3算法
- **灵活配置**: 支持设置为0保持原图尺寸，支持完全禁用灰度转换
- **性能优化**: 减少图片文件大小，提高传输和分析效率

#### 5. Ollama 本地模型支持

ScreenTime 支持使用 Ollama 本地大语言模型进行图片分析，无需联网即可使用。

**前置要求**:
1. 安装并启动 Ollama: https://ollama.ai/
2. 拉取支持视觉的模型，如 `llava:7b`:
   ```bash
   ollama pull llava:7b
   ```

**使用示例**:
```bash
# 基本使用
./target/release/screen_time \
  --api-key ollama \
  --api-url "http://localhost:11434/v1/chat/completions" \
  --model "llava:7b"

# 使用环境变量
export SILICONFLOW_API_KEY=ollama
export SILICONFLOW_API_URL=http://localhost:11434/v1/chat/completions
export SILICONFLOW_MODEL=llava:7b
./target/release/screen_time

# 结合图片处理参数
./target/release/screen_time \
  --api-key ollama \
  --api-url "http://localhost:11434/v1/chat/completions" \
  --model "llava:7b" \
  --image-target-width 1024 \
  --no-image-grayscale
```

**支持的模型**:
- `llava:7b` - 轻量级视觉语言模型
- `llava:13b` - 更强大的视觉语言模型
- `llava:34b` - 最高性能的视觉语言模型
- 其他支持视觉的 Ollama 模型

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
| `--api-url <API_URL>` | `SILICONFLOW_API_URL` | `https://api.siliconflow.cn/v1/chat/completions` | SiliconFlow API URL |
| `-m, --model <MODEL>` | `SILICONFLOW_MODEL` | `THUDM/GLM-4.1V-9B-Thinking` | 用于分析的模型 |
| `-p, --prompt <PROMPT>` | `SCREEN_ANALYSIS_PROMPT` | `请描述这张截图中用户正在使用什么软件，在做什么...` | 用于分析的提示 |
| `-i, --interval <INTERVAL>` | `SCREENSHOT_INTERVAL_SECONDS` | `60` | 截图间隔（秒） |
| `--data-dir <DATA_DIR>` | `SCREENTIME_DATA_DIR` | 系统默认目录* | 数据存储根目录 |
| `--image-target-width <WIDTH>` | `IMAGE_TARGET_WIDTH` | `1440` | 图片处理的目标宽度，设置为0保持原图尺寸 |
| `--image-grayscale` | `IMAGE_GRAYSCALE` | `true` | 是否将图片转换为灰度图 |
| `--mcp` | - | `false` | 启动 MCP 服务器模式 |
| `--test-prompt <TEST_PROMPT>` | - | - | 测试新的prompt，使用现有的截图和上下文重新计算 |
| `--test-log-path <TEST_LOG_PATH>` | `TEST_LOG_PATH` | `test_log.json` | 测试结果保存路径 |
| `--keep-screenshots` | `KEEP_SCREENSHOTS` | `false` | 分析完成后保留截图文件（默认删除） |

**系统默认目录**:
- macOS: `~/Library/Application Support/ScreenTime/`
- Linux: `~/.local/share/screentime/`  
- Windows: `%APPDATA%/ScreenTime/`

### 环境变量配置示例

```bash
# SiliconFlow API 配置
export SILICONFLOW_API_KEY=your_api_key_here
export SILICONFLOW_API_URL=https://api.siliconflow.cn/v1/chat/completions
export SILICONFLOW_MODEL=Qwen/Qwen2-VL-7B-Instruct

# Ollama 本地模型配置
export SILICONFLOW_API_KEY=ollama
export SILICONFLOW_API_URL=http://localhost:11434/v1/chat/completions
export SILICONFLOW_MODEL=llava:7b

# 其他配置
export SCREEN_ANALYSIS_PROMPT="请描述这张截图中用户正在使用什么软件，在做什么，并进行分类，严格按照格式输出结果：【类型】【软件】【主要工作摘要】。"
export SCREENSHOT_INTERVAL_SECONDS=60
export SCREENTIME_DATA_DIR=/path/to/your/data
export IMAGE_TARGET_WIDTH=1440
export IMAGE_GRAYSCALE=true
export TEST_LOG_PATH=test_log.json
export KEEP_SCREENSHOTS=1
```

### 🔧 自定义API端点

ScreenTime 支持配置自定义的 API 端点，适用于以下场景：

- **私有部署**: 如果您有自己的 SiliconFlow 服务实例
- **企业环境**: 公司内部的API服务
- **测试环境**: 开发或测试用的API端点
- **本地模型**: 使用 Ollama 等本地大语言模型服务

**使用示例**:
```bash
# 使用私有部署的API
./target/release/screen_time \
  --api-key your_private_key \
  --api-url https://your-company.com/siliconflow/v1/chat/completions

# 使用测试环境API
export SILICONFLOW_API_URL=https://test-api.example.com/v1/chat/completions
./target/release/screen_time --api-key test_key

# 使用 Ollama 本地模型
./target/release/screen_time \
  --api-key ollama \
  --api-url "http://localhost:11434/v1/chat/completions" \
  --model "llava:7b"
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
│   ├── config.rs            # 配置解析（简化版）
│   ├── screenshot.rs        # 屏幕截图功能
│   ├── siliconflow.rs       # SiliconFlow API 调用（包含token统计）
│   ├── logger.rs            # 日志记录功能（按日期分类）
│   ├── models.rs            # 数据模型定义（扩展版）
│   ├── capture.rs           # 截屏循环控制
│   ├── context.rs           # 系统上下文收集
│   ├── permissions.rs       # 权限检查和请求
│   ├── mcp_service.rs       # MCP 服务实现
│   ├── service_state.rs     # 服务状态管理
│   ├── standalone_service.rs # 独立服务实现
│   └── test_prompt.rs       # 测试prompt功能

├── Cargo.toml              # 项目配置和依赖
├── CHANGELOG.md            # 更新日志
└── README.md               # 项目文档
```

**运行时数据目录结构**：
```
数据根目录/
├── screenshots/             # 截图文件（自动创建）
├── logs/                   # 按日期分类的日志（自动创建）
│   ├── 2024-01-01.json
│   └── ...
├── service_state.json      # 服务状态文件
└── service.sock           # 服务控制Socket
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

活动日志以 JSON 格式保存，按日期分类存储在 `logs/` 目录下：

**日志目录结构**:
```
logs/
├── 2024-01-01.json    # 2024年1月1日的所有记录
├── 2024-01-02.json    # 2024年1月2日的所有记录
└── ...
```

**日志条目格式**:
```json
{
  "timestamp": "2024-01-01T12:00:00+08:00",
  "description": "【工作】【VSCode】【正在编辑Rust代码，进行项目开发】",
  "model": "THUDM/GLM-4.1V-9B-Thinking",
  "token_usage": {
    "prompt_tokens": 1024,
    "completion_tokens": 156,
    "total_tokens": 1180
  },
  "context": {
    "username": "用户名",
    "hostname": "主机名",
    "os_name": "macOS",
    "uptime_secs": 3600,
    "total_memory_mb": 16384,
    "used_memory_mb": 8192,
    "processes_top": [...],
    "active_window": {
      "app_name": "Visual Studio Code",
      "window_title": "main.rs - ScreenTime - VSCode"
    },
    "interfaces": [...]
  },
  "screenshot_path": "screenshots/screenshot_20240101_120000.png"
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
7. **自定义API**: 使用自定义API端点时，请确保端点支持与官方API相同的接口格式

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目！

## 📄 许可证

本项目采用 MIT 许可证。