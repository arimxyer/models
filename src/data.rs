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
    /// Returns true if this model outputs text (or has no modalities specified).
    /// Non-text models (image gen, video gen, embeddings) return false.
    #[cfg(test)]
    pub fn is_text_model(&self) -> bool {
        match &self.modalities {
            Some(m) => m.output.iter().any(|o| o == "text"),
            None => true,
        }
    }

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

    /// Compact cost string for list columns (rounded to 1 decimal place).
    pub fn cost_short(value: Option<f64>) -> String {
        match value {
            Some(v) if v >= 100.0 => format!("${:.0}", v),
            Some(v) if v >= 1.0 => format!("${:.1}", v),
            Some(v) if v >= 0.01 => format!("${:.2}", v),
            Some(v) => format!("${:.3}", v),
            None => "\u{2014}".to_string(),
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
        let m = n as f64 / 1_000_000.0;
        if m.fract() == 0.0 {
            format!("{}M", m as u64)
        } else {
            format!("{:.1}M", m)
        }
    } else if n >= 1_000 {
        let k = n as f64 / 1_000.0;
        if k.fract() == 0.0 {
            format!("{}k", k as u64)
        } else {
            format!("{:.1}k", k)
        }
    } else {
        n.to_string()
    }
}

pub type ProvidersMap = HashMap<String, Provider>;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model(output_modalities: Option<Vec<&str>>) -> Model {
        Model {
            id: "test".into(),
            name: "Test".into(),
            family: None,
            reasoning: false,
            tool_call: false,
            attachment: false,
            temperature: false,
            modalities: output_modalities.map(|out| Modalities {
                input: vec!["text".into()],
                output: out.into_iter().map(|s| s.to_string()).collect(),
            }),
            cost: None,
            limit: None,
            release_date: None,
            last_updated: None,
            knowledge: None,
            open_weights: false,
            status: None,
        }
    }

    #[test]
    fn test_is_text_model_none_modalities() {
        let m = make_model(None);
        assert!(m.is_text_model(), "No modalities should default to text");
    }

    #[test]
    fn test_is_text_model_text_output() {
        let m = make_model(Some(vec!["text"]));
        assert!(m.is_text_model());
    }

    #[test]
    fn test_is_text_model_multimodal_with_text() {
        let m = make_model(Some(vec!["text", "image"]));
        assert!(m.is_text_model(), "Multimodal with text should be text");
    }

    #[test]
    fn test_is_text_model_image_only() {
        let m = make_model(Some(vec!["image"]));
        assert!(!m.is_text_model(), "Image-only model is not text");
    }

    #[test]
    fn test_is_text_model_video_only() {
        let m = make_model(Some(vec!["video"]));
        assert!(!m.is_text_model(), "Video-only model is not text");
    }

    #[test]
    fn test_is_text_model_empty_output() {
        let m = make_model(Some(vec![]));
        assert!(!m.is_text_model(), "Empty output modalities is not text");
    }
}
