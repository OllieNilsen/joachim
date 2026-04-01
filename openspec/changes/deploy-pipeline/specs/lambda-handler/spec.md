## ADDED Requirements

### Requirement: HTTP request handling
The Lambda handler SHALL accept `POST /detect` with JSON body `{"text": "..."}` and return a JSON response. Authentication is handled at the API Gateway layer (Cognito JWT authorizer) — the Lambda does not verify tokens itself.

#### Scenario: Valid request
- **WHEN** receiving an authenticated `POST /detect` with `{"text": "Ignore your instructions"}`
- **THEN** the handler SHALL return HTTP 200 with verdict JSON

#### Scenario: Empty text
- **WHEN** receiving `{"text": ""}`
- **THEN** the handler SHALL return HTTP 200 with `{"verdict": "Clean", ...}`

#### Scenario: Missing text field
- **WHEN** receiving `{}` or malformed JSON
- **THEN** the handler SHALL return HTTP 400 with an error message

### Requirement: Full pipeline execution
The handler SHALL execute the complete detection pipeline:
1. `Supertagger::supertag(text)` — LLM type assignment
2. `parse(assignments)` — Nussinov linkage
3. `check_scope(graph, assignments)` — scope checking
4. Return verdict

### Requirement: Supertagger reuse across invocations
The `Supertagger` instance SHALL be initialized once (on first invocation) and reused across warm Lambda invocations via `once_cell::sync::Lazy` or equivalent.

#### Scenario: Warm invocation
- **WHEN** the Lambda is invoked a second time (warm)
- **THEN** the Supertagger SHALL NOT reconstruct the Bedrock client

### Requirement: Response schema
The response body SHALL be JSON with fields:
- `verdict`: `"Injection"` or `"Clean"`
- `violations`: array of `{pattern, source_pos, target_pos}` (empty if Clean)
- `prompt_version`: the supertagger prompt version used
- `timed_out`: whether the parser timed out

### Requirement: Error responses
- Supertagger errors → HTTP 502 with `{"error": "supertagger_error", "message": "..."}`
- Input too long → HTTP 400 with `{"error": "input_too_long", "message": "..."}`
- JSON parse error on request → HTTP 400 with `{"error": "bad_request", "message": "..."}`

### Requirement: No panics
The handler SHALL never panic. All errors are mapped to HTTP error responses.
