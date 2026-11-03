use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const REQUEST_TIMEOUT_SECS: u64 = 180;
const MODELS_TIMEOUT_SECS: u64 = 30;
const MAX_ERROR_BODY: usize = 500;

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiToolCall {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<AiToolCall>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatRequest {
    pub provider: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    pub messages: Vec<AiMessage>,
    #[serde(default)]
    pub tools: Vec<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatResponse {
    pub content: String,
    pub tool_calls: Vec<AiToolCall>,
    pub finish_reason: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelsRequest {
    pub provider: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
}

fn default_temperature() -> f64 {
    0.2
}

fn arguments_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "{}".to_string(),
        other => other.to_string(),
    }
}

fn arguments_object(value: &Value) -> Value {
    match value {
        Value::String(s) => serde_json::from_str(s).unwrap_or_else(|_| json!({})),
        Value::Null => json!({}),
        other => other.clone(),
    }
}

fn snippet(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= MAX_ERROR_BODY {
        return trimmed.to_string();
    }
    let cut: String = trimmed.chars().take(MAX_ERROR_BODY).collect();
    format!("{cut}…")
}

fn openai_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let base = base.strip_suffix("/chat/completions").unwrap_or(base);
    format!("{base}/chat/completions")
}

fn openai_models_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let base = base.strip_suffix("/chat/completions").unwrap_or(base);
    let base = base.strip_suffix("/models").unwrap_or(base);
    format!("{base}/models")
}

fn ollama_root(base_url: &str) -> &str {
    let base = base_url.trim().trim_end_matches('/');
    let base = base.strip_suffix("/api/chat").unwrap_or(base);
    let base = base.strip_suffix("/api/tags").unwrap_or(base);
    base.strip_suffix("/v1").unwrap_or(base)
}

fn ollama_url(base_url: &str) -> String {
    format!("{}/api/chat", ollama_root(base_url))
}

fn ollama_tags_url(base_url: &str) -> String {
    format!("{}/api/tags", ollama_root(base_url))
}

fn sorted_names(mut names: Vec<String>) -> Vec<String> {
    names.sort_by_key(|name| name.to_ascii_lowercase());
    names.dedup();
    names
}

fn parse_ollama_models(value: &Value) -> Vec<String> {
    let names = value
        .get("models")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter_map(|model| {
                    model
                        .get("name")
                        .and_then(Value::as_str)
                        .or_else(|| model.get("model").and_then(Value::as_str))
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    sorted_names(names)
}

fn parse_openai_models(value: &Value) -> Vec<String> {
    let names = value
        .get("data")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter_map(|model| model.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    sorted_names(names)
}

fn openai_message(message: &AiMessage) -> Value {
    match message.role.as_str() {
        "assistant" if !message.tool_calls.is_empty() => json!({
            "role": "assistant",
            "content": if message.content.is_empty() {
                Value::Null
            } else {
                Value::String(message.content.clone())
            },
            "tool_calls": message
                .tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": arguments_string(&call.arguments),
                        }
                    })
                })
                .collect::<Vec<_>>(),
        }),
        "tool" => json!({
            "role": "tool",
            "tool_call_id": message.tool_call_id.clone().unwrap_or_default(),
            "content": message.content,
        }),
        role => json!({
            "role": role,
            "content": message.content,
        }),
    }
}

fn ollama_message(message: &AiMessage) -> Value {
    match message.role.as_str() {
        "assistant" if !message.tool_calls.is_empty() => json!({
            "role": "assistant",
            "content": message.content,
            "tool_calls": message
                .tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "function": {
                            "name": call.name,
                            "arguments": arguments_object(&call.arguments),
                        }
                    })
                })
                .collect::<Vec<_>>(),
        }),
        "tool" => {
            let mut value = json!({
                "role": "tool",
                "content": message.content,
            });
            if let Some(name) = message.tool_name.as_ref().filter(|n| !n.is_empty()) {
                value["tool_name"] = Value::String(name.clone());
            }
            value
        }
        role => json!({
            "role": role,
            "content": message.content,
        }),
    }
}

fn parse_openai_tool_calls(value: &Value) -> Vec<AiToolCall> {
    value
        .as_array()
        .map(|calls| {
            calls
                .iter()
                .enumerate()
                .filter_map(|(index, call)| {
                    let name = call
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(Value::as_str)?;
                    let id = call
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("call_{index}"));
                    let arguments = call
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .map(arguments_string)
                        .unwrap_or_else(|| "{}".to_string());
                    Some(AiToolCall {
                        id,
                        name: name.to_string(),
                        arguments: Value::String(arguments),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_ollama_tool_calls(value: &Value) -> Vec<AiToolCall> {
    value
        .as_array()
        .map(|calls| {
            calls
                .iter()
                .enumerate()
                .filter_map(|(index, call)| {
                    let function = call.get("function")?;
                    let name = function.get("name").and_then(Value::as_str)?;
                    let arguments = function
                        .get("arguments")
                        .map(arguments_string)
                        .unwrap_or_else(|| "{}".to_string());
                    let id = call
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("call_{index}"));
                    Some(AiToolCall {
                        id,
                        name: name.to_string(),
                        arguments: Value::String(arguments),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

async fn response_json(response: reqwest::Response, provider: &str) -> Result<Value, String> {
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "{provider} request failed ({}): {}",
            status.as_u16(),
            snippet(&text)
        ));
    }
    serde_json::from_str(&text).map_err(|e| format!("Invalid {provider} response: {e}"))
}

async fn openai_chat(
    client: &reqwest::Client,
    request: &AiChatRequest,
) -> Result<AiChatResponse, String> {
    let messages: Vec<Value> = request.messages.iter().map(openai_message).collect();
    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "temperature": request.temperature,
    });
    if !request.tools.is_empty() {
        body["tools"] = Value::Array(request.tools.clone());
        body["tool_choice"] = Value::String("auto".to_string());
    }

    let mut builder = client.post(openai_url(&request.base_url)).json(&body);
    if !request.api_key.trim().is_empty() {
        builder = builder.bearer_auth(request.api_key.trim());
    }
    let response = builder.send().await.map_err(|e| e.to_string())?;
    let value = response_json(response, "OpenAI").await?;
    let choice = value
        .get("choices")
        .and_then(|c| c.get(0))
        .ok_or_else(|| "OpenAI response did not include any choices".to_string())?;
    let message = choice
        .get("message")
        .ok_or_else(|| "OpenAI response did not include a message".to_string())?;

    let content = match message.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    };
    let tool_calls = message
        .get("tool_calls")
        .map(parse_openai_tool_calls)
        .unwrap_or_default();
    let finish_reason = choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .unwrap_or("stop")
        .to_string();

    Ok(AiChatResponse {
        content,
        tool_calls,
        finish_reason,
    })
}

async fn ollama_chat(
    client: &reqwest::Client,
    request: &AiChatRequest,
) -> Result<AiChatResponse, String> {
    let messages: Vec<Value> = request.messages.iter().map(ollama_message).collect();
    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": false,
        "options": {
            "temperature": request.temperature,
        },
    });
    if !request.tools.is_empty() {
        body["tools"] = Value::Array(request.tools.clone());
    }

    let mut builder = client.post(ollama_url(&request.base_url)).json(&body);
    if !request.api_key.trim().is_empty() {
        builder = builder.bearer_auth(request.api_key.trim());
    }
    let response = builder.send().await.map_err(|e| e.to_string())?;
    let value = response_json(response, "Ollama").await?;
    let message = value
        .get("message")
        .ok_or_else(|| "Ollama response did not include a message".to_string())?;

    let content = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let tool_calls = message
        .get("tool_calls")
        .map(parse_ollama_tool_calls)
        .unwrap_or_default();
    let finish_reason = value
        .get("done_reason")
        .and_then(Value::as_str)
        .unwrap_or("stop")
        .to_string();

    Ok(AiChatResponse {
        content,
        tool_calls,
        finish_reason,
    })
}

#[tauri::command(async)]
pub async fn ai_chat(request: AiChatRequest) -> Result<AiChatResponse, String> {
    if request.model.trim().is_empty() {
        return Err("No AI model configured".to_string());
    }
    if request.base_url.trim().is_empty() {
        return Err("No AI provider URL configured".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    match request.provider.as_str() {
        "ollama" => ollama_chat(&client, &request).await,
        "openai" => openai_chat(&client, &request).await,
        other => Err(format!("Unsupported AI provider: {other}")),
    }
}

#[tauri::command(async)]
pub async fn ai_list_models(request: AiModelsRequest) -> Result<Vec<String>, String> {
    if request.base_url.trim().is_empty() {
        return Err("No AI provider URL configured".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(MODELS_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    match request.provider.as_str() {
        "ollama" => {
            let mut builder = client.get(ollama_tags_url(&request.base_url));
            if !request.api_key.trim().is_empty() {
                builder = builder.bearer_auth(request.api_key.trim());
            }
            let response = builder.send().await.map_err(|e| e.to_string())?;
            let value = response_json(response, "Ollama").await?;
            Ok(parse_ollama_models(&value))
        }
        "openai" => {
            let mut builder = client.get(openai_models_url(&request.base_url));
            if !request.api_key.trim().is_empty() {
                builder = builder.bearer_auth(request.api_key.trim());
            }
            let response = builder.send().await.map_err(|e| e.to_string())?;
            let value = response_json(response, "OpenAI").await?;
            Ok(parse_openai_models(&value))
        }
        other => Err(format!("Unsupported AI provider: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(role: &str, content: &str) -> AiMessage {
        AiMessage {
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            tool_name: None,
        }
    }

    #[test]
    fn normalizes_openai_tool_call_arguments_to_string() {
        let calls = parse_openai_tool_calls(&json!([{
            "id": "abc",
            "type": "function",
            "function": { "name": "list_commits", "arguments": "{\"limit\":5}" }
        }]));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "abc");
        assert_eq!(calls[0].name, "list_commits");
        assert_eq!(calls[0].arguments, json!("{\"limit\":5}"));
    }

    #[test]
    fn normalizes_ollama_object_arguments_to_string() {
        let calls = parse_ollama_tool_calls(&json!([{
            "function": { "name": "get_commit", "arguments": { "oid": "deadbeef" } }
        }]));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_0");
        assert_eq!(calls[0].arguments, json!("{\"oid\":\"deadbeef\"}"));
    }

    #[test]
    fn ollama_assistant_tool_calls_use_object_arguments() {
        let mut msg = message("assistant", "");
        msg.tool_calls = vec![AiToolCall {
            id: "call_1".into(),
            name: "get_status".into(),
            arguments: Value::String("{}".into()),
        }];
        let value = ollama_message(&msg);
        assert_eq!(value["tool_calls"][0]["function"]["arguments"], json!({}));
        assert!(value["tool_calls"][0].get("id").is_none());
    }

    #[test]
    fn openai_tool_messages_include_call_id() {
        let mut msg = message("tool", "clean");
        msg.tool_call_id = Some("abc".into());
        msg.tool_name = Some("get_status".into());
        let value = openai_message(&msg);
        assert_eq!(value["role"], "tool");
        assert_eq!(value["tool_call_id"], "abc");
        assert!(value.get("tool_name").is_none());
    }

    #[test]
    fn builds_provider_urls() {
        assert_eq!(
            openai_url("https://api.openai.com/v1/"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            openai_models_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            ollama_url("http://localhost:11434/v1"),
            "http://localhost:11434/api/chat"
        );
        assert_eq!(
            ollama_url("http://localhost:11434/"),
            "http://localhost:11434/api/chat"
        );
        assert_eq!(
            ollama_tags_url("http://localhost:11434/v1/"),
            "http://localhost:11434/api/tags"
        );
    }

    #[test]
    fn parses_installed_ollama_models() {
        let models = parse_ollama_models(&json!({
            "models": [
                { "name": "qwen2.5-coder:latest", "model": "qwen2.5-coder:latest" },
                { "name": "llama3.2:latest", "model": "llama3.2:latest" },
                { "name": "llama3.2:latest", "model": "llama3.2:latest" }
            ]
        }));
        assert_eq!(models, vec!["llama3.2:latest", "qwen2.5-coder:latest"]);
    }

    #[test]
    fn parses_openai_model_ids() {
        let models = parse_openai_models(&json!({
            "data": [{ "id": "gpt-4o-mini" }, { "id": "gpt-4o" }]
        }));
        assert_eq!(models, vec!["gpt-4o", "gpt-4o-mini"]);
    }
}
