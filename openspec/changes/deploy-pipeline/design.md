## Context

JOACHIM needs CI and deployment infrastructure. The druum repo has a battle-tested CI runner setup (ephemeral EC2 with sccache, OIDC, GC, spot interceptor) that we adapt. The detection API is new — a Lambda behind API Gateway that runs the full pipeline.

All resources share the same AWS account as druum but use separate Pulumi stacks and are tagged `Project: joachim` for cost attribution.

**Constraints**:
- Same AWS account as druum, separate Pulumi stacks
- OIDC authentication only — no long-lived credentials
- All resources tagged `Project: joachim`
- Lambda must have `bedrock:InvokeModel` permission for the supertagger
- EC2 runners need sccache S3 access for compilation caching
- Free GitHub runners are too resource-constrained for the AWS SDK compilation

## Goals / Non-Goals

**Goals:**
- CI pipeline: fmt, clippy, audit, test on ephemeral EC2 runners with sccache
- Detection API: Lambda + API Gateway HTTP endpoint
- IaC for both stacks via Pulumi (TypeScript)
- Cost tracking via `Project: joachim` tag on all resources
- Operational alarms for Lambda errors and DLQ backlog

**Non-Goals:**
- Multi-environment (staging/prod) — MVP is single environment
- Custom domain / TLS cert — use API Gateway default URL
- User self-service signup — admin creates users manually in Cognito
- CDK (we use Pulumi)
- Selective CI routing (joachim is a small repo, not a monorepo)

## Decisions

### Decision 1: Reuse druum CI Runner Pattern

**Choice**: Adapt the druum `ci-runner` Pulumi stack with joachim-specific naming.

**What we reuse** (proven, no changes needed to the pattern):
- VPC with public subnets, no NAT gateway (cost saving)
- Security group: egress-only, no SSH
- Launch template: IMDSv2 enforced, no SSH key
- sccache S3 bucket with lifecycle policies (14d PR, 90d main)
- IAM runner instance role (scoped S3 access)
- OIDC controller role (restricted to `repo:OllieNilsen/joachim`)
- Webhook ingest Lambda (signature validation, DynamoDB dedup, EventBridge)
- GC Lambda (terminates orphaned runners)
- Spot interceptor Lambda (terminates + reruns on spot interruption)
- CloudWatch alarms and dashboard

**What changes**:
- All resource names prefixed `joachim-` instead of `ci-runner-`
- OIDC sub claim: `repo:OllieNilsen/joachim:ref:refs/heads/*`
- sccache bucket: `joachim-ci-sccache-us-east-1`
- Region: `us-east-1` (same as Bedrock)
- All resources tagged `Project: joachim`

### Decision 2: Mandatory Resource Tagging

**Choice**: Every AWS resource created by Pulumi SHALL have the tag `Project: joachim`. This is enforced via Pulumi's `transformations` API which applies default tags to all taggable resources.

```typescript
const projectTags = { Project: "joachim" };

// Applied to all resources in the stack:
pulumi.runtime.registerStackTransformation((args) => {
    if (args.props.tags !== undefined) {
        args.props.tags = { ...args.props.tags, ...projectTags };
    } else if (args.type.startsWith("aws:")) {
        args.props.tags = projectTags;
    }
    return { props: args.props, opts: args.opts };
});
```

**Rationale**:
- Cost Explorer can filter by `Project: joachim` to track all JOACHIM spend.
- Prevents cost bleed from untagged resources.
- Stack-level transformation ensures no resource is forgotten.

### Decision 3: Lambda Detection Endpoint

**Choice**: A single Lambda function behind an API Gateway HTTP API.

```
Client → API Gateway HTTP API → Lambda → Supertagger (Bedrock) → Parser → Scope Checker → Verdict
```

The Lambda handler:
1. Receives `POST /detect` with JSON body `{"text": "..."}`
2. Constructs a `Supertagger` (reused across warm invocations via `once_cell`)
3. Calls `supertag()` → `parse()` → `check_scope()`
4. Returns JSON `{"verdict": "Injection"|"Clean", "violations": [...], "prompt_version": "v1", "timed_out": false}`

**Rationale**:
- Lambda is the cheapest option for MVP traffic (pay per request, no idle cost).
- API Gateway HTTP API is simpler and cheaper than REST API.
- `once_cell::sync::Lazy` holds the `Supertagger` across warm invocations, amortizing Bedrock client construction.
- Cold start is acceptable for MVP — the Rust binary is small (~10-20MB), cold start ~200ms.

### Decision 4: Lambda IAM with Bedrock Access

**Choice**: The Lambda execution role has:
1. `AWSLambdaBasicExecutionRole` (CloudWatch Logs)
2. `bedrock:InvokeModel` scoped to the specific model ARN

```typescript
{
    Effect: "Allow",
    Action: ["bedrock:InvokeModel"],
    Resource: [`arn:aws:bedrock:us-east-1::foundation-model/anthropic.claude-sonnet-4-20250514`]
}
```

**Rationale**:
- Least privilege — only the specific model, not all Bedrock models.
- No `bedrock:*` wildcard.

### Decision 5: Reusable Rust CI Workflow

**Choice**: Adapt druum's `reusable-rust-ci.yml` for joachim. The workflow has:
- `static-checks` job: fmt, cargo-audit (runs on free GitHub runner — lightweight)
- `compile-and-test-ec2` job: clippy, test (runs on ephemeral EC2 with sccache)

The main CI workflow calls the reusable workflow on PR and push to main.

**Rationale**:
- Static checks don't need EC2 — they're fast and lightweight.
- Compilation and tests need EC2 — the AWS SDK dependency tree is too large for free runners.
- Reusable workflow is parameterized by manifest path and runner label.

### Decision 6: API Gateway HTTP API with Cognito JWT Authorizer

**Choice**: Use API Gateway HTTP API with a Cognito User Pool JWT authorizer. All requests to `POST /detect` must include a valid JWT `Authorization: Bearer <token>` header. Unauthenticated requests receive 401.

```
Client → (JWT Bearer) → API Gateway HTTP API → JWT Authorizer (Cognito) → Lambda
```

**Cognito User Pool setup**:
- A Cognito User Pool (`joachim-api-users`) with email-based sign-in.
- An App Client (`joachim-api-client`) with `ALLOW_USER_PASSWORD_AUTH` flow (for programmatic access via `InitiateAuth`). No client secret (public client for CLI/SDK use).
- Admin creates users manually — no self-service signup (MVP).
- The API Gateway JWT authorizer validates the `id_token` or `access_token` against the User Pool's JWKS endpoint.

**Rationale**:
- An unauthenticated endpoint backed by Bedrock is a direct billing risk — anyone who discovers the URL can invoke arbitrarily expensive LLM calls.
- Cognito is AWS-native, zero-ops, and integrates natively with API Gateway HTTP API's built-in JWT authorizer (no Lambda authorizer needed).
- JWT validation happens at the API Gateway layer before the Lambda is invoked — unauthorized requests don't cost Lambda compute or Bedrock tokens.
- `ALLOW_USER_PASSWORD_AUTH` enables programmatic token acquisition for API clients without a browser (CLI, SDK, CI tests).

### Decision 7: Lambda Handler Crate

**Choice**: A thin `joachim-lambda` crate that depends on `joachim-core` and `joachim-supertag`.

```rust
use lambda_http::{run, service_fn, Request, Response, Body, Error};

async fn handler(event: Request) -> Result<Response<Body>, Error> {
    // Parse request body
    // Call supertagger → parser → scope checker
    // Return verdict JSON
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}
```

**Dependencies**: `lambda_http`, `lambda_runtime`, `tokio`, `serde_json`, `once_cell`.

**Rationale**:
- `lambda_http` provides ergonomic HTTP request/response handling for API Gateway integration.
- The handler is deliberately thin — all logic lives in `joachim-core` and `joachim-supertag`.
- `once_cell::sync::Lazy` initializes the `Supertagger` on first invocation and reuses it.

## Risks / Trade-offs

**[Risk] Cold start latency**
→ Mitigation: Rust Lambda cold starts are ~200ms. Acceptable for MVP. Provisioned concurrency can be added later if needed.

**[Risk] Bedrock latency dominates request time**
→ Mitigation: The Bedrock call (1-5s) dwarfs everything else. Lambda compute adds ~10ms. This is inherent to the LLM-based architecture; optimizing Lambda won't help.

**[Risk] EC2 runner costs**
→ Mitigation: Ephemeral runners are terminated immediately after use. GC Lambda catches orphans. Spot instances reduce cost. sccache reduces compilation time (and thus EC2 runtime).

**[Risk] Shared account resource naming collisions with druum**
→ Mitigation: All joachim resources prefixed `joachim-`. Different Pulumi stacks. Tags enable cost separation.

**[Risk] Cognito token management overhead for API clients**
→ Mitigation: Clients call `InitiateAuth` to get a JWT, then pass it in `Authorization: Bearer` header. Tokens last 1 hour by default. Simple enough for programmatic use. A helper script or SDK wrapper can be added later.

**[Risk] API Gateway throttling as cost backstop**
→ Mitigation: API Gateway default throttle (10,000 req/s burst, 5,000 sustained) is far above MVP needs. We set a lower default rate limit (100 req/s) on the stage to bound worst-case cost even with valid tokens.
