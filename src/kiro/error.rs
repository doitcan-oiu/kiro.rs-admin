//! Shared typed errors for Kiro upstream calls.

/// The upstream still returned HTTP 429 after any applicable failover/retry.
#[derive(Debug, Clone, thiserror::Error)]
#[error("upstream rate limited")]
pub struct UpstreamRateLimitError {
    retry_after: Option<String>,
}

impl UpstreamRateLimitError {
    pub(crate) fn new(retry_after: Option<String>) -> Self {
        Self {
            retry_after: retry_after.and_then(normalize_retry_after),
        }
    }

    pub(crate) fn from_headers(headers: &http::HeaderMap) -> Self {
        let retry_after = headers
            .get(http::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        Self::new(retry_after)
    }

    pub fn retry_after(&self) -> Option<&str> {
        self.retry_after.as_deref()
    }
}

fn normalize_retry_after(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if value.parse::<u64>().is_ok() || httpdate::parse_http_date(value).is_ok() {
        Some(value.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_delta_seconds_and_http_date() {
        let seconds = UpstreamRateLimitError::new(Some(" 1800 ".to_string()));
        assert_eq!(seconds.retry_after(), Some("1800"));

        let date = "Sun, 12 Jul 2026 02:30:00 GMT";
        let http_date = UpstreamRateLimitError::new(Some(date.to_string()));
        assert_eq!(http_date.retry_after(), Some(date));
    }

    #[test]
    fn rejects_invalid_retry_after() {
        let error = UpstreamRateLimitError::new(Some("not-a-retry-delay".to_string()));
        assert_eq!(error.retry_after(), None);
    }
}
