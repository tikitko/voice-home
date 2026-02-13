use crate::config::ToolConfig;
use serde_json::{Value, json};
use std::process::Command;

pub struct ToolManager {
    tools: Vec<ToolConfig>,
}

impl ToolManager {
    pub fn new(tools: Vec<ToolConfig>) -> Self {
        eprintln!("[Инструменты]: загружено {} инструментов", tools.len());
        for t in &tools {
            eprintln!("  - {}", t.name);
        }
        Self { tools }
    }

    /// Return tool definitions in the format expected by OpenAI (name, description, inputSchema).
    pub fn tools(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|t| {
                let mut properties = json!({});
                for (name, param) in &t.params {
                    properties[name] = json!({
                        "type": param.param_type,
                        "description": param.description,
                    });
                }
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": {
                        "type": "object",
                        "properties": properties,
                        "required": t.required_params,
                    }
                })
            })
            .collect()
    }

    /// Execute a tool call by name. Substitutes `{{param}}` placeholders in the
    /// command template with actual argument values, then runs via `sh -c`.
    pub fn call_tool(&self, name: &str, args: Value) -> String {
        let tool = match self.tools.iter().find(|t| t.name == name) {
            Some(t) => t,
            None => return format!("Инструмент «{}» не найден", name),
        };

        let mut cmd = tool.command.clone();
        if let Some(obj) = args.as_object() {
            for (key, val) in obj {
                let replacement = match val {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                cmd = cmd.replace(&format!("{{{{{}}}}}", key), &replacement);
            }
        }

        eprintln!("[Инструмент «{}»]: {}", name, cmd);

        match Command::new("sh").arg("-c").arg(&cmd).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if output.status.success() {
                    if stdout.is_empty() {
                        "OK".into()
                    } else {
                        stdout
                    }
                } else {
                    format!(
                        "Ошибка (код {}): {}",
                        output.status,
                        if stderr.is_empty() { &stdout } else { &stderr }
                    )
                }
            }
            Err(e) => format!("Ошибка запуска: {}", e),
        }
    }
}
