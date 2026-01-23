//! LLM provider configuration.

use crate::llm::error::LlmError;
use serdes_ai_models::{build_model_with_config, infer_model, openrouter::OpenRouterModel, Model};
use std::sync::Arc;

/// Provider-specific configuration parsed from `config.toml` for a single LLM provider.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub(crate) enum LlmConfig {
    /// OpenAI provider configuration.
    #[serde(rename = "openai")]
    OpenAi {
        /// Model name (e.g., "gpt-4o").
        model: String,
        /// API key (optional, falls back to OPENAI_API_KEY env var).
        api_key: Option<String>,
        /// Base URL (optional).
        #[serde(rename = "base-url")]
        base_url: Option<String>,
        /// Timeout in seconds (optional).
        #[serde(rename = "timeout-secs")]
        timeout_secs: Option<u64>,
        /// Sampling temperature (optional, 0.0-2.0).
        temperature: Option<f64>,
    },

    /// OpenRouter provider configuration.
    #[serde(rename = "openrouter")]
    OpenRouter {
        /// Model name (e.g., "anthropic/claude-3-opus").
        model: String,
        /// API key (optional, falls back to OPENROUTER_API_KEY env var).
        api_key: Option<String>,
        /// HTTP Referer header (optional).
        #[serde(rename = "http-referer")]
        http_referer: Option<String>,
        /// App title header (optional).
        #[serde(rename = "app-title")]
        app_title: Option<String>,
        /// Sampling temperature (optional, 0.0-2.0).
        temperature: Option<f64>,
    },

    /// Anthropic provider configuration.
    Anthropic {
        /// Model name (e.g., "claude-3-5-sonnet-20241022").
        model: String,
        /// API key (optional, falls back to ANTHROPIC_API_KEY env var).
        api_key: Option<String>,
        /// Base URL (optional).
        #[serde(rename = "base-url")]
        base_url: Option<String>,
        /// Timeout in seconds (optional).
        #[serde(rename = "timeout-secs")]
        timeout_secs: Option<u64>,
        /// Sampling temperature (optional, 0.0-2.0).
        temperature: Option<f64>,
    },

    /// Gemini provider configuration.
    Gemini {
        /// Model name (e.g., "gemini-2.0-flash").
        model: String,
        /// API key (optional, falls back to GOOGLE_API_KEY env var).
        api_key: Option<String>,
        /// Base URL (optional).
        #[serde(rename = "base-url")]
        base_url: Option<String>,
        /// Timeout in seconds (optional).
        #[serde(rename = "timeout-secs")]
        timeout_secs: Option<u64>,
        /// Sampling temperature (optional, 0.0-2.0).
        temperature: Option<f64>,
    },
}

impl LlmConfig {
    /// Returns the configured temperature, if any.
    pub(crate) fn temperature(&self) -> Option<f64> {
        match self {
            Self::OpenAi { temperature, .. }
            | Self::OpenRouter { temperature, .. }
            | Self::Anthropic { temperature, .. }
            | Self::Gemini { temperature, .. } => *temperature,
        }
    }

    /// Builds a model from the configuration.
    pub(crate) fn build_model(&self) -> Result<Arc<dyn Model>, LlmError> {
        match self {
            Self::OpenRouter {
                model,
                api_key,
                http_referer,
                app_title,
                ..
            } => {
                // Env vars take precedence over config values
                let resolved_key = std::env::var("OPENROUTER_API_KEY")
                    .ok()
                    .or_else(|| api_key.clone());
                let resolved_referer = std::env::var("OPENROUTER_HTTP_REFERER")
                    .ok()
                    .or_else(|| http_referer.clone());
                let resolved_title = std::env::var("OPENROUTER_APP_TITLE")
                    .ok()
                    .or_else(|| app_title.clone());

                if resolved_key.is_none() && resolved_referer.is_none() && resolved_title.is_none()
                {
                    let spec = format!("openrouter:{model}");
                    return infer_model(&spec).map_err(LlmError::Model);
                }
                let mut model = match resolved_key {
                    Some(key) => OpenRouterModel::new(model, &key),
                    None => OpenRouterModel::from_env(model).map_err(LlmError::Model)?,
                };
                if let Some(referer) = resolved_referer {
                    model = model.with_http_referer(&referer);
                }
                if let Some(title) = resolved_title {
                    model = model.with_app_title(&title);
                }
                Ok(Arc::new(model))
            }
            Self::OpenAi {
                model,
                api_key,
                base_url,
                timeout_secs,
                ..
            } => build_configured_model("openai", model, api_key, base_url, timeout_secs),
            Self::Anthropic {
                model,
                api_key,
                base_url,
                timeout_secs,
                ..
            } => build_configured_model("anthropic", model, api_key, base_url, timeout_secs),
            Self::Gemini {
                model,
                api_key,
                base_url,
                timeout_secs,
                ..
            } => build_configured_model("gemini", model, api_key, base_url, timeout_secs),
        }
    }
}

/// Builds a configured model for generic providers.
fn build_configured_model(
    provider: &str,
    model: &str,
    api_key: &Option<String>,
    base_url: &Option<String>,
    timeout_secs: &Option<u64>,
) -> Result<Arc<dyn Model>, LlmError> {
    // Env vars take precedence over config values
    let resolved_key = env_api_key(provider).or_else(|| api_key.as_deref().map(str::to_owned));
    let resolved_base_url = env_base_url(provider).or_else(|| base_url.clone());
    let resolved_timeout_secs = env_timeout_secs(provider).or(*timeout_secs);
    let timeout = resolved_timeout_secs.map(core::time::Duration::from_secs);

    if resolved_key.is_none() && resolved_base_url.is_none() && resolved_timeout_secs.is_none() {
        let spec = format!("{provider}:{model}");
        return infer_model(&spec).map_err(LlmError::Model);
    }
    build_model_with_config(
        provider,
        model,
        resolved_key.as_deref(),
        resolved_base_url.as_deref(),
        timeout,
    )
    .map_err(LlmError::Model)
}

/// Gets the API key from environment variables for a provider.
fn env_api_key(provider: &str) -> Option<String> {
    let var = match provider {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "gemini" => "GOOGLE_API_KEY",
        _ => return None,
    };
    std::env::var(var).ok()
}

/// Gets the base URL from environment variables for a provider.
fn env_base_url(provider: &str) -> Option<String> {
    let var = match provider {
        "openai" => "OPENAI_BASE_URL",
        "anthropic" => "ANTHROPIC_BASE_URL",
        "gemini" => "GEMINI_BASE_URL",
        _ => return None,
    };
    std::env::var(var).ok()
}

/// Gets the timeout from environment variables for a provider.
fn env_timeout_secs(provider: &str) -> Option<u64> {
    let var = match provider {
        "openai" => "OPENAI_TIMEOUT_SECS",
        "anthropic" => "ANTHROPIC_TIMEOUT_SECS",
        "gemini" => "GEMINI_TIMEOUT_SECS",
        _ => return None,
    };
    std::env::var(var).ok()?.parse().ok()
}
