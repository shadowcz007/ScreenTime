use fastvlm::{FastVLMClient, FastVLMConfig};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::models::{TokenUsage, AnalysisResult};

/// FastVLM本地模型服务
pub struct FastVLMService {
    client: Arc<Mutex<Option<FastVLMClient>>>,
    model_dir: PathBuf,
    config: FastVLMConfig,
}

impl FastVLMService {
    /// 创建新的FastVLM服务实例
    pub fn new(model_dir: PathBuf) -> Self {
        let config = FastVLMConfig {
            max_response_length: 200, // 设置较长的响应长度
            default_prompt: "请描述这张截图中用户正在使用什么软件，在做什么，并进行分类，严格按照格式输出结果：【类型】【软件】【主要工作摘要】。".to_string(),
        };

        Self {
            client: Arc::new(Mutex::new(None)),
            model_dir,
            config,
        }
    }

    /// 初始化FastVLM模型
    pub async fn initialize(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut client_guard = self.client.lock().await;
        
        if client_guard.is_some() {
            return Ok(()); // 已初始化
        }

        println!("🔧 正在初始化FastVLM本地模型...");
        println!("📁 模型目录: {:?}", self.model_dir);

        // 检查模型目录是否存在
        if !self.model_dir.exists() || !self.model_dir.is_dir() {
            return Err(format!("FastVLM模型目录不存在或不是目录: {:?}", self.model_dir).into());
        }

        // 创建FastVLM客户端
        let mut client = FastVLMClient::new();
        
        // 使用模型目录路径初始化
        let model_path_str = self.model_dir.to_string_lossy();
        client.initialize(Some(&model_path_str), self.config.clone()).await
            .map_err(|e| format!("初始化FastVLM模型失败: {}", e))?;

        *client_guard = Some(client);
        println!("✅ FastVLM本地模型初始化成功");
        
        Ok(())
    }

    /// 分析截图
    pub async fn analyze_screenshot_with_prompt(
        &self,
        image_path: &str,
        prompt: &str,
        extra_context: Option<&str>, // 移除下划线，启用系统上下文
        activity_history: Option<&str>, // 移除下划线，启用活动历史
    ) -> Result<AnalysisResult, Box<dyn Error + Send + Sync>> {
        let mut client_guard = self.client.lock().await;
        
        let client = client_guard.as_mut()
            .ok_or("FastVLM模型未初始化，请先调用initialize()")?;

        println!("📸 FastVLM正在分析图片: {}", image_path);
        
        let start_time = std::time::Instant::now();
        
        // 构建完整的prompt，与硅基流动保持一致
        let mut full_prompt = prompt.to_string();
        
        if let Some(ctx) = extra_context {
            full_prompt.push_str(&format!("\n\n以下是当前系统上下文，请结合截图一起分析：\n{}", ctx));
        }
        
        if let Some(history) = activity_history {
            full_prompt.push_str(&format!("\n\n{}以下是用户最近的活动历史，仅供参考。请独立分析当前截图，当前行为可能与历史活动相关，也可能完全无关。", history));
        }
        
        // 使用完整的prompt分析图片
        let result = client.analyze_image_file(image_path, Some(full_prompt))
            .await
            .map_err(|e| format!("FastVLM分析图片失败: {}", e))?;

        let processing_time = start_time.elapsed();
        println!("✅ FastVLM分析完成，耗时: {:.2}秒", processing_time.as_secs_f32());

        Ok(AnalysisResult {
            description: result.text,
            token_usage: None,
        })
    }
}

impl Drop for FastVLMService {
    fn drop(&mut self) {
        // 注意: 这里不能使用async，但我们会在其他地方进行清理
        println!("🔄 FastVLMService实例被释放");
    }
}

/// 创建FastVLM服务实例并初始化
pub async fn create_fastvlm_service(model_dir: PathBuf) -> Result<FastVLMService, Box<dyn Error + Send + Sync>> {
    let service = FastVLMService::new(model_dir);
    service.initialize().await?;
    Ok(service)
}
