## ADDED Requirements

### Requirement: Supertag function signature
The supertagger SHALL expose an async function:
```rust
pub async fn supertag(text: &str, config: &SupertaggerConfig) -> Result<SupertaggerOutput, SupertaggerError>
```

Where `SupertaggerOutput` contains `assignments: Vec<TypeAssignment>` and `prompt_version: &'static str`.

#### Scenario: Successful supertagging
- **WHEN** given valid input text and working Bedrock credentials
- **THEN** the function SHALL return `Ok(SupertaggerOutput)` with valid `Vec<TypeAssignment>`

#### Scenario: Empty input
- **WHEN** given an empty string
- **THEN** the function SHALL return `Ok` with an empty `Vec<TypeAssignment>` (no Bedrock call)

### Requirement: JSON deserialization
The client SHALL deserialize the LLM's JSON response into a `Vec<RawChunkAssignment>` intermediate type, then convert to `Vec<TypeAssignment>`.

The intermediate type includes `chunk_text: String` for debugging. This field is stripped during conversion to `TypeAssignment`.

#### Scenario: Valid JSON parsed
- **WHEN** the LLM returns valid JSON matching the schema
- **THEN** the client SHALL produce valid `Vec<TypeAssignment>` values

#### Scenario: Malformed JSON
- **WHEN** the LLM returns invalid JSON
- **THEN** the client SHALL return `SupertaggerError::JsonParseError` with the raw response

### Requirement: Output validation
After deserialization, the client SHALL validate:
1. `chunk_idx` values are monotonically non-decreasing.
2. No `type_expr` is empty.
3. All `base` values are valid `TypeId` variants.
4. `adjoint` values are in range `[-5, 5]`.
5. At least one chunk is present (for non-empty input).

#### Scenario: Valid output passes
- **WHEN** the LLM returns correctly structured assignments
- **THEN** validation SHALL pass

#### Scenario: Non-monotonic chunk_idx rejected
- **WHEN** the LLM returns chunk indices `[0, 2, 1]`
- **THEN** the client SHALL return `SupertaggerError::InvalidOutput`

#### Scenario: Adjoint out of range rejected
- **WHEN** the LLM returns an adjoint value of 50
- **THEN** the client SHALL return `SupertaggerError::InvalidOutput`

### Requirement: Bedrock integration
The client SHALL use `aws-sdk-bedrockruntime` to invoke the model via `InvokeModel` (non-streaming).

#### Scenario: Successful Bedrock call
- **WHEN** credentials are valid and the model is available
- **THEN** the client SHALL send the request and receive a response

#### Scenario: Bedrock error
- **WHEN** the Bedrock API returns an error (credentials, throttling, model not found)
- **THEN** the client SHALL return `SupertaggerError::BedrockError`

### Requirement: Request timeout
The client SHALL enforce a configurable timeout on the Bedrock call.

#### Scenario: Within timeout
- **WHEN** the LLM responds within the configured timeout
- **THEN** processing continues normally

#### Scenario: Timeout exceeded
- **WHEN** the LLM does not respond within the configured timeout
- **THEN** the client SHALL return `SupertaggerError::Timeout`

### Requirement: Configuration
The client SHALL accept a `SupertaggerConfig` with model_id, region, max_tokens, timeout, and temperature.

#### Scenario: Default configuration
- **WHEN** using `SupertaggerConfig::default()`
- **THEN** model SHALL be `anthropic.claude-sonnet-4-20250514`, region `us-east-1`, max_tokens 4096, timeout 30s, temperature 0.0

### Requirement: Error types
All errors SHALL be captured in a `SupertaggerError` enum using `thiserror`. Each variant SHALL preserve enough context for debugging (raw response text where applicable).

#### Scenario: Error is displayable
- **WHEN** a `SupertaggerError` occurs
- **THEN** its `Display` implementation SHALL produce a human-readable message

### Requirement: No panics
The supertagger client SHALL never panic. All failure modes are captured as `SupertaggerError` variants.

#### Scenario: Garbage input from LLM
- **WHEN** the LLM returns completely unparseable output
- **THEN** the client SHALL return an error, not panic
