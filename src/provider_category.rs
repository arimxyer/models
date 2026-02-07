use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderCategory {
    All,
    Origin,
    Cloud,
    #[default]
    Inference,
    Gateway,
    Tool,
}

impl ProviderCategory {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Origin => "Orig",
            Self::Cloud => "Cloud",
            Self::Inference => "Infra",
            Self::Gateway => "Gate",
            Self::Tool => "Tool",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Origin => "Origin",
            Self::Cloud => "Cloud Platform",
            Self::Inference => "Inference",
            Self::Gateway => "Gateway",
            Self::Tool => "Dev Tool",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::All => Color::White,
            Self::Origin => Color::Magenta,
            Self::Cloud => Color::Blue,
            Self::Inference => Color::Green,
            Self::Gateway => Color::Yellow,
            Self::Tool => Color::Cyan,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Origin,
            Self::Origin => Self::Cloud,
            Self::Cloud => Self::Inference,
            Self::Inference => Self::Gateway,
            Self::Gateway => Self::Tool,
            Self::Tool => Self::All,
        }
    }

    #[cfg(test)]
    fn display_order(self) -> u8 {
        match self {
            Self::All => 0,
            Self::Origin => 1,
            Self::Cloud => 2,
            Self::Inference => 3,
            Self::Gateway => 4,
            Self::Tool => 5,
        }
    }
}

pub fn provider_category(id: &str) -> ProviderCategory {
    match id {
        // Origin (26): Provider created these models
        "anthropic"
        | "openai"
        | "google"
        | "deepseek"
        | "mistral"
        | "cohere"
        | "xai"
        | "llama"
        | "inception"
        | "upstage"
        | "zhipuai"
        | "minimax"
        | "moonshotai"
        | "xiaomi"
        | "alibaba"
        | "perplexity"
        | "lucidquery"
        | "bailing"
        | "nova"
        | "alibaba-cn"
        | "minimax-cn"
        | "minimax-cn-coding-plan"
        | "minimax-coding-plan"
        | "moonshotai-cn"
        | "zhipuai-coding-plan"
        | "zai-coding-plan" => ProviderCategory::Origin,

        // Cloud (11): Runs models on broader cloud infra
        "amazon-bedrock"
        | "azure"
        | "azure-cognitive-services"
        | "google-vertex"
        | "google-vertex-anthropic"
        | "nvidia"
        | "ovhcloud"
        | "scaleway"
        | "vultr"
        | "sap-ai-core"
        | "cloudflare-workers-ai" => ProviderCategory::Cloud,

        // Inference (21): Specialized inference hosting
        "deepinfra" | "togetherai" | "fireworks-ai" | "groq" | "cerebras" | "baseten"
        | "novita-ai" | "friendli" | "nebius" | "chutes" | "io-net" | "siliconflow" | "cortecs"
        | "moark" | "berget" | "inference" | "privatemode-ai" | "synthetic" | "venice"
        | "vivgrid" | "siliconflow-cn" => ProviderCategory::Inference,

        // Gateway (12): Routes to other providers
        "openrouter"
        | "helicone"
        | "requesty"
        | "302ai"
        | "aihubmix"
        | "cloudflare-ai-gateway"
        | "fastrouter"
        | "zenmux"
        | "submodel"
        | "vercel"
        | "nano-gpt"
        | "poe" => ProviderCategory::Gateway,

        // Tool (16): Dev tools/platforms wrapping model access
        "github-copilot" | "github-models" | "gitlab" | "v0" | "huggingface" | "lmstudio"
        | "ollama-cloud" | "wandb" | "morph" | "opencode" | "firmware" | "kimi-for-coding"
        | "modelscope" | "abacus" | "iflowcn" | "zai" => ProviderCategory::Tool,

        // Unknown/new providers default to Inference
        _ => ProviderCategory::Inference,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_providers() {
        assert_eq!(provider_category("anthropic"), ProviderCategory::Origin);
        assert_eq!(provider_category("openai"), ProviderCategory::Origin);
        assert_eq!(provider_category("google"), ProviderCategory::Origin);
        assert_eq!(provider_category("deepseek"), ProviderCategory::Origin);
        assert_eq!(provider_category("alibaba-cn"), ProviderCategory::Origin);

        assert_eq!(provider_category("amazon-bedrock"), ProviderCategory::Cloud);
        assert_eq!(provider_category("azure"), ProviderCategory::Cloud);
        assert_eq!(provider_category("google-vertex"), ProviderCategory::Cloud);
        assert_eq!(provider_category("nvidia"), ProviderCategory::Cloud);

        assert_eq!(provider_category("deepinfra"), ProviderCategory::Inference);
        assert_eq!(provider_category("groq"), ProviderCategory::Inference);
        assert_eq!(provider_category("togetherai"), ProviderCategory::Inference);
        assert_eq!(provider_category("cerebras"), ProviderCategory::Inference);

        assert_eq!(provider_category("openrouter"), ProviderCategory::Gateway);
        assert_eq!(provider_category("helicone"), ProviderCategory::Gateway);
        assert_eq!(provider_category("vercel"), ProviderCategory::Gateway);

        assert_eq!(provider_category("github-copilot"), ProviderCategory::Tool);
        assert_eq!(provider_category("ollama-cloud"), ProviderCategory::Tool);
        assert_eq!(provider_category("huggingface"), ProviderCategory::Tool);
    }

    #[test]
    fn test_unknown_defaults_to_inference() {
        assert_eq!(
            provider_category("some-new-provider"),
            ProviderCategory::Inference
        );
        assert_eq!(
            provider_category("totally-unknown"),
            ProviderCategory::Inference
        );
    }

    #[test]
    fn test_cycle_behavior() {
        let start = ProviderCategory::All;
        assert_eq!(start.next(), ProviderCategory::Origin);
        assert_eq!(start.next().next(), ProviderCategory::Cloud);
        assert_eq!(start.next().next().next(), ProviderCategory::Inference);
        assert_eq!(start.next().next().next().next(), ProviderCategory::Gateway);
        assert_eq!(
            start.next().next().next().next().next(),
            ProviderCategory::Tool
        );
        assert_eq!(
            start.next().next().next().next().next().next(),
            ProviderCategory::All
        );
    }

    #[test]
    fn test_short_labels() {
        assert_eq!(ProviderCategory::All.short_label(), "All");
        assert_eq!(ProviderCategory::Origin.short_label(), "Orig");
        assert_eq!(ProviderCategory::Cloud.short_label(), "Cloud");
        assert_eq!(ProviderCategory::Inference.short_label(), "Infra");
        assert_eq!(ProviderCategory::Gateway.short_label(), "Gate");
        assert_eq!(ProviderCategory::Tool.short_label(), "Tool");
    }

    #[test]
    fn test_labels() {
        assert_eq!(ProviderCategory::Origin.label(), "Origin");
        assert_eq!(ProviderCategory::Cloud.label(), "Cloud Platform");
        assert_eq!(ProviderCategory::Inference.label(), "Inference");
        assert_eq!(ProviderCategory::Gateway.label(), "Gateway");
        assert_eq!(ProviderCategory::Tool.label(), "Dev Tool");
    }

    #[test]
    fn test_display_order() {
        assert!(ProviderCategory::Origin.display_order() < ProviderCategory::Cloud.display_order());
        assert!(
            ProviderCategory::Cloud.display_order() < ProviderCategory::Inference.display_order()
        );
        assert!(
            ProviderCategory::Inference.display_order() < ProviderCategory::Gateway.display_order()
        );
        assert!(ProviderCategory::Gateway.display_order() < ProviderCategory::Tool.display_order());
    }
}
