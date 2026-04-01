## ADDED Requirements

### Requirement: Supertagger struct with reusable client
The supertagger SHALL be a struct holding a pre-built Bedrock client and configuration:
```rust
pub struct Supertagger {
    client: BedrockRuntimeClient,
    config: SupertaggerConfig,
}
```

Construction is async (credential resolution may be async). The struct is reusable across multiple calls.

#### Scenario: Construct once, call many
- **WHEN** creating a `Supertagger` and calling `supertag()` twice
- **THEN** both calls SHALL reuse the same Bedrock client (no reconstruction)

### Requirement: Supertag method signature
The supertagger SHALL expose an async method:
```rust
impl Supertagger {
    pub async fn new(config: SupertaggerConfig) -> Result<Self, SupertaggerError>;
    pub async fn supertag(&self, text: &str) -> Result<SupertaggerOutput, SupertaggerError>;
}
```

Where `SupertaggerOutput` contains `assignments: Vec<TypeAssignment>` and `prompt_version: &'static str`.

#### Scenario: Successful supertagging
- **WHEN** given valid input text and working Bedrock credentials
- **THEN** the method SHALL return `Ok(SupertaggerOutput)` with valid `Vec<TypeAssignment>`

#### Scenario: Empty input
- **WHEN** given an empty string
- **THEN** the method SHALL return `Ok` with an empty `Vec<TypeAssignment>` (no Bedrock call)

### Requirement: JSON extraction from LLM response
Before deserialization, the client SHALL extract the JSON array from the raw LLM response:
1. Strip leading/trailing whitespace.
2. If markdown fences are present (`` ```json ... ``` `` or `` ``` ... ``` ``), extract content between them.
3. Find the first `[` and last `]` to locate the JSON array bounds.

#### Scenario: Clean JSON response
- **WHEN** the LLM returns `[{"chunk_idx": 0, ...}]`
- **THEN** extraction SHALL pass through unchanged

#### Scenario: Markdown-fenced response
- **WHEN** the LLM returns `` ```json\n[...]\n``` ``
- **THEN** extraction SHALL unwrap the fences and parse the inner JSON

#### Scenario: Preamble before JSON
- **WHEN** the LLM returns "Here is the analysis:\n[...]"
- **THEN** extraction SHALL find the `[` and `]` bounds and parse the array

### Requirement: JSON deserialization via intermediate types
The client SHALL deserialize the extracted JSON into `Vec<RawChunkAssignment>` (with `String` fields for `base` and `voiding`), then convert to `Vec<TypeAssignment>` via a `convert_raw()` step that maps strings to enums.

#### Scenario: Valid JSON parsed
- **WHEN** the LLM returns valid JSON matching the schema
- **THEN** the client SHALL produce valid `Vec<TypeAssignment>` values

#### Scenario: Malformed JSON
- **WHEN** the LLM returns content that cannot be parsed as JSON even after extraction
- **THEN** the client SHALL return `SupertaggerError::JsonParseError` with the response length

#### Scenario: Unknown base type
- **WHEN** the LLM returns a base type string like "Foo" that doesn't map to any TypeId
- **THEN** the client SHALL return `SupertaggerError::InvalidOutput`

#### Scenario: Unknown voiding kind
- **WHEN** the LLM returns a voiding string like "Conditional" that doesn't map to any VoidingKind
- **THEN** the client SHALL return `SupertaggerError::InvalidOutput`

### Requirement: Output validation
After deserialization and conversion, the client SHALL validate:
1. `chunk_idx` values are monotonically non-decreasing.
2. No `type_expr` is empty.
3. All `base` values are valid `TypeId` variants (checked during conversion).
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
All errors SHALL be captured in a `SupertaggerError` enum using `thiserror`. `Display` output SHALL NOT dump the entire raw LLM response — use response length or truncation instead. The raw response SHALL be available as a field for programmatic access.

#### Scenario: Error is displayable
- **WHEN** a `SupertaggerError` occurs
- **THEN** its `Display` SHALL produce a concise human-readable message (not kilobytes of raw JSON)

### Requirement: No panics
The supertagger client SHALL never panic. All failure modes are captured as `SupertaggerError` variants.

#### Scenario: Garbage input from LLM
- **WHEN** the LLM returns completely unparseable output
- **THEN** the client SHALL return an error, not panic
