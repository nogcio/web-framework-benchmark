use crate::prelude::*;
use crate::db::Db;
use crate::analysis_context::{AnalysisContext, AnalysisLanguage};
use std::path::PathBuf;
use std::fs;
use std::collections::HashSet;
use serde_json::json;
use reqwest::Client;

pub async fn run_analysis(
    db: Db,
    run_id: Option<u32>,
    api_key: String,
    model: String,
    api_url: String,
    languages: Vec<AnalysisLanguage>,
) -> Result<()> {
    let runs = db.get_full_runs()?;
    let frameworks_info = db.get_framework_records()?;
    let languages_info = db.get_language_records()?;
    let benchmarks_info = db.get_benchmark_records()?;

    let client = Client::new();

    for run in runs {
        if let Some(id) = run_id && run.id != id {
            continue;
        }

        info!("Analyzing run {}", run.id);

        // Get unique environments and sort them
        let mut environments: Vec<String> = run.frameworks.iter()
            .map(|f| f.environment.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        environments.sort();

        for environment in environments {
            // Iterate benchmarks in order
            for benchmark_config in &benchmarks_info {
                // Find corresponding framework run
                let framework_run = match run.frameworks.iter().find(|f| 
                    f.environment == environment && 
                    f.framework == benchmark_config.name && 
                    f.language == benchmark_config.language
                ) {
                    Some(f) => f,
                    None => continue,
                };

                let language = &framework_run.language;
                let framework = &framework_run.framework;

                // Find framework info
                let framework_info = frameworks_info.iter().find(|f| f.name == *framework);
                let language_info = languages_info.iter().find(|l| l.name == *language);
                
                // Load environment info
                let env_info = db.get_environment(&environment)?;

                // Iterate tests in order
                for test in &benchmark_config.tests {
                    let result = match framework_run.results.get(test) {
                        Some(r) => r,
                        None => continue,
                    };

                    let test_name = serde_json::to_value(test).unwrap().as_str().unwrap().to_string();
                    let test_description = test.description();

                    let file_path = PathBuf::from("data")
                        .join(run.id.to_string())
                        .join(&environment)
                        .join(language)
                        .join(framework)
                        .join(format!("{}.yaml", test_name));
                    
                    for lang_config in &languages {
                        let extension = format!("{}.md", lang_config.as_str());
                        // If the original file is test.yaml, with_extension("en.md") makes it test.en.md
                        let md_path = file_path.with_extension(&extension);

                        if md_path.exists() {
                            debug!("Skipping {}, {} file exists", file_path.display(), extension);
                            continue;
                        }

                        info!("Analyzing {} for {}", file_path.display(), lang_config.as_str());

                        let context = AnalysisContext {
                            framework,
                            language,
                            test: &test_name,
                            test_description,
                            framework_info,
                            language_info,
                            benchmark_config: Some(benchmark_config),
                            environment: env_info.as_ref(),
                            results: result,
                        };

                        // Serialize context to JSON
                        let json_content = match serde_json::to_string(&context) {
                            Ok(content) => content,
                            Err(e) => {
                                error!("Failed to serialize context for {}: {}", file_path.display(), e);
                                continue;
                            }
                        };

                        // Construct prompt
                        let prompt = format!(
                            "{}\n\nAnalyze the following benchmark results for test: \"{}\". \
                            Test description: \"{}\". \
                            The results include a series of samples with increasing concurrency (connections). \
                            \
                            Your task is to provide a deep analysis of the performance dynamics, not just a summary of the final numbers. \
                            \
                            Please cover the following points: \
                            1. **Throughput Dynamics**: How does RPS change as concurrency increases? Is there a saturation point? Does it degrade under high load? \
                            2. **Latency Analysis**: Analyze how latency (avg, p99, max) behaves. Identify the concurrency level where latency starts to degrade significantly. \
                            3. **Stability**: Look at standard deviations and error rates. Are there signs of instability or resource exhaustion? \
                            4. **Key Insights**: Provide the architect with useful information. What are the advantages? What are the critical points? There is no need to explain what needs to be profiled and increased resources. \
                            \
                            Do not simply list the numbers from the JSON. Interpret them. \
                            \
                            \n\n```json\n{}\n```",
                            lang_config.prompt_instruction(),
                            test_name,
                            test_description,
                            json_content
                        );

                        // Call AI
                        match call_ai(&client, &api_key, &model, &api_url, &prompt).await {
                            Ok(response) => {
                                // Save to MD
                                if let Err(e) = fs::write(&md_path, response) {
                                    error!("Failed to write analysis to {}: {}", md_path.display(), e);
                                } else {
                                    info!("Saved analysis to {}", md_path.display());
                                }
                            }
                            Err(e) => {
                                error!("Failed to analyze {}: {}", file_path.display(), e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn call_ai(
    client: &Client,
    api_key: &str,
    model: &str,
    api_url: &str,
    prompt: &str,
) -> Result<String> {
    let request_body = json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "You are an expert software performance analyst. Analyze the provided web framework benchmark results. The audience is high-level specialists comparing frameworks who already see the raw data. Provide a concise, technical summary focusing on key performance characteristics, stability, and resource usage. Highlight value and important data points in the text using inline code blocks. When highlighting approximate values with a tilde (e.g., ~10ms), always include the tilde inside the inline code block (e.g., `~10ms`, not ~`10ms`). Use bold text for key points (e.g., **Throughput**, **Latency**, **Stability**, **Key Insights**). Do not use headers. Keep it short and to the point. Format as Markdown."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "stream": false
    });

    let response = client.post(format!("{}/chat/completions", api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| Error::System(format!("Failed to call AI: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::System(format!("AI API error: {}", error_text)));
    }

    let response_json: serde_json::Value = response.json().await
        .map_err(|e| Error::System(format!("Failed to parse AI response: {}", e)))?;

    let content = response_json["choices"][0]["message"]["content"].as_str()
        .ok_or_else(|| Error::System("Invalid AI response format".to_string()))?;

    Ok(content.to_string())
}
