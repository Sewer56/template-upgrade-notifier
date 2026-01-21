//! Rate limiting utilities for GitHub API.
//!
//! This module provides functions to check and wait for GitHub API rate limits,
//! respecting the Retry-After header and implementing exponential backoff.

use octocrab::Octocrab;
use std::time::Duration;
use tracing::{info, warn};

/// Maximum time to wait for rate limit reset (1 hour).
const MAX_WAIT_SECS: u64 = 3600;

/// Minimum remaining requests before proactively waiting.
const MIN_REMAINING_THRESHOLD: u32 = 5;

/// Rate limit information for a specific resource.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Requests remaining in the current window.
    pub remaining: u32,
    /// Unix timestamp when the rate limit resets.
    pub reset: u64,
    /// Total requests allowed per window.
    pub limit: u32,
}

/// Checks the current rate limit status for search API.
///
/// # Errors
///
/// Returns an error if the rate limit API call fails.
pub async fn check_search_rate_limit(
    octocrab: &Octocrab,
) -> Result<RateLimitInfo, octocrab::Error> {
    let rate_limit = octocrab.ratelimit().get().await?;
    let search = &rate_limit.resources.search;

    Ok(RateLimitInfo {
        remaining: search.remaining as u32,
        reset: search.reset,
        limit: search.limit as u32,
    })
}

/// Checks the current rate limit status for core API (issues, PRs, etc.).
///
/// # Errors
///
/// Returns an error if the rate limit API call fails.
pub async fn check_core_rate_limit(octocrab: &Octocrab) -> Result<RateLimitInfo, octocrab::Error> {
    let rate_limit = octocrab.ratelimit().get().await?;
    let core = &rate_limit.resources.core;

    Ok(RateLimitInfo {
        remaining: core.remaining as u32,
        reset: core.reset,
        limit: core.limit as u32,
    })
}

/// Waits if the rate limit is low, returning true if we waited.
///
/// This function proactively waits when remaining requests fall below
/// `MIN_REMAINING_THRESHOLD` to avoid hitting hard limits.
///
/// # Arguments
///
/// * `info` - Current rate limit information
///
/// # Returns
///
/// Returns `true` if we waited, `false` if no wait was needed.
pub async fn wait_if_needed(info: &RateLimitInfo) -> bool {
    if info.remaining >= MIN_REMAINING_THRESHOLD {
        return false;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if info.reset <= now {
        return false;
    }

    let wait_secs = info.reset - now;
    if wait_secs > MAX_WAIT_SECS {
        warn!(
            wait_secs,
            max_wait = MAX_WAIT_SECS,
            "Rate limit reset too far in future, capping wait time"
        );
    }

    let actual_wait = wait_secs.min(MAX_WAIT_SECS);
    info!(
        remaining = info.remaining,
        wait_secs = actual_wait,
        "Rate limit low, waiting for reset"
    );

    tokio::time::sleep(Duration::from_secs(actual_wait)).await;
    true
}

/// Waits for rate limit reset with a specific duration.
///
/// # Arguments
///
/// * `retry_after_secs` - Seconds to wait (from Retry-After header)
pub async fn wait_for_retry_after(retry_after_secs: u64) {
    let actual_wait = retry_after_secs.min(MAX_WAIT_SECS);
    info!(
        retry_after = retry_after_secs,
        actual_wait, "Received Retry-After header, waiting"
    );
    tokio::time::sleep(Duration::from_secs(actual_wait)).await;
}

/// Ensures sufficient rate limit before making search API calls.
///
/// This is a convenience function that combines checking and waiting.
///
/// # Errors
///
/// Returns an error if the rate limit check fails.
pub async fn ensure_search_rate_limit(octocrab: &Octocrab) -> Result<(), octocrab::Error> {
    let info = check_search_rate_limit(octocrab).await?;
    wait_if_needed(&info).await;
    Ok(())
}

/// Ensures sufficient rate limit before making core API calls.
///
/// # Errors
///
/// Returns an error if the rate limit check fails.
pub async fn ensure_core_rate_limit(octocrab: &Octocrab) -> Result<(), octocrab::Error> {
    let info = check_core_rate_limit(octocrab).await?;
    wait_if_needed(&info).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_info() {
        let info = RateLimitInfo {
            remaining: 10,
            reset: 1234567890,
            limit: 30,
        };

        assert_eq!(info.remaining, 10);
        assert_eq!(info.reset, 1234567890);
        assert_eq!(info.limit, 30);
    }

    #[tokio::test]
    async fn test_wait_if_needed_no_wait() {
        let info = RateLimitInfo {
            remaining: 100,
            reset: 0,
            limit: 1000,
        };

        let waited = wait_if_needed(&info).await;
        assert!(!waited);
    }

    #[tokio::test]
    async fn test_wait_if_needed_reset_passed() {
        let info = RateLimitInfo {
            remaining: 1,
            reset: 0, // Already passed
            limit: 30,
        };

        let waited = wait_if_needed(&info).await;
        assert!(!waited);
    }
}
