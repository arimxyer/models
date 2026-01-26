use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub npm: Option<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub doc: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub models: HashMap<String, Model>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub temperature: bool,
    #[serde(default)]
    pub modalities: Option<Modalities>,
    #[serde(default)]
    pub cost: Option<Cost>,
    #[serde(default)]
    pub limit: Option<Limits>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub last_updated: Option<String>,
    #[serde(default)]
    pub knowledge: Option<String>,
    #[serde(default)]
    pub open_weights: bool,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cost {
    #[serde(default)]
    pub input: Option<f64>,
    #[serde(default)]
    pub output: Option<f64>,
    #[serde(default)]
    pub cache_read: Option<f64>,
    #[serde(default)]
    pub cache_write: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Limits {
    #[serde(default)]
    pub context: Option<u64>,
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Modalities {
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
}

impl Model {
    pub fn context_str(&self) -> String {
        self.limit
            .as_ref()
            .and_then(|l| l.context)
            .map(format_tokens)
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn output_str(&self) -> String {
        self.limit
            .as_ref()
            .and_then(|l| l.output)
            .map(format_tokens)
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn input_limit_str(&self) -> String {
        self.limit
            .as_ref()
            .and_then(|l| l.input)
            .map(format_tokens)
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn cost_str(&self) -> String {
        match &self.cost {
            Some(c) => {
                let input = c
                    .input
                    .map(|v| format!("${}", v))
                    .unwrap_or("-".to_string());
                let output = c
                    .output
                    .map(|v| format!("${}", v))
                    .unwrap_or("-".to_string());
                format!("{}/{}", input, output)
            }
            None => "-/-".to_string(),
        }
    }

    pub fn capabilities_str(&self) -> String {
        let mut caps = Vec::new();
        if self.reasoning {
            caps.push("reasoning");
        }
        if self.tool_call {
            caps.push("tools");
        }
        if self.attachment {
            caps.push("files");
        }
        if self.temperature {
            caps.push("temperature");
        }
        if caps.is_empty() {
            "-".to_string()
        } else {
            caps.join(", ")
        }
    }

    pub fn modalities_str(&self) -> String {
        match &self.modalities {
            Some(m) => {
                let input = if m.input.is_empty() {
                    "text".to_string()
                } else {
                    m.input.join(", ")
                };
                let output = if m.output.is_empty() {
                    "text".to_string()
                } else {
                    m.output.join(", ")
                };
                format!("{} -> {}", input, output)
            }
            None => "text -> text".to_string(),
        }
    }
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

pub type ProvidersMap = HashMap<String, Provider>;
