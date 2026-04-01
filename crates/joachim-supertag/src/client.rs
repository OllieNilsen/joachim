//! Bedrock client and top-level supertagger API.
//!
//! The [`Supertagger`] struct holds a reusable AWS Bedrock client.

use joachim_core::types::TypeAssignment;

use crate::error::SupertaggerError;
use crate::prompt::PROMPT_VERSION;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the supertagger client.
#[derive(Clone, Debug)]
pub struct SupertaggerConfig {
    /// Bedrock model ID.
    pub model_id: String,
    /// AWS region for Bedrock.
    pub region: String,
    /// Maximum tokens in the LLM response.
    pub max_tokens: u32,
    /// Request timeout.
    pub timeout: std::time::Duration,
    /// Temperature (0.0 for deterministic output).
    pub temperature: f32,
}

impl Default for SupertaggerConfig {
    fn default() -> Self {
        Self {
            model_id: "anthropic.claude-sonnet-4-20250514".into(),
            region: "us-east-1".into(),
            max_tokens: 1024,
            timeout: std::time::Duration::from_secs(30),
            temperature: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Successful supertagger output.
#[derive(Clone, Debug)]
pub struct SupertaggerOutput {
    /// The type assignments produced by the LLM.
    pub assignments: Vec<TypeAssignment>,
    /// Which prompt version was used (for audit).
    pub prompt_version: &'static str,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum allowed input text length in characters.
pub const MAX_INPUT_LEN: usize = 10_000;

// ---------------------------------------------------------------------------
// Supertagger
// ---------------------------------------------------------------------------

/// Reusable LLM supertagger client.
///
/// Holds a pre-built AWS Bedrock client for connection reuse across calls.
pub struct Supertagger {
    client: aws_sdk_bedrockruntime::Client,
    config: SupertaggerConfig,
}

impl Supertagger {
    /// Create a new supertagger, resolving AWS credentials.
    ///
    /// # Errors
    ///
    /// Returns `SupertaggerError::BedrockError` if AWS config cannot be loaded.
    pub async fn new(config: SupertaggerConfig) -> Result<Self, SupertaggerError> {
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()))
            .load()
            .await;
        let client = aws_sdk_bedrockruntime::Client::new(&aws_config);
        Ok(Self { client, config })
    }

    /// Supertag a text: chunk it and assign pregroup types via LLM.
    ///
    /// # Errors
    ///
    /// Returns:
    /// - `InputTooLong` if `text.len() > MAX_INPUT_LEN`
    /// - `Timeout` if Bedrock doesn't respond in time
    /// - `BedrockError` on API failures
    /// - `JsonParseError` if the LLM response isn't valid JSON
    /// - `InvalidOutput` if the parsed output fails validation
    pub async fn supertag(&self, text: &str) -> Result<SupertaggerOutput, SupertaggerError> {
        // Empty input → empty output, no network call.
        if text.is_empty() {
            return Ok(SupertaggerOutput {
                assignments: Vec::new(),
                prompt_version: PROMPT_VERSION,
            });
        }

        // Length guard.
        if text.len() > MAX_INPUT_LEN {
            return Err(SupertaggerError::InputTooLong {
                limit: MAX_INPUT_LEN,
                actual: text.len(),
            });
        }

        // Build prompt.
        let system_prompt = crate::prompt::build_system_prompt();
        let user_message = crate::prompt::build_user_message(text);

        // Invoke model.
        let raw_response = self.invoke_model(system_prompt, &user_message).await?;

        // Parse and validate.
        let raw = crate::types::parse_response(&raw_response)?;
        let assignments = crate::types::convert_raw(raw)?;
        crate::types::validate_output(&assignments)?;

        Ok(SupertaggerOutput {
            assignments,
            prompt_version: PROMPT_VERSION,
        })
    }

    /// Invoke the Bedrock model with the given prompts.
    async fn invoke_model(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, SupertaggerError> {
        let body = serde_json::json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "system": system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": user_message
                }
            ]
        });

        let body_bytes = serde_json::to_vec(&body).map_err(|e| {
            SupertaggerError::BedrockError(format!("failed to serialize request: {e}"))
        })?;

        let invoke_future = self
            .client
            .invoke_model()
            .model_id(&self.config.model_id)
            .content_type("application/json")
            .body(aws_sdk_bedrockruntime::primitives::Blob::new(body_bytes))
            .send();

        let result = tokio::time::timeout(self.config.timeout, invoke_future)
            .await
            .map_err(|_| SupertaggerError::Timeout(self.config.timeout))?
            .map_err(|e| SupertaggerError::BedrockError(e.to_string()))?;

        let response_bytes = result.body().as_ref();
        let response_json: serde_json::Value =
            serde_json::from_slice(response_bytes).map_err(|e| {
                SupertaggerError::BedrockError(format!(
                    "failed to parse Bedrock response envelope: {e}"
                ))
            })?;

        // Extract the text content from Anthropic's response format.
        let text = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                SupertaggerError::BedrockError("Bedrock response missing content[0].text".into())
            })?;

        Ok(text.to_owned())
    }
}
