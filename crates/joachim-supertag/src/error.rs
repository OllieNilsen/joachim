//! Error types for the supertagger.

use std::time::Duration;

/// Errors that can occur during supertagging.
///
/// All failure modes are captured here — the supertagger never panics.
#[derive(Debug, thiserror::Error)]
pub enum SupertaggerError {
    /// AWS Bedrock request failed.
    #[error("Bedrock request failed: {0}")]
    BedrockError(String),

    /// LLM response was not valid JSON.
    #[error("LLM response was not valid JSON ({len}B response)")]
    JsonParseError {
        /// The raw response text (for debugging).
        raw: String,
        /// Length of the raw response.
        len: usize,
        /// The underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// LLM output failed validation.
    #[error("LLM output failed validation: {reason}")]
    InvalidOutput {
        /// What went wrong.
        reason: String,
        /// The raw response text (for debugging).
        raw: String,
    },

    /// Input text exceeds the maximum allowed length.
    #[error("Input text exceeds maximum length of {limit} chars (got {actual})")]
    InputTooLong {
        /// The configured limit.
        limit: usize,
        /// The actual input length.
        actual: usize,
    },

    /// Request timed out.
    #[error("Request timed out after {0:?}")]
    Timeout(Duration),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bedrock_error_display() {
        let e = SupertaggerError::BedrockError("connection refused".into());
        assert_eq!(e.to_string(), "Bedrock request failed: connection refused");
    }

    #[test]
    fn json_parse_error_shows_length_not_raw() {
        let raw = "x".repeat(5000);
        let serde_err = serde_json::from_str::<Vec<u8>>("not json").unwrap_err();
        let e = SupertaggerError::JsonParseError {
            len: raw.len(),
            raw,
            source: serde_err,
        };
        let display = e.to_string();
        assert!(display.contains("5000B response"), "got: {display}");
        assert!(
            !display.contains("xxxxx"),
            "raw content should not appear in Display"
        );
    }

    #[test]
    fn invalid_output_display() {
        let e = SupertaggerError::InvalidOutput {
            reason: "non-monotonic chunk_idx".into(),
            raw: String::new(),
        };
        assert!(e.to_string().contains("non-monotonic"));
    }

    #[test]
    fn input_too_long_display() {
        let e = SupertaggerError::InputTooLong {
            limit: 10_000,
            actual: 15_000,
        };
        let display = e.to_string();
        assert!(display.contains("10000"), "got: {display}");
        assert!(display.contains("15000"), "got: {display}");
    }

    #[test]
    fn timeout_display() {
        let e = SupertaggerError::Timeout(Duration::from_secs(30));
        assert!(e.to_string().contains("30s"));
    }
}
