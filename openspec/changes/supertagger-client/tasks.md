## 1. Project Setup

- [ ] 1.1 Create `crates/joachim-supertag/Cargo.toml`: depend on `joachim-core` (with `serde` feature), `aws-sdk-bedrockruntime`, `aws-config`, `serde`, `serde_json`, `tokio`, `thiserror`
- [ ] 1.2 Add `joachim-supertag` to workspace members in root `Cargo.toml`
- [ ] 1.3 Set up module structure: `lib.rs`, `prompt.rs`, `client.rs`, `types.rs`, `error.rs`

## 2. Error Types

- [ ] 2.1 Implement `SupertaggerError` enum with `thiserror`: `BedrockError(String)`, `JsonParseError { raw: String, source: serde_json::Error }`, `InvalidOutput { reason: String, raw: String }`, `Timeout(Duration)`
- [ ] 2.2 Write unit tests: each variant displays a human-readable message

## 3. Configuration

- [ ] 3.1 Implement `SupertaggerConfig { model_id, region, max_tokens, timeout, temperature }`
- [ ] 3.2 Implement `Default` for `SupertaggerConfig`: `anthropic.claude-sonnet-4-20250514`, `us-east-1`, 4096, 30s, 0.0

## 4. Prompt Template

- [ ] 4.1 Create `prompt.rs` with `pub const PROMPT_VERSION: &str = "v1"`
- [ ] 4.2 Write system prompt: type inventory (9 primitives), functional modifier patterns, VoidingKind descriptions, JSON output schema
- [ ] 4.3 Add 4-6 few-shot examples from ground truth: inj_001 (direct override), inj_004 (role hijack), ben_009 (hypothetical voiding), ben_015 (user-domain directive), ben_018 (negation voiding)
- [ ] 4.4 Implement `build_system_prompt() -> &'static str` returning the complete system prompt
- [ ] 4.5 Implement `build_user_message(text: &str) -> String` returning just the input text
- [ ] 4.6 Write unit test: system prompt contains all 9 TypeId names
- [ ] 4.7 Write unit test: system prompt contains VoidingKind names
- [ ] 4.8 Write unit test: system prompt contains JSON schema example

## 5. Intermediate Types and Parsing

- [ ] 5.1 Define `RawChunkAssignment { chunk_idx: u16, chunk_text: String, type_expr: Vec<RawSimpleType>, voiding: Option<String> }` with serde derives
- [ ] 5.2 Define `RawSimpleType { base: String, adjoint: i8 }` with serde derives
- [ ] 5.3 Implement `parse_response(json: &str) -> Result<Vec<RawChunkAssignment>, SupertaggerError>`: deserialize JSON, return `JsonParseError` on failure
- [ ] 5.4 Implement `convert_raw(raw: Vec<RawChunkAssignment>) -> Result<Vec<TypeAssignment>, SupertaggerError>`: convert `base` strings to `TypeId`, `voiding` strings to `VoidingKind`, strip `chunk_text`
- [ ] 5.5 Write unit test: valid JSON round-trips correctly
- [ ] 5.6 Write unit test: malformed JSON returns `JsonParseError`
- [ ] 5.7 Write unit test: unknown base type returns `InvalidOutput`
- [ ] 5.8 Write unit test: unknown voiding kind returns `InvalidOutput`

## 6. Output Validation

- [ ] 6.1 Implement `validate_output(assignments: &[TypeAssignment]) -> Result<(), SupertaggerError>`: monotonic chunk_idx, non-empty type_exprs, adjoint in [-5, 5], at least one chunk
- [ ] 6.2 Write unit test: valid output passes
- [ ] 6.3 Write unit test: non-monotonic chunk_idx rejected
- [ ] 6.4 Write unit test: empty type_expr rejected
- [ ] 6.5 Write unit test: adjoint out of range rejected
- [ ] 6.6 Write unit test: empty assignment list for non-empty input rejected

## 7. Bedrock Client

- [ ] 7.1 Implement `build_bedrock_client(config: &SupertaggerConfig) -> Result<BedrockRuntimeClient, SupertaggerError>`: create AWS client with region from config
- [ ] 7.2 Implement `invoke_model(client: &BedrockRuntimeClient, config: &SupertaggerConfig, system_prompt: &str, user_message: &str) -> Result<String, SupertaggerError>`: build request body, invoke model, extract response text
- [ ] 7.3 Implement timeout wrapping: `tokio::time::timeout` around the invoke call
- [ ] 7.4 Write unit test with mock: successful response returns body text
- [ ] 7.5 Write unit test with mock: timeout returns `SupertaggerError::Timeout`

## 8. Top-Level API

- [ ] 8.1 Define `SupertaggerOutput { assignments: Vec<TypeAssignment>, prompt_version: &'static str }`
- [ ] 8.2 Implement `pub async fn supertag(text: &str, config: &SupertaggerConfig) -> Result<SupertaggerOutput, SupertaggerError>`: empty text → Ok(empty), else build prompt → invoke → parse → validate → return
- [ ] 8.3 Write integration-style test with canned JSON: full pipeline from text to `SupertaggerOutput`
- [ ] 8.4 Write test: empty text returns empty assignments without calling Bedrock

## 9. Canned Response Tests

- [ ] 9.1 Create `tests/canned_responses/` directory with JSON files for inj_001, ben_009, ben_018
- [ ] 9.2 Write tests that load canned JSON, parse, validate, and assert correct `TypeAssignment` output
- [ ] 9.3 Write test that loads a canned malformed JSON and asserts `JsonParseError`
- [ ] 9.4 Write test that loads a canned JSON with invalid types and asserts `InvalidOutput`

## 10. Documentation

- [ ] 10.1 Add rustdoc to all public types and functions
- [ ] 10.2 Add module-level docs explaining the supertagger's role in the pipeline
- [ ] 10.3 Document the prompt versioning scheme
- [ ] 10.4 Add a crate-level README with usage example (mock Bedrock client for local testing)
