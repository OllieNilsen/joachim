## Why

The pregroup core engine (`joachim-core`) is complete but requires manually constructed `Vec<TypeAssignment>` as input. To close the loop from raw text to verdict, we need an LLM-based supertagger that chunks input text and assigns pregroup types (including `VoidingKind` annotations). This is the bridge between natural language and the algebraic detection engine — without it, JOACHIM cannot process real-world input.

## What Changes

- Add `joachim-supertag` crate implementing the LLM supertagger client
- Design a structured prompt with defensive delimiters to resist meta-prompt-injection
- Parse Claude's structured JSON output into `Vec<TypeAssignment>` (from `joachim-core`) via intermediate types
- Integrate with AWS Bedrock (Claude) via a reusable `Supertagger` struct
- Handle supertagger failures gracefully (timeout, malformed output, rate limiting)
- Include the prompt template as a versioned asset for iteration

## Capabilities

### New Capabilities

- `supertagger-prompt`: The prompt template and output schema that instructs the LLM to chunk text and assign pregroup types. Covers the type inventory, functional modifier patterns, voiding annotations, few-shot examples from the ground truth corpus, and defensive delimiter tags (`<input>...</input>`) to resist meta-prompt-injection.
- `supertagger-client`: The Rust client struct (`Supertagger`) that holds a reusable Bedrock connection, sends text, extracts JSON from LLM responses (handling markdown fences and preamble), parses via intermediate types, validates output, and handles errors. Includes timeout handling.

### Modified Capabilities

(None — `joachim-core` types are consumed as-is via the `serde` feature flag.)

## Impact

- **New crate**: `crates/joachim-supertag/` added to workspace
- **Cargo.toml**: Updated workspace members
- **Dependencies**: `aws-sdk-bedrockruntime`, `aws-config`, `serde_json`, `tokio` (async runtime), `thiserror` (error types)
- **Feature flags**: `joachim-core/serde` must be enabled for JSON deserialization of type assignments
- **Secrets**: Requires AWS credentials (IAM role or env vars) — no API keys committed
- **Test strategy**: Unit tests with canned JSON responses (no live LLM calls in CI); live integration test suite against Bedrock behind a `live-test` feature flag; adversarial meta-prompt-injection test cases
