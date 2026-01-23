//! LLM configuration and serdes-ai harness.

mod config;
mod error;

pub(crate) use config::LlmConfig;
pub(crate) use error::LlmError;

use crate::config::Migration;
use llm_coding_tools_serdesai::agent_ext::AgentBuilderExt;
use llm_coding_tools_serdesai::allowed::{EditTool, GlobTool, GrepTool, ReadTool};
use llm_coding_tools_serdesai::{AllowedPathResolver, BashTool, SystemPromptBuilder};
use serde::Deserialize;
use serdes_ai::{agent::Agent, agent::AgentBuilder};
use std::path::Path;
use std::sync::Arc;

const MODEL_ENV: &str = "TEMPLATE_UPGRADE_LLM_MODEL";
const TEMPERATURE_ENV: &str = "TEMPLATE_UPGRADE_LLM_TEMPERATURE";
const LLM_TIMEOUT_SECS: u64 = 3600; // 60 minutes

/// Top-level structure for `config.toml` with a single `[llm]` section.
#[derive(Debug, Clone, Deserialize)]
struct LlmConfigFile {
    /// LLM provider configuration.
    llm: LlmConfig,
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
    let config = load_config(config_path)?;
    let model = resolve_model(config.as_ref())?;
    let temperature = resolve_temperature(config.as_ref());
    let agent = build_agent(model, repo_path, temperature)?;
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
fn resolve_model(config: Option<&LlmConfig>) -> Result<Arc<dyn serdes_ai_models::Model>, LlmError> {
    if let Some(config) = config {
        return config.build_model();
    }
    let model_spec = std::env::var(MODEL_ENV).map_err(|_| LlmError::MissingModel)?;
    serdes_ai_models::infer_model(&model_spec).map_err(LlmError::Model)
}

/// Validates that a temperature value is finite and within 0.0-2.0.
fn validate_temperature(value: f64, source: &str) -> Option<f64> {
    if !value.is_finite() || !(0.0..=2.0).contains(&value) {
        tracing::warn!(
            "Invalid temperature {value} from {source}: must be finite and in range 0.0-2.0"
        );
        return None;
    }
    Some(value)
}

/// Resolves the temperature from environment or config.
///
/// Environment variable takes precedence over config file.
fn resolve_temperature(config: Option<&LlmConfig>) -> Option<f64> {
    if let Ok(val) = std::env::var(TEMPERATURE_ENV) {
        if let Ok(temp) = val.parse::<f64>() {
            return validate_temperature(temp, "environment variable");
        }
    }
    config
        .and_then(LlmConfig::temperature)
        .and_then(|t| validate_temperature(t, "config file"))
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
fn build_agent(
    model: Arc<dyn serdes_ai_models::Model>,
    path: &Path,
    temperature: Option<f64>,
) -> Result<Agent<(), String>, LlmError> {
    let resolver = AllowedPathResolver::new([path])?;
    let read = ReadTool::<true>::new(resolver.clone());
    let edit = EditTool::new(resolver.clone());
    let glob = GlobTool::new(resolver.clone());
    let grep = GrepTool::<true>::new(resolver);
    let bash = BashTool::new().with_default_workdir(path);

    let path_str = path.display().to_string();
    let mut prompt_builder = SystemPromptBuilder::new().working_directory(path_str);

    let mut builder = AgentBuilder::from_arc(model)
        .tool(prompt_builder.track(read))
        .tool(prompt_builder.track(edit))
        .tool(prompt_builder.track(glob))
        .tool(prompt_builder.track(grep))
        .tool(prompt_builder.track(bash))
        .system_prompt(prompt_builder.build());

    if let Some(temp) = temperature {
        builder = builder.temperature(temp);
    }

    Ok(builder.build())
}

/// Builds the migration prompt for the LLM.
fn build_prompt(migration: &Migration) -> String {
    let guide_line = migration
        .migration_guide_link
        .as_ref()
        .map(|g| format!("Migration guide: {g}\n"))
        .unwrap_or_default();

    format!(
        "Apply the template migration using the available tools.\n\
Target file: {target_file}\n\
Old string: {old_string}\n\
New string: {new_string}\n\
{guide_line}\
Steps:\n\
1) Use glob/grep to locate relevant files.\n\
2) Update occurrences of the old string to the new string.\n\
3) Keep changes minimal and confined to the repo.\n\
4) Do not commit or push any changes.\n\
5) Reply with a brief summary of edits.",
        target_file = migration.target_file,
        old_string = migration.old_string,
        new_string = migration.new_string,
    )
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
provider = "openai"
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
provider = "openrouter"
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

    #[test]
    fn load_config_parses_temperature() {
        let temp = TempDir::new().unwrap();
        let path = write_config(
            &temp,
            r#"
[llm]
provider = "openai"
model = "gpt-4o"
temperature = 0.5
"#,
        );
        let config = load_config(&path).unwrap().unwrap();
        assert_eq!(config.temperature(), Some(0.5));
    }

    #[test]
    fn load_config_temperature_defaults_to_none() {
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
        assert_eq!(config.temperature(), None);
    }

    #[test]
    fn resolve_temperature_returns_none_without_config_or_env() {
        temp_env::with_var_unset(TEMPERATURE_ENV, || {
            assert_eq!(resolve_temperature(None), None);
        });
    }

    #[test]
    fn resolve_temperature_uses_config_value() {
        temp_env::with_var_unset(TEMPERATURE_ENV, || {
            let config = LlmConfig::OpenAi {
                model: "gpt-4o".to_string(),
                api_key: None,
                base_url: None,
                timeout_secs: None,
                temperature: Some(0.3),
            };
            assert_eq!(resolve_temperature(Some(&config)), Some(0.3));
        });
    }

    #[test]
    fn resolve_temperature_prefers_env_over_config() {
        temp_env::with_var(TEMPERATURE_ENV, Some("0.8"), || {
            let config = LlmConfig::OpenAi {
                model: "gpt-4o".to_string(),
                api_key: None,
                base_url: None,
                timeout_secs: None,
                temperature: Some(0.3),
            };
            assert_eq!(resolve_temperature(Some(&config)), Some(0.8));
        });
    }
}
