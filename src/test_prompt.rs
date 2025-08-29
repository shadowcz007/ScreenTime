use crate::siliconflow;
use crate::logger;
use crate::models::ActivityLog;
use crate::config::Config;
use crate::context;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;

pub async fn run_test_prompt(config: Config) -> Result<(), Box<dyn Error + Send + Sync>> {
    let test_prompt = config.test_prompt.as_ref()
        .ok_or("测试prompt不能为空")?;
    
    println!("🧪 开始测试新prompt...");
    println!("📝 测试prompt: {}", test_prompt);
    println!("📊 使用日志文件: {:?}", config.log_path);
    println!("💾 测试结果保存到: {:?}", config.test_log_path);
    println!();

    // 读取现有的活动日志
    let log_path_str = config.log_path.to_str().unwrap_or("activity_log.json");
    let existing_logs = logger::load_activity_logs(log_path_str)?;
    
    if existing_logs.is_empty() {
        return Err("没有找到现有的活动日志，无法进行测试".into());
    }

    println!("📋 找到 {} 条现有记录，开始重新分析...", existing_logs.len());

    let mut processed_count = 0;
    let mut success_count = 0;
    let mut skip_count = 0;

    // 初始化测试日志文件
    initialize_test_log(&config.test_log_path)?;
    println!("💾 测试日志文件已初始化: {:?}", config.test_log_path);

    for (index, original_log) in existing_logs.iter().enumerate() {
        processed_count += 1;
        println!("🔄 处理第 {}/{} 条记录...", processed_count, existing_logs.len());

        // 检查截图文件是否存在
        if let Some(screenshot_path) = &original_log.screenshot_path {
            if !std::path::Path::new(screenshot_path).exists() {
                println!("⚠️  截图文件不存在: {}，跳过此记录", screenshot_path);
                skip_count += 1;
                continue;
            }

            // 获取历史活动上下文（排除当前记录）
            let history_context = get_history_context_excluding_current(&existing_logs, index, 5)?;

            // 使用新的prompt重新分析截图
            match siliconflow::analyze_screenshot_with_prompt(
                &config.api_key,
                &config.model,
                screenshot_path,
                test_prompt,
                original_log.context.as_ref().map(|ctx| context::format_context_as_text(ctx)).as_deref(),
                Some(&history_context),
            ).await {
                Ok(new_description) => {
                    println!("✅ 重新分析完成: {}", new_description.lines().next().unwrap_or("无描述"));

                    // 创建新的测试日志条目
                    let test_log = ActivityLog {
                        timestamp: original_log.timestamp,
                        description: new_description,
                        context: original_log.context.clone(),
                        screenshot_path: original_log.screenshot_path.clone(),
                    };

                    // 立即保存到测试日志文件
                    append_test_result(&test_log, &config.test_log_path)?;
                    println!("💾 已保存到测试日志");
                    
                    success_count += 1;
                },
                Err(e) => {
                    eprintln!("❌ 重新分析失败: {}", e);
                    skip_count += 1;
                    continue;
                }
            }
        } else {
            println!("⚠️  记录中没有截图路径，跳过此记录");
            skip_count += 1;
        }
    }

    // 显示最终统计信息
    println!("\n🎉 测试完成！");
    println!("📊 成功重新分析了 {} 条记录", success_count);
    println!("⚠️  跳过了 {} 条记录", skip_count);
    println!("💾 结果已保存到: {:?}", config.test_log_path);
    
    // 读取最终结果进行对比
    let final_results = load_test_results(&config.test_log_path)?;
    if !final_results.is_empty() {
        show_comparison_summary(&existing_logs, &final_results)?;
    } else {
        println!("❌ 没有成功重新分析任何记录");
    }

    Ok(())
}

/// 获取历史活动上下文，排除当前记录
fn get_history_context_excluding_current(
    logs: &[ActivityLog], 
    current_index: usize, 
    count: usize
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut context = String::new();
    context.push_str("【用户最近的活动历史】\n");
    
    let mut added_count = 0;
    let mut index = 0;
    
    // 从最新的记录开始，跳过当前记录
    for log in logs.iter().rev() {
        if index == current_index {
            index += 1;
            continue;
        }
        
        if added_count >= count {
            break;
        }
        
        context.push_str(&format!(
            "{}. 时间: {}\n   描述: {}\n\n",
            added_count + 1,
            log.timestamp.format("%Y-%m-%d %H:%M:%S"),
            log.description.trim()
        ));
        
        added_count += 1;
        index += 1;
    }
    
    if added_count == 0 {
        context.push_str("暂无历史活动记录\n");
    }
    
    Ok(context)
}

/// 初始化测试日志文件
fn initialize_test_log(file_path: &std::path::Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    let file = File::create(file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &Vec::<ActivityLog>::new())?;
    Ok(())
}

/// 追加测试结果到文件
fn append_test_result(result: &ActivityLog, file_path: &std::path::Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 读取现有结果
    let mut results = load_test_results(file_path)?;
    
    // 添加新结果
    results.push(result.clone());
    
    // 保存更新后的结果
    let file = File::create(file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &results)?;
    Ok(())
}

/// 读取测试结果
fn load_test_results(file_path: &std::path::Path) -> Result<Vec<ActivityLog>, Box<dyn Error + Send + Sync>> {
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    
    let file = File::open(file_path)?;
    let results: Vec<ActivityLog> = serde_json::from_reader(file)?;
    Ok(results)
}

/// 显示对比摘要
fn show_comparison_summary(original: &[ActivityLog], test: &[ActivityLog]) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("\n📈 对比摘要:");
    println!("原始记录数: {}", original.len());
    println!("测试记录数: {}", test.len());
    
    if original.len() == test.len() {
        println!("✅ 所有记录都成功重新分析");
    } else {
        println!("⚠️  部分记录重新分析失败");
    }
    
    // 计算描述长度对比
    let original_avg_length: f64 = original.iter()
        .map(|log| log.description.len())
        .sum::<usize>() as f64 / original.len() as f64;
    
    let test_avg_length: f64 = test.iter()
        .map(|log| log.description.len())
        .sum::<usize>() as f64 / test.len() as f64;
    
    println!("📏 描述长度对比:");
    println!("  原始平均长度: {:.1} 字符", original_avg_length);
    println!("  测试平均长度: {:.1} 字符", test_avg_length);
    println!("  长度变化: {:.1}%", ((test_avg_length - original_avg_length) / original_avg_length * 100.0));
    
    Ok(())
}
