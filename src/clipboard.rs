use crate::config::Config;
use crate::models::{ClipboardIndex, ClipboardItem, ClipboardStatus, ClipboardStoreState};
use arboard::Clipboard;
use chrono::Local;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::time::{self, Duration};
use uuid::Uuid;

pub struct ClipboardManager {
    config: Config,
    store: ClipboardStoreState,
    index: ClipboardIndex,
    status: ClipboardStatus,
}

impl ClipboardManager {
    pub async fn new(config: Config) -> Result<Self, Box<dyn Error + Send + Sync>> {
        tokio::fs::create_dir_all(config.get_clipboard_dir()).await?;
        tokio::fs::create_dir_all(config.get_clipboard_export_dir()).await?;

        let store = load_or_default::<ClipboardStoreState>(&config.get_clipboard_store_path()).await;
        let index = load_or_default::<ClipboardIndex>(&config.get_clipboard_index_path()).await;

        let last_capture_time = store.items.iter().map(|item| item.last_seen).max();
        let status = ClipboardStatus {
            enabled: config.clipboard_enabled,
            auto_save: config.clipboard_auto_save,
            total_items: store.items.len(),
            last_capture_time,
        };

        Ok(Self {
            config,
            store,
            index,
            status,
        })
    }

    pub fn status(&self) -> ClipboardStatus {
        let mut status = self.status.clone();
        status.total_items = self.store.items.len();
        status
    }

    pub fn list_recent(&self, limit: usize) -> Vec<ClipboardItem> {
        let mut items = self.store.items.clone();
        items.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
        items.into_iter().take(limit).collect()
    }

    pub fn set_auto_save(&mut self, enabled: bool) {
        self.status.auto_save = enabled;
    }

    pub fn apply_runtime_config(&mut self, config: &Config) {
        self.config = config.clone();
        self.status.enabled = config.clipboard_enabled;
        self.status.auto_save = config.clipboard_auto_save;
    }

    pub async fn capture_from_text(
        &mut self,
        text: &str,
    ) -> Result<Option<ClipboardItem>, Box<dyn Error + Send + Sync>> {
        if !self.status.enabled {
            return Ok(None);
        }

        if text.trim().is_empty() {
            return Ok(None);
        }

        if text.len() > self.config.clipboard_max_bytes {
            return Ok(None);
        }

        let normalized = normalize_content(text);
        let hash = calculate_hash(&normalized);
        let now = Local::now();

        if let Some(existing_id) = self.index.hash_to_id.get(&hash).cloned() {
            if let Some(item) = self.store.items.iter_mut().find(|item| item.id == existing_id) {
                let item_id = item.id.clone();
                let seen_before = item.seen_count;
                item.seen_count += 1;
                item.last_seen = now;
                self.status.last_capture_time = Some(now);
                self.persist().await?;
                self.log_event(
                    "clipboard_fetch",
                    &format!(
                        "dedup_hit id={} hash={} len={} seen_count_before={}",
                        item_id,
                        short_hash(&hash),
                        normalized.chars().count(),
                        seen_before
                    ),
                )
                .await;
                return Ok(None);
            }
        }

        self.log_event(
            "clipboard_fetch",
            &format!(
                "new_item hash={} len={} preview={}",
                short_hash(&hash),
                normalized.chars().count(),
                preview(&normalized, 80)
            ),
        )
        .await;

        let decision = self.should_save_by_ai(&normalized).await?;
        if !decision.save {
            self.log_event(
                "clipboard_save",
                &format!(
                    "skip_by_ai hash={} len={} category={} reason={}",
                    short_hash(&hash),
                    normalized.chars().count(),
                    decision.category,
                    decision.reason
                ),
            )
            .await;
            return Ok(None);
        }

        let mut item = ClipboardItem {
            id: Uuid::new_v4().to_string(),
            timestamp: now,
            content: normalized,
            content_type: "text/plain".to_string(),
            hash: hash.clone(),
            seen_count: 1,
            last_seen: now,
            saved_path: None,
        };

        if self.status.auto_save {
            if let Some(path) = self.save_item_to_markdown_inner(&mut item, None).await? {
                item.saved_path = Some(path.to_string_lossy().to_string());
                self.log_event(
                    "clipboard_save",
                    &format!("auto_saved id={} path={}", item.id, path.to_string_lossy()),
                )
                .await;
            }
        }

        self.index.hash_to_id.insert(hash, item.id.clone());
        self.store.items.push(item.clone());
        self.status.last_capture_time = Some(now);
        self.persist().await?;
        Ok(Some(item))
    }

    pub async fn save_item_to_markdown(
        &mut self,
        id: &str,
        target_dir: Option<PathBuf>,
    ) -> Result<Option<PathBuf>, Box<dyn Error + Send + Sync>> {
        let maybe_idx = self.store.items.iter().position(|item| item.id == id);
        let idx = match maybe_idx {
            Some(i) => i,
            None => return Ok(None),
        };

        let mut item = self.store.items[idx].clone();
        let saved = self.save_item_to_markdown_inner(&mut item, target_dir).await?;
        if let Some(path) = &saved {
            self.store.items[idx].saved_path = Some(path.to_string_lossy().to_string());
            self.log_event(
                "clipboard_save",
                &format!("manual_saved id={} path={}", id, path.to_string_lossy()),
            )
            .await;
            self.persist().await?;
        }
        Ok(saved)
    }

    async fn save_item_to_markdown_inner(
        &self,
        item: &mut ClipboardItem,
        target_dir: Option<PathBuf>,
    ) -> Result<Option<PathBuf>, Box<dyn Error + Send + Sync>> {
        let export_dir = target_dir.unwrap_or_else(|| self.config.get_clipboard_export_dir());
        tokio::fs::create_dir_all(&export_dir).await?;

        let short_hash = item.hash.chars().take(8).collect::<String>();
        let slug = build_content_slug(&item.content, 32);
        let base = format!(
            "{}_{}_{}",
            item.timestamp.format("%Y%m%d_%H%M%S"),
            short_hash,
            slug
        );
        let mut path = export_dir.join(format!("{}.md", base));
        let mut suffix = 1usize;
        while tokio::fs::try_exists(&path).await.unwrap_or(false) {
            path = export_dir.join(format!("{}_{}.md", base, suffix));
            suffix += 1;
        }

        let md = format!(
            "# Clipboard Note\n\n- id: {}\n- captured_at: {}\n- hash: {}\n- seen_count: {}\n\n---\n\n{}\n",
            item.id,
            item.timestamp.format("%Y-%m-%d %H:%M:%S"),
            item.hash,
            item.seen_count,
            item.content
        );
        tokio::fs::write(&path, md).await?;
        Ok(Some(path))
    }

    async fn persist(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let store_json = serde_json::to_string_pretty(&self.store)?;
        let index_json = serde_json::to_string_pretty(&self.index)?;
        tokio::fs::write(self.config.get_clipboard_store_path(), store_json).await?;
        tokio::fs::write(self.config.get_clipboard_index_path(), index_json).await?;
        Ok(())
    }

    async fn should_save_by_ai(
        &self,
        content: &str,
    ) -> Result<ClipboardDecisionNormalized, Box<dyn Error + Send + Sync>> {
        if !self.config.clipboard_ai_filter_enabled {
            return Ok(ClipboardDecisionNormalized {
                save: true,
                reason: "ai_filter_disabled".to_string(),
                category: "other".to_string(),
            });
        }

        if content.chars().count() < self.config.clipboard_ai_min_chars {
            self.log_event(
                "clipboard_ai",
                &format!(
                    "skip_min_chars len={} threshold={}",
                    content.chars().count(),
                    self.config.clipboard_ai_min_chars
                ),
            )
            .await;
            return Ok(ClipboardDecisionNormalized {
                save: false,
                reason: format!("below_min_chars_{}", self.config.clipboard_ai_min_chars),
                category: "noise".to_string(),
            });
        }

        match request_clipboard_ai_decision(&self.config, content).await {
            Ok(decision) => {
                let normalized = normalize_decision(decision);
                self.log_event(
                    "clipboard_ai",
                    &format!(
                        "decision save={} category={} reason={} len={}",
                        normalized.save,
                        normalized.category,
                        normalized.reason,
                        content.chars().count()
                    ),
                )
                .await;
                Ok(normalized)
            }
            Err(e) => {
                eprintln!("剪贴板 AI 判定失败: {}", e);
                self.log_event("clipboard_ai", &format!("error {}", e)).await;
                Ok(ClipboardDecisionNormalized {
                    save: self.config.clipboard_ai_save_on_error,
                    reason: format!("ai_error_{}", sanitize_for_log(&e.to_string(), 60)),
                    category: "other".to_string(),
                })
            }
        }
    }

    async fn log_event(&self, event: &str, detail: &str) {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("[{}] {} | {}\n", now, event, detail);
        print!("📋 {}", line);
        let path = self.config.get_clipboard_dir().join("events.log");
        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
        {
            let _ = file.write_all(line.as_bytes()).await;
        }
    }
}

pub async fn run_clipboard_loop(
    config: Config,
    manager: std::sync::Arc<tokio::sync::Mutex<ClipboardManager>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut config = config;
    if !config.clipboard_enabled {
        return Ok(());
    }

    let mut current_interval_ms = config.clipboard_interval_ms.max(100);
    let mut timer = time::interval(Duration::from_millis(current_interval_ms));
    let mut last_hash = String::new();

    loop {
        timer.tick().await;

        // 运行时自动重载 .env 配置
        if let Ok(changed) = config.reload_from_dotenv_and_args() {
            if changed {
                let new_interval_ms = config.clipboard_interval_ms.max(100);
                if new_interval_ms != current_interval_ms {
                    current_interval_ms = new_interval_ms;
                    timer = time::interval(Duration::from_millis(current_interval_ms));
                    println!(
                        "🔄 检测到 .env 变更，剪贴板轮询间隔已更新为 {} ms",
                        current_interval_ms
                    );
                }
                let mut guard = manager.lock().await;
                guard.apply_runtime_config(&config);
            }
        }

        if !config.clipboard_enabled {
            continue;
        }

        let text = read_clipboard_text().await?;
        if let Some(content) = text {
            let normalized = normalize_content(&content);
            let current_hash = calculate_hash(&normalized);
            if current_hash == last_hash {
                continue;
            }

            last_hash = current_hash;
            let mut guard = manager.lock().await;
            let _ = guard.capture_from_text(&content).await?;
        }
    }
}

async fn read_clipboard_text() -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let result = tokio::task::spawn_blocking(move || {
        let mut clipboard = Clipboard::new()?;
        clipboard.get_text().map(Some)
    })
    .await?;

    match result {
        Ok(text) => Ok(text),
        Err(_) => Ok(None),
    }
}

fn normalize_content(input: &str) -> String {
    let normalized_line_breaks = input.replace("\r\n", "\n").replace('\r', "\n");
    normalized_line_breaks.trim().to_string()
}

fn calculate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn build_content_slug(content: &str, max_len: usize) -> String {
    let first_line = content.lines().next().unwrap_or("").trim().to_lowercase();
    if first_line.is_empty() {
        return "clipboard".to_string();
    }

    let mut slug = String::new();
    let mut last_is_sep = false;
    for ch in first_line.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_is_sep = false;
        } else if !last_is_sep {
            slug.push('_');
            last_is_sep = true;
        }

        if slug.len() >= max_len {
            break;
        }
    }

    let trimmed = slug.trim_matches('_');
    if trimmed.is_empty() {
        "clipboard".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Deserialize)]
struct ClipboardDecision {
    save: bool,
    reason: Option<String>,
    category: Option<String>,
}

struct ClipboardDecisionNormalized {
    save: bool,
    reason: String,
    category: String,
}

async fn request_clipboard_ai_decision(
    config: &Config,
    content: &str,
) -> Result<ClipboardDecision, Box<dyn Error + Send + Sync>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(
            config.clipboard_ai_timeout_seconds,
        ))
        .build()?;

    // 固定输出协议：即使用户自定义 prompt，也必须输出固定 JSON 结构
    let protocol = "你必须严格只输出一行 JSON，且字段结构固定为：{\"save\": true|false, \"reason\": \"...\", \"category\": \"url|keyword_combo|topic|noise|sensitive|other\"}。不要输出任何额外文本、markdown、代码块。";
    let user_prompt = format!(
        "{}\n\n{}\n\n待判断内容：\n<<<\n{}\n>>>",
        protocol, config.clipboard_ai_filter_prompt, content
    );

    let request_body = serde_json::json!({
        "model": config.model,
        "messages": [
            { "role": "user", "content": user_prompt }
        ],
        "temperature": 0
    });

    let response = client
        .post(&config.api_url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("AI过滤请求失败: {} - {}", status, body).into());
    }

    let text = response.text().await?;
    let completion: ChatCompletionResponse = serde_json::from_str(&text)?;
    let content = completion
        .choices
        .first()
        .map(|choice| choice.message.content.trim().to_string())
        .ok_or("AI过滤返回为空")?;

    let decision: ClipboardDecision = serde_json::from_str(&content)
        .or_else(|_| {
            // 容错：部分模型会返回 markdown 包裹
            let cleaned = content
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            serde_json::from_str(cleaned)
        })?;

    Ok(decision)
}

fn short_hash(hash: &str) -> String {
    hash.chars().take(8).collect()
}

fn preview(content: &str, max: usize) -> String {
    let mut p = content.lines().next().unwrap_or("").trim().to_string();
    if p.chars().count() > max {
        p = p.chars().take(max).collect::<String>();
    }
    p.replace('\n', " ")
}

fn normalize_decision(raw: ClipboardDecision) -> ClipboardDecisionNormalized {
    let reason = raw
        .reason
        .unwrap_or_else(|| "no_reason".to_string())
        .trim()
        .to_string();
    let category_raw = raw
        .category
        .unwrap_or_else(|| "other".to_string())
        .trim()
        .to_lowercase();
    let category = match category_raw.as_str() {
        "url" | "keyword_combo" | "topic" | "noise" | "sensitive" | "other" => category_raw,
        _ => "other".to_string(),
    };
    ClipboardDecisionNormalized {
        save: raw.save,
        reason: if reason.is_empty() { "no_reason".to_string() } else { sanitize_for_log(&reason, 80) },
        category,
    }
}

fn sanitize_for_log(input: &str, max_chars: usize) -> String {
    let mut out = input.replace('\n', " ").replace('\r', " ");
    if out.chars().count() > max_chars {
        out = out.chars().take(max_chars).collect::<String>();
    }
    out
}

async fn load_or_default<T>(path: &Path) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str::<T>(&content).unwrap_or_default(),
        Err(_) => T::default(),
    }
}
