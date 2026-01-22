//! Rate limit information.

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
}
