## Why

The pregroup core engine (`joachim-core`) is complete but requires manually constructed `Vec<TypeAssignment>` as input. To close the loop from raw text to verdict, we need an LLM-based supertagger that chunks input text and assigns pregroup types (including `VoidingKind` annotations). This is the bridge between natural language and the algebraic detection engine — without it, JOACHIM cannot process real-world input.

## What Changes

- Add `joachim-supertag` crate implementing the LLM supertagger client
- Design a structured prompt that instructs Claude to chunk text and assign pregroup types
- Parse Claude's structured JSON output into `Vec<TypeAssignment>` (from `joachim-core`)
- Integrate with AWS Bedrock (Claude) for the LLM backend
- Handle supertagger failures gracefully (timeout, malformed output, rate limiting)
- Include the prompt template as a versioned asset for iteration

## Capabilities

### New Capabilities

- `supertagger-prompt`: The prompt template and output schema that instructs the LLM to chunk text and assign pregroup types. Covers the type inventory, functional modifier patterns, voiding annotations, and few-shot examples from the ground truth corpus.
- `supertagger-client`: The Rust client that sends text to AWS Bedrock (Claude), parses the structured JSON response into `Vec<TypeAssignment>`, validates the output, and handles errors. Includes retry logic and timeout handling.

### Modified Capabilities

(None — `joachim-core` types are consumed as-is via the `serde` feature flag.)

## Impact

- **New crate**: `crates/joachim-supertag/` added to workspace
- **Cargo.toml**: Updated workspace members
- **Dependencies**: `aws-sdk-bedrockruntime`, `serde_json`, `tokio` (async runtime), `thiserror` (error types)
- **Feature flags**: `joachim-core/serde` must be enabled for JSON deserialization of type assignments
- **Secrets**: Requires AWS credentials (IAM role or env vars) — no API keys committed
- **Test strategy**: Unit tests with canned JSON responses (no live LLM calls in CI); integration test suite against live Bedrock behind a feature flag
