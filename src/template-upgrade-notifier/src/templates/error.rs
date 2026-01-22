//! Template rendering error types.

/// Template rendering error.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// Handlebars rendering error.
    #[error("Template rendering error: {0}")]
    RenderError(#[from] handlebars::RenderError),

    /// Template registration error.
    #[error("Template registration error: {0}")]
    RegistrationError(#[from] handlebars::TemplateError),

    /// Invalid git branch name.
    #[error("Invalid branch name '{branch}': {reason}")]
    InvalidBranchName {
        /// The invalid branch name.
        branch: String,
        /// Reason for invalidity.
        reason: String,
    },
}
