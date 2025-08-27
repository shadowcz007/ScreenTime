# ScreenTime

这是一个用 Rust 编写的屏幕时间监控工具，它可以定期截取屏幕截图，并使用 SiliconFlow 提供的 Qwen 视觉模型分析用户在特定时间点的活动。

## 功能

- 定期间隔截屏
- 使用多模态模型分析截图内容
- 记录活动日志

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

## 配置

在运行程序之前，你需要设置 SiliconFlow API 密钥作为环境变量：

```bash
export SILICONFLOW_API_KEY=your_api_key_here
```

## 使用

运行程序：

```bash
export SILICONFLOW_API_KEY=your_siliconflow_api_key_here
cargo run
```

或者运行已构建的版本：

```bash
export SILICONFLOW_API_KEY=your_siliconflow_api_key_here
./target/release/screen_time
```

## 项目结构

- `src/main.rs`: 程序入口点
- `src/screenshot.rs`: 屏幕截图功能
- `src/siliconflow.rs`: SiliconFlow API 调用功能
- `src/logger.rs`: 日志记录功能
- `src/models.rs`: 数据模型
- `src/capture.rs`: 截屏循环控制

## 依赖

- `tokio`: 异步运行时
- `image`: 图像处理
- `screenshot`: 屏幕截图
- `reqwest`: HTTP 客户端
- `serde`: 序列化/反序列化
- `chrono`: 日期时间处理

## 注意事项

- 请确保你的系统允许屏幕录制权限。
- SiliconFlow API 调用可能会产生费用，请根据你的使用情况进行监控。