//! Repository discovery using GitHub Code Search API.
//!
//! This module provides functions to discover repositories containing
//! outdated template versions using GitHub's code search.

use crate::config::Migration;
use crate::rate_limit::ensure_search_rate_limit;
use octocrab::Octocrab;
use serde::Serialize;
use std::collections::HashSet;
use thiserror::Error;
use tracing::{debug, info, info_span, warn, Instrument};

/// Errors that can occur during repository discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// GitHub API error.
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, reset at {reset_at}")]
    RateLimitExceeded { reset_at: u64 },
}

/// A repository discovered to contain an outdated template version.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredRepository {
    /// Repository owner (user or organization).
    pub owner: String,

    /// Repository name.
    pub name: String,

    /// Full repository name in "owner/name" format.
    pub full_name: String,

    /// Path to the file containing the match.
    pub file_path: String,

    /// GitHub URL to the matched file.
    pub file_url: String,

    /// Default branch name (e.g., "main").
    pub default_branch: String,
}

/// Maximum results from GitHub Code Search API.
const MAX_SEARCH_RESULTS: usize = 1000;

/// Results per page for code search.
const RESULTS_PER_PAGE: u8 = 100;

/// Discovers repositories containing the outdated template version.
///
/// Uses GitHub Code Search API to find repositories containing the
/// `old_string` in the `target_file`.
///
/// # Arguments
///
/// * `octocrab` - Authenticated GitHub client
/// * `migration` - Migration to search for
///
/// # Returns
///
/// A vector of discovered repositories, deduplicated by full_name.
///
/// # Errors
///
/// Returns [`DiscoveryError`] if the search fails.
pub async fn discover_repositories(
    octocrab: &Octocrab,
    migration: &Migration,
) -> Result<Vec<DiscoveredRepository>, DiscoveryError> {
    let span = info_span!(
        "discover",
        migration_id = %migration.id,
        old_string = %migration.old_string,
        target_file = %migration.target_file
    );

    async {
        info!("Starting repository discovery");

        // Ensure we have rate limit capacity
        ensure_search_rate_limit(octocrab).await?;

        // Build search query
        let query = build_search_query(&migration.old_string, &migration.target_file);
        debug!(query = %query, "Executing code search");

        // Execute search with pagination
        let results = execute_code_search(octocrab, &query).await?;

        // Deduplicate results
        let repositories = deduplicate_results(results);

        info!(count = repositories.len(), "Discovery complete");
        Ok(repositories)
    }
    .instrument(span)
    .await
}

/// Builds a GitHub code search query.
///
/// Format: `"{old_string}" in:file filename:{target_file}`
fn build_search_query(old_string: &str, target_file: &str) -> String {
    format!("\"{}\" in:file filename:{}", old_string, target_file)
}

/// Executes the code search with pagination.
async fn execute_code_search(
    octocrab: &Octocrab,
    query: &str,
) -> Result<Vec<CodeSearchResult>, DiscoveryError> {
    let mut all_results = Vec::new();

    // Get first page
    let mut page = octocrab
        .search()
        .code(query)
        .per_page(RESULTS_PER_PAGE)
        .send()
        .await?;

    // Extract results from first page
    all_results.extend(extract_search_results(&page));

    // Paginate through remaining results
    while let Some(next_page) = octocrab
        .get_page::<octocrab::models::Code>(&page.next)
        .await?
    {
        if all_results.len() >= MAX_SEARCH_RESULTS {
            warn!(
                max = MAX_SEARCH_RESULTS,
                "Reached maximum search results limit"
            );
            break;
        }

        // Check rate limit before next page
        ensure_search_rate_limit(octocrab).await?;

        all_results.extend(extract_page_results(&next_page));
        page.next = next_page.next;

        if page.next.is_none() {
            break;
        }
    }

    Ok(all_results)
}

/// Intermediate search result before deduplication.
struct CodeSearchResult {
    owner: String,
    name: String,
    full_name: String,
    file_path: String,
    file_url: String,
}

/// Extracts search results from a search response page.
fn extract_search_results(page: &octocrab::Page<octocrab::models::Code>) -> Vec<CodeSearchResult> {
    page.items
        .iter()
        .filter_map(|item| {
            let repo = &item.repository;
            let owner = repo.owner.as_ref()?.login.clone();
            let name = repo.name.clone();
            let full_name = format!("{}/{}", owner, name);

            Some(CodeSearchResult {
                owner,
                name,
                full_name,
                file_path: item.path.clone(),
                file_url: item.html_url.to_string(),
            })
        })
        .collect()
}

/// Extracts search results from a raw page response.
fn extract_page_results(page: &octocrab::Page<octocrab::models::Code>) -> Vec<CodeSearchResult> {
    extract_search_results(page)
}

/// Deduplicates search results by repository full_name.
///
/// If multiple files match in the same repository, only the first is kept.
fn deduplicate_results(results: Vec<CodeSearchResult>) -> Vec<DiscoveredRepository> {
    let mut seen = HashSet::new();
    let mut repositories = Vec::new();

    for result in results {
        if seen.insert(result.full_name.clone()) {
            repositories.push(DiscoveredRepository {
                owner: result.owner,
                name: result.name,
                full_name: result.full_name,
                file_path: result.file_path,
                file_url: result.file_url,
                // Default branch will be fetched separately if needed
                default_branch: "main".to_string(),
            });
        }
    }

    repositories
}

/// Fetches the default branch for a repository.
///
/// # Arguments
///
/// * `octocrab` - Authenticated GitHub client
/// * `owner` - Repository owner
/// * `repo` - Repository name
///
/// # Returns
///
/// The default branch name (e.g., "main" or "master").
///
/// # Errors
///
/// Returns an error if the repository info cannot be fetched.
pub async fn get_default_branch(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<String, DiscoveryError> {
    let repo_info = octocrab.repos(owner, repo).get().await?;
    Ok(repo_info
        .default_branch
        .unwrap_or_else(|| "main".to_string()))
}

/// Enriches discovered repositories with default branch information.
///
/// This makes additional API calls to fetch the default branch for each repository.
/// Use sparingly to avoid rate limiting.
pub async fn enrich_with_default_branches(
    octocrab: &Octocrab,
    repositories: &mut [DiscoveredRepository],
) -> Result<(), DiscoveryError> {
    for repo in repositories.iter_mut() {
        match get_default_branch(octocrab, &repo.owner, &repo.name).await {
            Ok(branch) => repo.default_branch = branch,
            Err(e) => {
                warn!(
                    repo = %repo.full_name,
                    error = %e,
                    "Failed to get default branch, using 'main'"
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_query() {
        let query = build_search_query("my-template:1.0.0", "version.txt");
        assert_eq!(query, "\"my-template:1.0.0\" in:file filename:version.txt");
    }

    #[test]
    fn test_deduplicate_results() {
        let results = vec![
            CodeSearchResult {
                owner: "user".to_string(),
                name: "repo".to_string(),
                full_name: "user/repo".to_string(),
                file_path: "file1.txt".to_string(),
                file_url: "https://github.com/user/repo/file1.txt".to_string(),
            },
            CodeSearchResult {
                owner: "user".to_string(),
                name: "repo".to_string(),
                full_name: "user/repo".to_string(),
                file_path: "file2.txt".to_string(),
                file_url: "https://github.com/user/repo/file2.txt".to_string(),
            },
            CodeSearchResult {
                owner: "other".to_string(),
                name: "project".to_string(),
                full_name: "other/project".to_string(),
                file_path: "version.txt".to_string(),
                file_url: "https://github.com/other/project/version.txt".to_string(),
            },
        ];

        let deduped = deduplicate_results(results);

        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].full_name, "user/repo");
        assert_eq!(deduped[1].full_name, "other/project");
    }
}
