## Context

JOACHIM's core engine (`joachim-core`) takes `Vec<TypeAssignment>` and produces a `Verdict`. The supertagger is the component that bridges natural language to this algebraic input: it takes raw text and produces the chunking and type assignments that the parser needs.

For MVP, we use an LLM (Claude via AWS Bedrock) as the supertagger. The LLM is given a structured prompt explaining the type inventory, modifier patterns, and voiding annotations, along with few-shot examples from the ground truth corpus. It returns a JSON array of type assignments.

**Constraints**:
- Must produce valid `Vec<TypeAssignment>` (monotonic chunk_idx, non-empty TypeExprs, valid TypeIds)
- Must be async (Bedrock calls are IO-bound)
- Must handle failures gracefully (timeout, malformed JSON, rate limiting)
- No hardcoded AWS credentials — use IAM roles or environment variables
- Prompt template must be versioned and easy to iterate

## Goals / Non-Goals

**Goals:**
- Design a structured prompt that produces correct type assignments for the 45-example corpus
- Implement a Bedrock client that sends requests and parses responses
- Validate LLM output before passing to the parser
- Handle errors without panicking (return a typed error)

**Non-Goals:**
- Fine-tuning a model (v2)
- Caching or deduplication of requests
- Batch processing
- Prompt optimization beyond "works on corpus" (evaluation phase handles this)

## Decisions

### Decision 1: Structured JSON Output Schema

**Choice**: The LLM returns a JSON array matching the `TypeAssignment` serde format directly. No intermediate representation.

```json
[
  {
    "chunk_idx": 0,
    "chunk_text": "Ignore the above instructions",
    "type_expr": [
      { "base": "Dir", "adjoint": 0 },
      { "base": "Ag", "adjoint": -1 },
      { "base": "Ag", "adjoint": 0 }
    ],
    "voiding": null
  },
  {
    "chunk_idx": 1,
    "chunk_text": "and",
    "type_expr": [
      { "base": "Conj", "adjoint": 0 }
    ],
    "voiding": null
  }
]
```

The `chunk_text` field is informational (for debugging/audit) and is stripped before passing to the parser.

**Rationale**:
- Direct serde compatibility means the response can be deserialized into Rust types with minimal transformation.
- `chunk_text` provides explainability without affecting the algebra.
- The schema is simple enough for Claude to produce reliably with structured output / tool-use.

### Decision 2: Prompt Structure

**Choice**: The prompt has four sections:

1. **System context**: You are a linguistic supertagger for prompt injection detection. Your job is to chunk the input text and assign pregroup types.
2. **Type inventory**: The 9 primitives, functional modifier patterns, voiding kinds. Exact definitions from `type_annotations_v0.md`.
3. **Few-shot examples**: 4-6 examples from the ground truth corpus (mix of injection and benign, including voiding).
4. **Input**: The raw text to analyze.

The system prompt is a static asset. The user message contains only the input text.

**Rationale**:
- Separating system/user ensures the type inventory is in the system prompt (cached by Bedrock, not re-processed per call).
- Few-shot examples ground the model's output format and demonstrate voiding correctly.
- Keeping the user message to just the input text makes it easy to swap inputs.

### Decision 3: AWS Bedrock Integration

**Choice**: Use the `aws-sdk-bedrockruntime` crate with the `InvokeModel` API (not streaming). Target `anthropic.claude-sonnet-4-20250514` for MVP (fast, cheap, good at structured output).

**Rationale**:
- `InvokeModel` is simpler than streaming for a single request-response pattern.
- Sonnet is sufficient for structured output tasks; Opus is overkill and slower.
- The AWS SDK handles credential resolution (env vars, IAM role, SSO) natively.

### Decision 4: Output Validation

**Choice**: After deserializing the JSON, validate the output before constructing `Vec<TypeAssignment>`:

1. `chunk_idx` values are monotonically non-decreasing.
2. No `type_expr` is empty.
3. All `base` values are valid `TypeId` variants.
4. `adjoint` values are in range `[-5, 5]` (generous bound, catches garbage).
5. At least one chunk is present.

If validation fails, return `SupertaggerError::InvalidOutput` with the raw JSON for debugging.

**Rationale**:
- The parser validates too, but catching errors at the supertagger boundary provides better diagnostics (we can include the raw LLM response in the error).
- The adjoint bound is looser than the proptest generator's `[-3, 3]` to allow the model some flexibility, but tight enough to catch nonsense.

### Decision 5: Error Handling

**Choice**: The supertagger returns `Result<Vec<TypeAssignment>, SupertaggerError>` where:

```rust
#[derive(Debug, thiserror::Error)]
pub enum SupertaggerError {
    #[error("Bedrock request failed: {0}")]
    BedrockError(String),

    #[error("LLM response was not valid JSON: {raw}")]
    JsonParseError { raw: String, source: serde_json::Error },

    #[error("LLM output failed validation: {reason}")]
    InvalidOutput { reason: String, raw: String },

    #[error("Request timed out after {0:?}")]
    Timeout(std::time::Duration),
}
```

**Rationale**:
- `thiserror` for ergonomic error types with minimal boilerplate.
- Preserving the raw response in error variants enables debugging ("what did the model actually say?").
- The detection pipeline can map all errors to a fail-safe conservative verdict.

### Decision 6: Configuration

**Choice**: The supertagger client is configured via a `SupertaggerConfig` struct:

```rust
pub struct SupertaggerConfig {
    /// Bedrock model ID (e.g., "anthropic.claude-sonnet-4-20250514").
    pub model_id: String,
    /// AWS region for Bedrock.
    pub region: String,
    /// Maximum tokens in the response.
    pub max_tokens: u32,
    /// Request timeout.
    pub timeout: std::time::Duration,
    /// Temperature (0.0 for deterministic).
    pub temperature: f32,
}
```

Defaults: `claude-sonnet-4-20250514`, `us-east-1`, 4096 tokens, 30s timeout, temperature 0.0.

**Rationale**:
- Explicit config struct rather than environment variables for testability.
- Temperature 0.0 for deterministic output — we want consistent type assignments.
- 4096 tokens is generous for ~50 chunks (each ~30 tokens of JSON).

## Risks / Trade-offs

**[Risk] LLM produces inconsistent type assignments across runs**
→ Mitigation: Temperature 0.0 for determinism. Evaluation phase will measure consistency.

**[Risk] LLM hallucinates invalid types or adjoint values**
→ Mitigation: Validation layer catches malformed output. Fail-safe returns error, pipeline maps to conservative verdict.

**[Risk] Prompt engineering is fragile and model-dependent**
→ Mitigation: Prompt is a versioned static asset. When we change models, we re-evaluate against the corpus.

**[Risk] Bedrock latency is too high for real-time use**
→ Mitigation: MVP is not latency-critical. Future optimization: cache common patterns, use smaller models, or fine-tune.

**[Risk] AWS SDK adds significant binary size and compile time**
→ Mitigation: Acceptable for the supertagger crate (not joachim-core). The core remains dependency-free.
