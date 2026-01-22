//! LLM configuration and serdes-ai harness.

use crate::config::Migration;
use llm_coding_tools_serdesai::agent_ext::AgentBuilderExt;
use llm_coding_tools_serdesai::allowed::{EditTool, GlobTool, GrepTool, ReadTool};
use llm_coding_tools_serdesai::{AllowedPathResolver, BashTool, SystemPromptBuilder};
use serde::Deserialize;
use serdes_ai::{agent::AgentBuildError, agent::AgentBuilder, agent::AgentRunError, Agent};
use serdes_ai_models::{build_model_with_config, infer_model, openrouter::OpenRouterModel, Model};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

const MODEL_ENV: &str = "TEMPLATE_UPGRADE_LLM_MODEL";
const LLM_TIMEOUT_SECS: u64 = 600;

/// Provider-specific configuration parsed from `config.toml` for a single LLM provider.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub(crate) enum LlmConfig {
    /// OpenAI provider configuration.
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
    },
    /// OpenRouter provider configuration.
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
    },
}

/// Top-level structure for `config.toml` with a single `[llm]` section.
#[derive(Debug, Clone, Deserialize)]
struct LlmConfigFile {
    /// LLM provider configuration.
    llm: LlmConfig,
}

/// Internal error type for config loading and agent execution.
#[derive(Debug, Error)]
pub(crate) enum LlmError {
    /// Failed to read LLM config file.
    #[error("Failed to read LLM config '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// Failed to parse LLM config file.
    #[error("Failed to parse LLM config '{path}': {source}")]
    Toml {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    /// LLM model not configured.
    #[error("LLM model not configured; set TEMPLATE_UPGRADE_LLM_MODEL or config.toml")]
    MissingModel,
    /// LLM operation timed out.
    #[error("LLM timed out after {0} seconds")]
    Timeout(u64),
    /// Model error.
    #[error("Model error: {0}")]
    Model(#[from] serdes_ai_models::ModelError),
    /// Tool setup error.
    #[error("Tool setup error: {0}")]
    Tool(#[from] llm_coding_tools_serdesai::ToolError),
    /// Agent build error.
    #[error("Agent build error: {0}")]
    AgentBuild(#[from] AgentBuildError),
    /// Agent run error.
    #[error("Agent run error: {0}")]
    AgentRun(#[from] AgentRunError),
}

/// Applies a template migration using serdes-ai with coding tools.
///
/// # Arguments
///
/// * `repo_path` - Path to the cloned repository
/// * `config_path` - Path to the LLM config.toml file
/// * `migration` - Migration to apply
///
/// # Returns
///
/// Ok(()) if successful, Err(LlmError) on failure.
pub(crate) async fn apply_migration(
    repo_path: &Path,
    config_path: &Path,
    migration: &Migration,
) -> Result<(), LlmError> {
    let model = resolve_model(config_path)?;
    let agent = build_agent(model, repo_path)?;
    let prompt = build_prompt(migration);

    tokio::time::timeout(
        tokio::time::Duration::from_secs(LLM_TIMEOUT_SECS),
        agent.run(prompt, ()),
    )
    .await
    .map_err(|_| LlmError::Timeout(LLM_TIMEOUT_SECS))?
    .map(|_| ())
    .map_err(LlmError::from)
}

/// Resolves the LLM model from config or environment.
fn resolve_model(config_path: &Path) -> Result<Arc<dyn Model>, LlmError> {
    if let Some(config) = load_config(config_path)? {
        return config.build_model();
    }
    let model_spec = std::env::var(MODEL_ENV).map_err(|_| LlmError::MissingModel)?;
    infer_model(&model_spec).map_err(LlmError::Model)
}

/// Loads the LLM config file if it exists.
fn load_config(path: &Path) -> Result<Option<LlmConfig>, LlmError> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path).map_err(|source| LlmError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let parsed: LlmConfigFile = toml::from_str(&contents).map_err(|source| LlmError::Toml {
        path: path.display().to_string(),
        source,
    })?;
    Ok(Some(parsed.llm))
}

/// Builds an LLM agent with coding tools.
fn build_agent(model: Arc<dyn Model>, path: &Path) -> Result<Agent<(), String>, LlmError> {
    let resolver = AllowedPathResolver::new([path])?;
    let read = ReadTool::<true>::new(resolver.clone());
    let edit = EditTool::new(resolver.clone());
    let glob = GlobTool::new(resolver.clone());
    let grep = GrepTool::<true>::new(resolver);
    let bash = BashTool::new().with_default_workdir(path);

    let path_str = path.display().to_string();
    let mut prompt_builder = SystemPromptBuilder::new().working_directory(path_str);

    Ok(AgentBuilder::from_arc(model)
        .tool(prompt_builder.track(read))
        .tool(prompt_builder.track(edit))
        .tool(prompt_builder.track(glob))
        .tool(prompt_builder.track(grep))
        .tool(prompt_builder.track(bash))
        .system_prompt(prompt_builder.build())
        .temperature(0.2)
        .build())
}

/// Builds the migration prompt for the LLM.
fn build_prompt(migration: &Migration) -> String {
    format!(
        "Apply the template migration using the available tools.\n\
Target file: {target_file}\n\
Old string: {old_string}\n\
New string: {new_string}\n\
Migration guide: {guide}\n\
\n\
Steps:\n\
1) Use glob/grep to locate relevant files.\n\
2) Update occurrences of the old string to the new string.\n\
3) Keep changes minimal and confined to the repo.\n\
4) Do not commit or push any changes.\n\
5) Reply with a brief summary of edits.",
        target_file = migration.target_file,
        old_string = migration.old_string,
        new_string = migration.new_string,
        guide = migration.migration_guide_link,
    )
}

impl LlmConfig {
    /// Builds a model from the configuration.
    fn build_model(&self) -> Result<Arc<dyn Model>, LlmError> {
        match self {
            Self::OpenRouter {
                model,
                api_key,
                http_referer,
                app_title,
            } => {
                if api_key.is_none() && http_referer.is_none() && app_title.is_none() {
                    let spec = format!("openrouter:{model}");
                    return infer_model(&spec).map_err(LlmError::Model);
                }
                let mut model = match api_key {
                    Some(key) => OpenRouterModel::new(model, key),
                    None => OpenRouterModel::from_env(model).map_err(LlmError::Model)?,
                };
                if let Some(referer) = http_referer {
                    model = model.with_http_referer(referer);
                }
                if let Some(title) = app_title {
                    model = model.with_app_title(title);
                }
                Ok(Arc::new(model))
            }
            Self::OpenAi {
                model,
                api_key,
                base_url,
                timeout_secs,
            } => build_configured_model("openai", model, api_key, base_url, timeout_secs),
            Self::Anthropic {
                model,
                api_key,
                base_url,
                timeout_secs,
            } => build_configured_model("anthropic", model, api_key, base_url, timeout_secs),
            Self::Gemini {
                model,
                api_key,
                base_url,
                timeout_secs,
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
    let resolved_key = api_key
        .as_deref()
        .map(str::to_owned)
        .or_else(|| env_api_key(provider));
    let timeout = timeout_secs.map(core::time::Duration::from_secs);
    if resolved_key.is_none() && base_url.is_none() && timeout_secs.is_none() {
        let spec = format!("{provider}:{model}");
        return infer_model(&spec).map_err(LlmError::Model);
    }
    build_model_with_config(
        provider,
        model,
        resolved_key.as_deref(),
        base_url.as_deref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_config(temp: &TempDir, contents: &str) -> std::path::PathBuf {
        let path = temp.path().join("config.toml");
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn load_config_returns_none_when_missing() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("missing.toml");
        let config = load_config(&path).unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn load_config_parses_openai() {
        let temp = TempDir::new().unwrap();
        let path = write_config(
            &temp,
            r#"
[llm]
provider = "open_ai"
model = "gpt-4o"
base-url = "https://api.openai.com/v1"
timeout-secs = 30
"#,
        );
        let config = load_config(&path).unwrap().unwrap();
        match config {
            LlmConfig::OpenAi {
                model,
                base_url,
                timeout_secs,
                ..
            } => {
                assert_eq!(model, "gpt-4o");
                assert_eq!(base_url.as_deref(), Some("https://api.openai.com/v1"));
                assert_eq!(timeout_secs, Some(30));
            }
            _ => panic!("expected openai"),
        }
    }

    #[test]
    fn load_config_parses_openrouter() {
        let temp = TempDir::new().unwrap();
        let path = write_config(
            &temp,
            r#"
[llm]
provider = "open_router"
model = "anthropic/claude-3-opus"
http-referer = "https://example.com"
app-title = "Template Upgrade Notifier"
"#,
        );
        let config = load_config(&path).unwrap().unwrap();
        match config {
            LlmConfig::OpenRouter {
                model,
                http_referer,
                app_title,
                ..
            } => {
                assert_eq!(model, "anthropic/claude-3-opus");
                assert_eq!(http_referer.as_deref(), Some("https://example.com"));
                assert_eq!(app_title.as_deref(), Some("Template Upgrade Notifier"));
            }
            _ => panic!("expected openrouter"),
        }
    }

    #[test]
    fn load_config_parses_anthropic() {
        let temp = TempDir::new().unwrap();
        let path = write_config(
            &temp,
            r#"
[llm]
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
"#,
        );
        let config = load_config(&path).unwrap().unwrap();
        match config {
            LlmConfig::Anthropic { model, .. } => {
                assert_eq!(model, "claude-3-5-sonnet-20241022");
            }
            _ => panic!("expected anthropic"),
        }
    }

    #[test]
    fn load_config_parses_gemini() {
        let temp = TempDir::new().unwrap();
        let path = write_config(
            &temp,
            r#"
[llm]
provider = "gemini"
model = "gemini-2.0-flash"
"#,
        );
        let config = load_config(&path).unwrap().unwrap();
        match config {
            LlmConfig::Gemini { model, .. } => {
                assert_eq!(model, "gemini-2.0-flash");
            }
            _ => panic!("expected gemini"),
        }
    }

    #[test]
    fn load_config_reports_invalid_toml() {
        let temp = TempDir::new().unwrap();
        let path = write_config(&temp, "not = [valid");
        let error = load_config(&path).unwrap_err();
        assert!(matches!(error, LlmError::Toml { .. }));
    }
}
