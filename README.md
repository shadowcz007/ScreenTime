# OpenRecall

一个功能强大的屏幕时间监控工具，使用 Rust 编写，集成了 AI 图像分析和 MCP (Model Context Protocol) 服务功能。它可以定期截取屏幕截图，调用可配置的本地/远程多模态模型分析用户活动，并提供丰富的系统上下文信息。

## 📋 更新日志

### v1.0 (2025-02-21)
- 📤 **OpenClaw 上报**: 支持将 OpenRecall 计算结果按可配置间隔（默认 30 分钟）发送到 OpenClaw Gateway 的 `/hooks/agent`，由智能体做总结；需同时配置 `--openclaw-url` 与 `--openclaw-token` 才启用。

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
  - 支持 `OPENRECALL_API_URL` 环境变量
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
- **🔧 自定义API**: 支持配置自定义API端点，包括本地模型（Ollama 等）和远程服务
- **📤 OpenClaw 上报**: 可选将过去 N 分钟的 OpenRecall 摘要发送到 OpenClaw Gateway，便于与主会话联动
- **🧪 测试功能**: 支持使用新prompt重新分析现有截图，便于优化分析效果

## 🚀 快速开始

### 安装

1. 确保你已经安装了 Rust 和 Cargo
2. 克隆此仓库：
   ```bash
   git clone <repository-url>
   cd OpenRecall
   ```
3. 构建项目：
   ```bash
   cargo build --release
   ```

### 基本使用

#### 1. 标准监控模式

```bash
# 基本使用（使用默认数据目录）
./target/release/openrecall --api-key your_api_key_here

# 自定义数据目录
./target/release/openrecall --api-key your_api_key_here --data-dir /path/to/your/data

# 使用自定义API URL
./target/release/openrecall --api-key your_api_key_here --api-url https://your-custom-endpoint.com/v1/chat/completions

# 使用环境变量
export OPENRECALL_API_KEY=your_api_key_here
export SCREENTIME_DATA_DIR=/path/to/your/data
./target/release/openrecall

# 使用 Ollama 本地模型
./target/release/openrecall \
  --api-key ollama \
  --api-url "http://localhost:11434/v1/chat/completions" \
  --model "llava:7b" \
  --data-dir /path/to/ollama/data
```

#### 1.1 零传参启动（推荐）

OpenRecall 启动时会自动读取项目根目录下的 `.env`，因此可实现“无命令行参数启动”。
运行中修改 `.env` 也会自动重载（无需重启），截屏间隔、剪贴板轮询与 AI 过滤相关参数会按新配置生效。

示例 `.env`：

```bash
OPENRECALL_API_KEY=default
OPENRECALL_API_URL=http://127.0.0.1:1234/v1/chat/completions
OPENRECALL_MODEL=default
SCREENSHOT_INTERVAL_SECONDS=60
START_CAPTURE_ON_LAUNCH=true
CLIPBOARD_ENABLED=true
CLIPBOARD_INTERVAL_MS=500
CLIPBOARD_AUTO_SAVE=false
```

启动命令：

```bash
./target/release/openrecall
```

#### 截图保留与删除

- 默认：分析完成后删除截图，控制台会显示 `已删除截图: <路径>`。
- 保留截图：
  ```bash
  ./target/release/openrecall --api-key your_api_key_here --keep-screenshots
  ```
- 使用环境变量：
  ```bash
  export KEEP_SCREENSHOTS=1
  ./target/release/openrecall --api-key your_api_key_here
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
./target/release/openrecall --mcp --api-key your_api_key_here
```

MCP 服务器将在 `127.0.0.1:8000` 启动，提供以下工具：
- `monitor`: 控制监控状态 (start/stop/status)
- `read_logs`: 读取活动日志
- `clipboard_status`: 查询剪贴板监听状态
- `clipboard_list`: 查看最近剪贴板记录
- `clipboard_save`: 按 id 手动保存剪贴板记录为 Markdown
- `clipboard_auto_save`: 开关自动保存

#### 3. 测试新Prompt模式

```bash
# 使用新prompt重新分析现有截图
./target/release/openrecall \
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
./target/release/openrecall --api-key your_key

# 自定义宽度，保持灰度转换
./target/release/openrecall --api-key your_key --image-target-width 1024

# 保持原图尺寸，启用灰度转换
./target/release/openrecall --api-key your_key --image-target-width 0

# 自定义宽度，禁用灰度转换（保持彩色）
./target/release/openrecall --api-key your_key --image-target-width 800 --no-image-grayscale

# 使用环境变量配置图片处理
export IMAGE_TARGET_WIDTH=1200
export IMAGE_GRAYSCALE=false
./target/release/openrecall --api-key your_key
```

**图片处理功能说明**:
- **灰度转换**: 默认启用，减少颜色信息干扰，提高AI分析准确性
- **智能缩放**: 默认宽度1440像素，保持原始宽高比，使用高质量Lanczos3算法
- **灵活配置**: 支持设置为0保持原图尺寸，支持完全禁用灰度转换
- **性能优化**: 减少图片文件大小，提高传输和分析效率

#### 5. Ollama 本地模型支持

OpenRecall 支持使用 Ollama 本地大语言模型进行图片分析，无需联网即可使用。

**前置要求**:
1. 安装并启动 Ollama: https://ollama.ai/
2. 拉取支持视觉的模型，如 `llava:7b`:
   ```bash
   ollama pull llava:7b
   ```

**使用示例**:
```bash
# 基本使用
./target/release/openrecall \
  --api-key ollama \
  --api-url "http://localhost:11434/v1/chat/completions" \
  --model "llava:7b"

# 使用环境变量
export OPENRECALL_API_KEY=ollama
export OPENRECALL_API_URL=http://localhost:11434/v1/chat/completions
export OPENRECALL_MODEL=llava:7b
./target/release/openrecall

# 结合图片处理参数
./target/release/openrecall \
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

#### 6. OpenClaw 上报

将 OpenRecall 的计算结果（过去 N 分钟的本机活动摘要）定期提交到 [OpenClaw](https://docs.openclaw.ai/) Gateway 的 **/hooks/agent**，由智能体做总结并可在主会话/渠道中看到。**仅当同时提供 `--openclaw-url` 和 `--openclaw-token` 时才会启用**，默认每 30 分钟提交一次。

- **URL**：`--openclaw-url` 需填写**完整的 agent webhook 地址**（含路径，如 `http://host:port/hooks/agent`）。
- **Token**：`--openclaw-token` 必须与 OpenClaw Gateway 配置中的 **`hooks.token`** 完全一致（字符、首尾空格均需一致），否则会返回 401 Unauthorized。建议从 Gateway 配置中直接复制粘贴。

```bash
# 启用 OpenClaw agent 上报（默认每 30 分钟）
./target/release/openrecall \
  --api-key your_api_key_here \
  --openclaw-url http://127.0.0.1:18789/hooks/agent \
  --openclaw-token YOUR_WEBHOOK_TOKEN

# 使用环境变量
export OPENCLAW_URL=http://127.0.0.1:18789/hooks/agent
export OPENCLAW_TOKEN=YOUR_WEBHOOK_TOKEN
./target/release/openrecall --api-key your_api_key_here

# 自定义上报间隔（例如每 15 分钟）
./target/release/openrecall \
  --api-key your_api_key_here \
  --openclaw-url http://127.0.0.1:18789/hooks/agent \
  --openclaw-token YOUR_WEBHOOK_TOKEN \
  --openclaw-report-interval-minutes 15
```

**测试 OpenClaw agent 连接**（不依赖 OpenRecall 运行）：

```bash
curl -X POST http://127.0.0.1:18789/hooks/agent \
  -H 'Authorization: Bearer YOUR_WEBHOOK_TOKEN' \
  -H 'Content-Type: application/json' \
  -d '{"message":"用户电脑设备在过去30分钟内的活动摘要（共0条）：测试。请总结。","name":"OpenRecall","wakeMode":"now","deliver":true}'
```

成功时返回 **202 Accepted**（异步已接受），智能体会在后台处理并总结，主会话/渠道中可见。

## 🔐 权限要求

OpenRecall 需要以下系统权限才能正常工作：

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
| `-a, --api-key <API_KEY>` | `OPENRECALL_API_KEY` | `default` | API 密钥 |
| `--api-url <API_URL>` | `OPENRECALL_API_URL` | `http://127.0.0.1:1234/v1/chat/completions` | API URL |
| `-m, --model <MODEL>` | `OPENRECALL_MODEL` | `default` | 用于分析的模型 |
| `-p, --prompt <PROMPT>` | `SCREEN_ANALYSIS_PROMPT` | `请描述这张截图中用户正在使用什么软件，在做什么...` | 用于分析的提示 |
| `-i, --interval <INTERVAL>` | `SCREENSHOT_INTERVAL_SECONDS` | `60` | 截图间隔（秒） |
| `--start-capture-on-launch` | `START_CAPTURE_ON_LAUNCH` | `false` | 启动后强制开启截屏服务（忽略上次停止状态） |
| `--installed-apps-enabled` | `INSTALLED_APPS_ENABLED` | `false` | 在上下文中注入已安装软件清单（macOS） |
| `--installed-apps-refresh-minutes <MINUTES>` | `INSTALLED_APPS_REFRESH_MINUTES` | `30` | 已安装软件清单缓存刷新间隔（分钟） |
| `--installed-apps-max-items <N>` | `INSTALLED_APPS_MAX_ITEMS` | `300` | 注入上下文的已安装软件上限 |
| `--installed-apps-include-user-dir` | `INSTALLED_APPS_INCLUDE_USER_DIR` | `true` | 是否扫描 `~/Applications` |
| `--input-context-enabled` | `INPUT_CONTEXT_ENABLED` | `false` | 启用键盘/鼠标输入上下文采集 |
| `--input-context-window-seconds <SECONDS>` | `INPUT_CONTEXT_WINDOW_SECONDS` | `60` | 输入上下文统计窗口（秒） |
| `--input-context-max-keystrokes <N>` | `INPUT_CONTEXT_MAX_KEYSTROKES` | `120` | 上下文中包含的最大按键数量 |
| `--input-context-include-raw-keys` | `INPUT_CONTEXT_INCLUDE_RAW_KEYS` | `true` | 是否包含原始按键键名 |
| `--data-dir <DATA_DIR>` | `SCREENTIME_DATA_DIR` | 系统默认目录* | 数据存储根目录 |
| `--image-target-width <WIDTH>` | `IMAGE_TARGET_WIDTH` | `1440` | 图片处理的目标宽度，设置为0保持原图尺寸 |
| `--image-grayscale` | `IMAGE_GRAYSCALE` | `true` | 是否将图片转换为灰度图 |
| `--mcp` | - | `false` | 启动 MCP 服务器模式 |
| `--test-prompt <TEST_PROMPT>` | - | - | 测试新的prompt，使用现有的截图和上下文重新计算 |
| `--test-log-path <TEST_LOG_PATH>` | `TEST_LOG_PATH` | `test_log.json` | 测试结果保存路径 |
| `--keep-screenshots` | `KEEP_SCREENSHOTS` | `false` | 分析完成后保留截图文件（默认删除） |
| `--openclaw-url <URL>` | `OPENCLAW_URL` | - | OpenClaw agent webhook 完整 URL（如 `http://host:port/hooks/agent`）；与 `--openclaw-token` 同时设置时启用上报 |
| `--openclaw-token <TOKEN>` | `OPENCLAW_TOKEN` | - | OpenClaw webhook 令牌 |
| `--openclaw-report-interval-minutes <MINUTES>` | `OPENCLAW_REPORT_INTERVAL_MINUTES` | `30` | 向 OpenClaw 上报的间隔（分钟） |
| `--clipboard-enabled` | `CLIPBOARD_ENABLED` | `false` | 启用系统剪贴板监听 |
| `--clipboard-interval-ms <MS>` | `CLIPBOARD_INTERVAL_MS` | `500` | 剪贴板轮询间隔（毫秒） |
| `--clipboard-auto-save` | `CLIPBOARD_AUTO_SAVE` | `false` | 自动将新剪贴板内容保存为 Markdown |
| `--clipboard-ai-filter-enabled` | `CLIPBOARD_AI_FILTER_ENABLED` | `false` | 启用剪贴板 AI 过滤，仅 save=true 才保存 |
| `--clipboard-ai-filter-prompt <PROMPT>` | `CLIPBOARD_AI_FILTER_PROMPT` | 内置默认提示词 | 剪贴板 AI 过滤提示词 |
| `--clipboard-ai-min-chars <N>` | `CLIPBOARD_AI_MIN_CHARS` | `20` | 触发 AI 判定的最小字符数 |
| `--clipboard-ai-timeout-seconds <SECONDS>` | `CLIPBOARD_AI_TIMEOUT_SECONDS` | `10` | 剪贴板 AI 判定超时（秒） |
| `--clipboard-ai-save-on-error` | `CLIPBOARD_AI_SAVE_ON_ERROR` | `false` | AI 判定失败时是否仍保存 |
| `--clipboard-target-dir <DIR>` | `CLIPBOARD_TARGET_DIR` | `<data_dir>/clipboards/exports` | 剪贴板 Markdown 导出目录 |
| `--clipboard-max-bytes <BYTES>` | `CLIPBOARD_MAX_BYTES` | `200000` | 单条剪贴板内容最大字节数 |

**系统默认目录**:
- macOS: `~/Library/Application Support/OpenRecall/`
- Linux: `~/.local/share/screentime/`  
- Windows: `%APPDATA%/OpenRecall/`

### 环境变量配置示例

```bash
# API 配置
export OPENRECALL_API_KEY=your_api_key_here
export OPENRECALL_API_URL=http://127.0.0.1:1234/v1/chat/completions
export OPENRECALL_MODEL=Qwen/Qwen2-VL-7B-Instruct

# Ollama 本地模型配置
export OPENRECALL_API_KEY=ollama
export OPENRECALL_API_URL=http://localhost:11434/v1/chat/completions
export OPENRECALL_MODEL=llava:7b

# 其他配置
export SCREEN_ANALYSIS_PROMPT="请描述这张截图中用户正在使用什么软件，在做什么，并进行分类，严格按照格式输出结果：【类型】【软件】【主要工作摘要】。"
export SCREENSHOT_INTERVAL_SECONDS=60
export START_CAPTURE_ON_LAUNCH=false
export INSTALLED_APPS_ENABLED=true
export INSTALLED_APPS_REFRESH_MINUTES=30
export INSTALLED_APPS_MAX_ITEMS=300
export INSTALLED_APPS_INCLUDE_USER_DIR=true
export INPUT_CONTEXT_ENABLED=false
export INPUT_CONTEXT_WINDOW_SECONDS=60
export INPUT_CONTEXT_MAX_KEYSTROKES=120
export INPUT_CONTEXT_INCLUDE_RAW_KEYS=true
export SCREENTIME_DATA_DIR=/path/to/your/data
export IMAGE_TARGET_WIDTH=1440
export IMAGE_GRAYSCALE=true
export TEST_LOG_PATH=test_log.json
export KEEP_SCREENSHOTS=1

# OpenClaw agent 上报（可选，URL 为完整 /hooks/agent 地址）
export OPENCLAW_URL=http://127.0.0.1:18789/hooks/agent
export OPENCLAW_TOKEN=your_webhook_token
export OPENCLAW_REPORT_INTERVAL_MINUTES=30

# 剪贴板监听（可选）
export CLIPBOARD_ENABLED=true
export CLIPBOARD_INTERVAL_MS=500
export CLIPBOARD_AUTO_SAVE=false
export CLIPBOARD_AI_FILTER_ENABLED=true
export CLIPBOARD_AI_FILTER_PROMPT='你是剪贴板内容过滤器。任务：判断该内容是否应保存，用于后续 web-search 研究。仅当内容包含可研究 URL、研究主题或检索关键词时 save=true。如果是验证码、密码、token、私钥、无意义噪音则 save=false。只输出一行JSON：{"save":true|false,"reason":"简短原因","category":"url|topic|keywords|other"}。'
export CLIPBOARD_AI_MIN_CHARS=20
export CLIPBOARD_AI_TIMEOUT_SECONDS=10
export CLIPBOARD_AI_SAVE_ON_ERROR=false
export CLIPBOARD_TARGET_DIR=/path/to/clipboard/exports
export CLIPBOARD_MAX_BYTES=200000
```

### 🔧 自定义API端点

OpenRecall 支持配置自定义的 API 端点，适用于以下场景：

- **私有部署**: 如果您有自己的模型网关或推理服务实例
- **企业环境**: 公司内部的API服务
- **测试环境**: 开发或测试用的API端点
- **本地模型**: 使用 Ollama 等本地大语言模型服务

**使用示例**:
```bash
# 使用私有部署的API
./target/release/openrecall \
  --api-key your_private_key \
  --api-url https://your-company.com/v1/chat/completions

# 使用测试环境API
export OPENRECALL_API_URL=https://test-api.example.com/v1/chat/completions
./target/release/openrecall --api-key test_key

# 使用 Ollama 本地模型
./target/release/openrecall \
  --api-key ollama \
  --api-url "http://localhost:11434/v1/chat/completions" \
  --model "llava:7b"
```

## 📊 系统上下文收集

OpenRecall 会自动收集以下系统信息，为 AI 分析提供更丰富的上下文：

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

## 📋 剪贴板监听与去重

- **去重规则**: 对文本内容做标准化（统一换行并去除首尾空白）后计算哈希；重复内容仅更新出现次数与最后出现时间，不重复新增。
- **手动保存**: 通过 MCP `clipboard_list` 先查看记录 id，再调用 `clipboard_save` 保存到 Markdown。
- **自动保存**: 开启 `--clipboard-auto-save` 后，新的（非重复）剪贴板内容会自动导出到 `clipboards/exports/`。
- **AI 过滤保存**: 开启 `CLIPBOARD_AI_FILTER_ENABLED=true` 后，先用 `CLIPBOARD_AI_FILTER_PROMPT` 判定，只有返回 `{"save":true}` 的内容才会保存。
- **运行日志**: 剪贴板获取、去重命中、AI 判定结果与保存结果会同时输出到终端，并写入 `clipboards/events.log`。
- **文件命名**: `YYYYMMDD_HHMMSS_<short_hash>_<slug>.md`（如 `20260428_123015_a1b2c3d4_vscode_rust_refactor.md`），若重名会自动追加 `_1`、`_2`...

## 📁 项目结构

```
OpenRecall/
├── src/
│   ├── main.rs              # 程序入口点
│   ├── config.rs            # 配置解析（简化版）
│   ├── screenshot.rs        # 屏幕截图功能
│   ├── siliconflow.rs       # 模型 API 调用（包含 token 统计）
│   ├── logger.rs            # 日志记录功能（按日期分类）
│   ├── models.rs            # 数据模型定义（扩展版）
│   ├── capture.rs           # 截屏循环控制
│   ├── context.rs           # 系统上下文收集
│   ├── permissions.rs       # 权限检查和请求
│   ├── mcp_service.rs       # MCP 服务实现
│   ├── service_state.rs     # 服务状态管理
│   ├── standalone_service.rs # 独立服务实现
│   ├── openclaw.rs          # OpenClaw /hooks/agent 上报与智能体总结
│   ├── clipboard.rs         # 剪贴板监听、去重与 Markdown 导出
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
├── logs_md/                # 按日期分类的可读 Markdown 日志（自动创建）
│   ├── 2024-01-01.md
│   └── ...
├── clipboards/             # 剪贴板数据（自动创建）
│   ├── history.json        # 剪贴板历史记录
│   ├── index.json          # 去重索引
│   └── exports/            # Markdown 导出目录
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
  "model": "default",
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
      "window_title": "main.rs - OpenRecall - VSCode"
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

### 说明
当前 MCP 工具以 `monitor`、`read_logs` 以及剪贴板相关工具为主（`clipboard_status` / `clipboard_list` / `clipboard_save` / `clipboard_auto_save`）。

## ⚠️ 注意事项

1. **权限要求**: 请确保你的系统允许屏幕录制权限
2. **API 费用**: 当你使用计费型远程模型服务时，API 调用可能产生费用，请关注 token 消耗
3. **隐私保护**: 截图和分析结果会保存在本地，请注意保护个人隐私
4. **系统兼容性**: 支持 macOS 和 Windows 系统，Linux 系统的支持正在开发中
5. **网络连接**: 需要稳定的网络连接以调用 AI 分析服务
6. **管理员权限**: Windows 系统可能需要管理员权限来获取完整的窗口信息
7. **自定义API**: 使用自定义API端点时，请确保端点支持与官方API相同的接口格式
8. **OpenClaw 安全**: 使用 OpenClaw 上报时，建议将 Gateway 的 webhook 端点置于 loopback、tailnet 或受信任反向代理之后，并使用专用 webhook 令牌

## 🔎 定位与排查

当分析结果异常（如误识别软件、剪贴板未保存、AI 过滤不符合预期）时，建议按下列顺序定位：

1. **看终端实时输出**：确认截屏循环是否在运行、API 是否请求成功、是否存在重试。
2. **看活动 JSON 日志**：`logs/YYYY-MM-DD.json`，用于程序化核对（时间、描述、模型、token）。
3. **看可读 Markdown 日志**：`logs_md/YYYY-MM-DD.md`，用于人工快速回放每次分析结果。
4. **看剪贴板事件日志**：`clipboards/events.log`，重点关注：
   - `clipboard_fetch`（是否采集到内容）
   - `clipboard_ai`（save/category/reason）
   - `clipboard_save`（是否 `skip_by_ai` 或已保存）
5. **核对 `.env` 与热重载**：修改 `.env` 后观察终端是否出现重载提示（间隔变化、策略变化）。

常见问题建议：
- **误识别未安装软件**：开启 `INSTALLED_APPS_ENABLED=true`，并优化 `SCREEN_ANALYSIS_PROMPT`（要求无证据时输出“未知软件”）。
- **关键词被误跳过**：降低 `CLIPBOARD_AI_MIN_CHARS`（如 6），并强化 `CLIPBOARD_AI_FILTER_PROMPT` 对关键词组合的保留规则。
- **AI 过滤日志不清晰**：检查 `skip_by_ai` 行是否含 `reason/category`，若无通常是旧进程未重启。
- **输入状态未生效**：启用 `INPUT_CONTEXT_ENABLED=true` 后，确保系统已授予输入监控相关权限。

### 3 分钟速查表

- **分析没跑**：先用 `monitor status` 确认服务在 `Running`，再看终端有无 `尝试分析截图`。
- **软件识别不准**：检查上下文是否出现“已安装软件清单(部分)”，无证据时应输出“未知软件”。
- **剪贴板没落盘**：先看 `clipboards/events.log` 里的 `clipboard_ai` 与 `clipboard_save`。
- **配置改了没生效**：确认改的是项目根目录 `.env`，并观察热重载提示。
- **阅读日志优先级**：先看 `logs_md/YYYY-MM-DD.md`，再看 `logs/YYYY-MM-DD.json` 做精确核对。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目！

## 📄 许可证

本项目采用 MIT 许可证。