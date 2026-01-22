//! LLM error types.

use thiserror::Error;

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
    AgentBuild(#[from] serdes_ai::agent::AgentBuildError),

    /// Agent run error.
    #[error("Agent run error: {0}")]
    AgentRun(#[from] serdes_ai::agent::AgentRunError),
}
