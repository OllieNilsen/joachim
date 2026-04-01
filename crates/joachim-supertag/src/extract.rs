//! JSON extraction from LLM responses.
//!
//! Claude often wraps JSON in markdown fences or preamble text. This module
//! extracts the JSON array from such responses.

use crate::error::SupertaggerError;

/// Extract a JSON array from an LLM response that may contain wrapping.
///
/// Strategy:
/// 1. Strip leading/trailing whitespace.
/// 2. If markdown fences are present, extract content between them.
/// 3. If the result starts with `[`, return it directly.
/// 4. Otherwise, find the outermost `[` and `]` to locate the array bounds.
pub fn extract_json(response: &str) -> Result<&str, SupertaggerError> {
    let trimmed = response.trim();

    // Try markdown fence extraction.
    if let Some(inner) = extract_markdown_fenced(trimmed) {
        let inner = inner.trim();
        if inner.starts_with('[') {
            return Ok(inner);
        }
        // Fence content might still have preamble — fall through to bracket search.
        if let Some(extracted) = find_outermost_brackets(inner) {
            return Ok(extracted);
        }
    }

    // Already a JSON array?
    if trimmed.starts_with('[') {
        return Ok(trimmed);
    }

    // Find outermost [ and ].
    if let Some(extracted) = find_outermost_brackets(trimmed) {
        return Ok(extracted);
    }

    // Nothing found — construct a serde error for the JsonParseError.
    Err(make_parse_error(response))
}

/// Extract content between markdown fences (```json ... ``` or ``` ... ```).
fn extract_markdown_fenced(text: &str) -> Option<&str> {
    let start_markers = ["```json\n", "```json\r\n", "```\n", "```\r\n"];
    for marker in &start_markers {
        if let Some(start) = text.find(marker) {
            let content_start = start + marker.len();
            if let Some(end) = text[content_start..].find("```") {
                return Some(&text[content_start..content_start + end]);
            }
        }
    }
    None
}

/// Find the outermost matching `[` and `]` in the text.
fn find_outermost_brackets(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    if start < end {
        Some(&text[start..=end])
    } else {
        None
    }
}

/// Create a `JsonParseError` for when no JSON array can be found.
fn make_parse_error(raw: &str) -> SupertaggerError {
    // Force a serde error by trying to parse the raw text.
    let serde_err = serde_json::from_str::<serde_json::Value>(raw).unwrap_err();
    SupertaggerError::JsonParseError {
        len: raw.len(),
        raw: raw.to_owned(),
        source: serde_err,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_json_passes_through() {
        let input = r#"[{"chunk_idx": 0}]"#;
        assert_eq!(extract_json(input).unwrap(), input);
    }

    #[test]
    fn markdown_fenced_json_unwrapped() {
        let input = "```json\n[{\"chunk_idx\": 0}]\n```";
        assert_eq!(extract_json(input).unwrap(), "[{\"chunk_idx\": 0}]");
    }

    #[test]
    fn preamble_before_json_stripped() {
        let input = "Here is the analysis:\n[{\"chunk_idx\": 0}]";
        assert_eq!(extract_json(input).unwrap(), "[{\"chunk_idx\": 0}]");
    }

    #[test]
    fn no_brackets_returns_error() {
        let input = "This is not JSON at all";
        assert!(matches!(
            extract_json(input),
            Err(SupertaggerError::JsonParseError { .. })
        ));
    }
}
