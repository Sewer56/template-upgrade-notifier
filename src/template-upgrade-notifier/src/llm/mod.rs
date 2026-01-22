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
const LLM_TIMEOUT_SECS: u64 = 600;

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
fn resolve_model(config_path: &Path) -> Result<Arc<dyn serdes_ai_models::Model>, LlmError> {
    if let Some(config) = load_config(config_path)? {
        return config.build_model();
    }
    let model_spec = std::env::var(MODEL_ENV).map_err(|_| LlmError::MissingModel)?;
    serdes_ai_models::infer_model(&model_spec).map_err(LlmError::Model)
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
) -> Result<Agent<(), String>, LlmError> {
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
