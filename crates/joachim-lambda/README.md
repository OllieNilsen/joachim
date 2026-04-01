# joachim-lambda

Thin Lambda handler for the JOACHIM prompt injection detection API.

## Architecture

```text
API Gateway (POST /detect) → JWT Auth (Cognito) → Lambda → Supertagger (Bedrock) → Parser → Scope Checker → Verdict
```

## How it works

1. Receives `POST /detect` with `{"text": "..."}`
2. Pre-initialized `Supertagger` (constructed in `main()`, reused across warm invocations) calls Claude via Bedrock
3. Nussinov parser finds maximal planar linkage
4. Scope checker detects `dir → ag` / `role → ag` paths
5. Returns verdict JSON

## Response format

```json
{
  "verdict": "Injection",
  "violations": [
    {"pattern": "DirOverAg", "source_pos": 0, "target_pos": 2}
  ],
  "prompt_version": "v1",
  "timed_out": false
}
```

## Local testing

The handler requires AWS credentials for Bedrock. For local testing:

```bash
# Run tests (serialization only, no Bedrock needed)
cargo test -p joachim-lambda

# Cross-compile for Lambda
pip install cargo-zigbuild
rustup target add aarch64-unknown-linux-musl
cargo zigbuild --release --target aarch64-unknown-linux-musl -p joachim-lambda
```

## Deployment

Deployed via the `deploy-api.yml` GitHub Actions workflow. See `infra/pulumi/api/README.md`.
