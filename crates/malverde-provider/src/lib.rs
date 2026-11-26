use serde::{Deserialize, Serialize};

use malverde_core::{MalverdeError, Message, Result, ToolCall, Usage};

/// Esquema de parámetros JSON Schema de una herramienta, tal como se envía al modelo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Respuesta estructurada del proveedor LLM.
#[derive(Debug, Clone)]
pub struct Completion {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub finish_reason: String,
}

impl Completion {
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Abstracción de proveedor LLM: cualquier backend compatible puede implementarla.
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;

    /// Envía la conversación + herramientas y devuelve la siguiente respuesta del modelo.
    fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        temperature: f32,
        max_tokens: u32,
    ) -> Result<Completion>;
}

/// Convertidor de mensajes al formato wire de la API.
pub fn messages_to_wire(messages: &[Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|m| {
            let mut obj = serde_json::json!({ "role": m.role.as_str() });
            if let Some(c) = &m.content {
                obj["content"] = serde_json::Value::String(c.clone());
            } else {
                obj["content"] = serde_json::Value::Null;
            }
            if !m.tool_calls.is_empty() {
                obj["tool_calls"] = serde_json::json!(m.tool_calls.iter().map(|tc| serde_json::json!({
                    "id": tc.id,
                    "type": "function",
                    "function": { "name": tc.name, "arguments": tc.arguments.to_string() }
                })).collect::<Vec<_>>());
            }
            if let Some(id) = &m.tool_call_id {
                obj["tool_call_id"] = serde_json::Value::String(id.clone());
            }
            if let Some(name) = &m.name {
                obj["name"] = serde_json::Value::String(name.clone());
            }
            obj
        })
        .collect()
}

pub fn tools_to_wire(tools: &[ToolSchema]) -> Vec<serde_json::Value> {
    tools
        .iter()
        .map(|t| serde_json::json!({
            "type": "function",
            "function": {
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            }
        }))
        .collect()
}

#[derive(Debug, Deserialize)]
struct WireResponse {
    choices: Vec<WireChoice>,
    #[serde(default)]
    usage: WireUsage,
}

#[derive(Debug, Deserialize)]
struct WireChoice {
    #[serde(default)]
    message: WireMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct WireMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<WireToolCall>,
}

#[derive(Debug, Deserialize)]
struct WireToolCall {
    #[serde(default)]
    id: String,
    function: WireFunction,
}

#[derive(Debug, Deserialize)]
struct WireFunction {
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize, Default)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

/// Proveedor para cualquier endpoint compatible con la API de chat de OpenAI
/// (OpenAI, OpenRouter, Nous Portal, Groq, Ollama, LM Studio, vLLM…).
pub struct OpenAiCompatibleProvider {
    base_url: String,
    api_key: Option<String>,
    model: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            model: model.into(),
        }
    }
}

impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        "openai-compatible"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        temperature: f32,
        max_tokens: u32,
    ) -> Result<Completion> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages_to_wire(messages),
            "temperature": temperature,
            "max_tokens": max_tokens,
        });
        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools_to_wire(tools));
            body["tool_choice"] = serde_json::json!("auto");
        }

        let url = format!("{}/chat/completions", self.base_url);
        let mut req = ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(300));
        if let Some(key) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {key}"));
        }

        let response = req
            .send_json(body)
            .map_err(|e| match e {
                ureq::Error::Status(code, resp) => {
                    let text = resp.into_string().unwrap_or_default();
                    MalverdeError::provider(format!("HTTP {code}: {}", truncate(&text, 800)))
                }
                other => MalverdeError::provider(other.to_string()),
            })?;

        let parsed: WireResponse = response
            .into_json()
            .map_err(|e| MalverdeError::provider(format!("respuesta JSON inválida: {e}")))?;

        let choice = parsed.choices.first().ok_or_else(|| MalverdeError::provider("respuesta sin choices"))?;
        let tool_calls = choice
            .message
            .tool_calls
            .iter()
            .map(|tc| {
                let arguments = if tc.function.arguments.trim().is_empty() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    serde_json::from_str(&tc.function.arguments)
                        .unwrap_or_else(|_| serde_json::Value::String(tc.function.arguments.clone()))
                };
                ToolCall {
                    id: if tc.id.is_empty() { format!("call_{}", uuid_like()) } else { tc.id.clone() },
                    name: tc.function.name.clone(),
                    arguments,
                }
            })
            .collect();

        Ok(Completion {
            content: choice.message.content.clone(),
            tool_calls,
            usage: Usage {
                prompt_tokens: parsed.usage.prompt_tokens,
                completion_tokens: parsed.usage.completion_tokens,
                total_tokens: parsed.usage.total_tokens,
            },
            finish_reason: choice.finish_reason.clone().unwrap_or_else(|| "stop".to_string()),
        })
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("{:x}{:x}", nanos, std::process::id())
}

/// Proveedor determinista para tests, demos y modo offline.
#[derive(Default)]
pub struct MockProvider {
    pub model: String,
}

impl MockProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: model.into() }
    }
}

impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn complete(
        &self,
        messages: &[Message],
        _tools: &[ToolSchema],
        _temperature: f32,
        _max_tokens: u32,
    ) -> Result<Completion> {
        let last_user = messages
            .iter()
            .rev()
            .find(|m| m.role == malverde_core::Role::User)
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        // Si el último evento fue un resultado de herramienta, el mock termina.
        if messages.last().map(|m| m.role) == Some(malverde_core::Role::Tool) {
            let result = messages
                .last()
                .and_then(|m| m.content.clone())
                .unwrap_or_default();
            return Ok(Completion {
                content: Some(format!("Resultado recibido: {}", truncate(&result, 200))),
                tool_calls: vec![],
                usage: Usage { prompt_tokens: 10, completion_tokens: 10, total_tokens: 20 },
                finish_reason: "stop".to_string(),
            });
        }

        // Si la petición pide explícitamente "usa la herramienta now", el mock la llama.
        if last_user.contains("usa la herramienta now") || last_user.contains("usa now") {
            return Ok(Completion {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "call_mock_1".to_string(),
                    name: "now".to_string(),
                    arguments: serde_json::json!({}),
                }],
                usage: Usage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
                finish_reason: "tool_calls".to_string(),
            });
        }

        Ok(Completion {
            content: Some(format!("[mock:{model}] {last_user}", model = self.model)),
            tool_calls: vec![],
            usage: Usage { prompt_tokens: 10, completion_tokens: 10, total_tokens: 20 },
            finish_reason: "stop".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_messages_roundtrip() {
        let msgs = vec![
            Message::system("sys"),
            Message::user("hola"),
            Message::assistant_with_tool_calls(
                None,
                vec![ToolCall { id: "1".into(), name: "shell".into(), arguments: serde_json::json!({"cmd": "ls"}) }],
            ),
            Message::tool_result("1", "shell", "out"),
        ];
        let wire = messages_to_wire(&msgs);
        assert_eq!(wire.len(), 4);
        assert_eq!(wire[0]["role"], "system");
        assert_eq!(wire[2]["tool_calls"][0]["function"]["name"], "shell");
        assert_eq!(wire[3]["tool_call_id"], "1");
    }

    #[test]
    fn mock_provider_basic_echo() {
        let p = MockProvider::new("test");
        let c = p.complete(&[Message::user("ping")], &[], 0.7, 100).unwrap();
        assert!(!c.has_tool_calls());
        assert!(c.content.unwrap().contains("ping"));
    }
}
