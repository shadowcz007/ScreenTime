use base64::{Engine as _, engine::general_purpose};
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
struct SiliconFlowRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: Vec<Content>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Content {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<ImageUrl>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ImageUrl {
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SiliconFlowResponse {
    choices: Option<Vec<Choice>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Choice {
    message: MessageResponse,
}

#[derive(Serialize, Deserialize, Debug)]
struct MessageResponse {
    content: String,
}

pub async fn analyze_screenshot_with_prompt(
    api_key: &str,
    model: &str,
    image_path: &str,
    prompt: &str,
    extra_context: Option<&str>, // 系统上下文
    activity_history: Option<&str>, // 新增：用户活动历史
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = "https://api.siliconflow.cn/v1/chat/completions";
    
    // 读取图片文件并编码为base64
    let image_data = tokio::fs::read(image_path).await?;
    let base64_image = general_purpose::STANDARD.encode(&image_data);
    let image_url = format!("data:image/png;base64,{}", base64_image);
    
    // 构建请求体
    let mut contents = vec![
        Content {
            content_type: "text".to_string(),
            text: Some(prompt.to_string()),
            image_url: None,
        }
    ];

    if let Some(ctx) = extra_context {
        contents.push(Content {
            content_type: "text".to_string(),
            text: Some(format!("以下是当前系统上下文，请结合截图一起分析：\n{}", ctx)),
            image_url: None,
        });
    }

    if let Some(history) = activity_history {
        contents.push(Content {
            content_type: "text".to_string(),
            text: Some(format!("{}请参考用户的操作历史，分析当前截图时要考虑操作的连续性和上下文关系。", history)),
            image_url: None,
        });
    }

    contents.push(Content {
        content_type: "image_url".to_string(),
        text: None,
        image_url: Some(ImageUrl { url: image_url }),
    });

    let request_body = SiliconFlowRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: contents,
        }],
    };
    
    // 发送请求
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;
    
    // 检查响应状态
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(format!("API请求失败: {} - {}", status, error_text).into());
    }
    
    let response_text = response.text().await?;
    
    // 解析响应
    let siliconflow_response: Result<SiliconFlowResponse, _> = serde_json::from_str(&response_text);
    
    match siliconflow_response {
        Ok(response) => {
            // 提取描述文本
            if let Some(choices) = response.choices {
                if let Some(choice) = choices.first() {
                    return Ok(choice.message.content.clone());
                }
            }
            Ok("无法分析截图内容".to_string())
        },
        Err(e) => {
            eprintln!("解析API响应时出错: {}", e);
            eprintln!("原始响应: {}", response_text);
            Err("解析API响应失败".into())
        }
    }
}