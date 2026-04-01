## Why

JOACHIM has two working crates (`joachim-core`, `joachim-supertag`) but no way to build, test, or deploy them in CI or serve them as an API. We need:

1. A CI pipeline that compiles and tests Rust on ephemeral EC2 runners (free GitHub runners are too small/slow for the AWS SDK dependency tree).
2. A Lambda + API Gateway endpoint that runs the full detection pipeline (supertag → parse → scope check → verdict) as an HTTP API.
3. IaC (Pulumi) for both the CI runner infrastructure and the API deployment.

We reuse the proven CI runner pattern from `../druum` (VPC, ephemeral EC2, sccache, OIDC, GC Lambda, spot interceptor) adapted for the joachim repo, and add a new API stack for the Lambda detection endpoint.

All JOACHIM-specific AWS resources are tagged `Project: joachim` for cost tracking and governance.

## What Changes

- Add `infra/pulumi/ci-runner/` — Pulumi stack for ephemeral EC2 CI runners (adapted from druum)
- Add `infra/pulumi/api/` — Pulumi stack for Lambda detection endpoint + API Gateway + Cognito JWT auth + Bedrock IAM
- Add `.github/workflows/` — CI pipeline (reusable Rust CI), CI runner deploy, API deploy
- Add `scripts/sync-ci-runner-vars.sh` — sync Pulumi outputs to GitHub repo variables
- Add `crates/joachim-lambda/` — thin Lambda handler crate wiring the detection pipeline
- All AWS resources tagged with `Project: joachim` for cost attribution

## Capabilities

### New Capabilities

- `ci-pipeline`: GitHub Actions CI pipeline with ephemeral EC2 runners, sccache, fmt/clippy/test. Triggers on PR and push to main.
- `ci-runner-infra`: Pulumi IaC for VPC, launch template, sccache S3 bucket, OIDC controller role, GC Lambda, spot interceptor. Adapted from druum with joachim-specific naming and tagging.
- `api-infra`: Pulumi IaC for the detection API — Lambda function, API Gateway HTTP API with Cognito JWT authorizer, Cognito User Pool (admin-managed), IAM role with scoped `bedrock:InvokeModel` permission, CloudWatch alarms, throttling.
- `lambda-handler`: Rust Lambda crate that deserializes HTTP requests, runs supertag → parse → scope check, returns verdict JSON.

### Modified Capabilities

(None — all new infrastructure.)

## Impact

- **New crate**: `crates/joachim-lambda/` added to workspace
- **New IaC**: `infra/pulumi/ci-runner/` and `infra/pulumi/api/`
- **New workflows**: `.github/workflows/` (CI, deploy-ci-runner, deploy-api)
- **Dependencies**: `lambda_http`, `lambda_runtime`, `tokio` (Lambda crate); `@pulumi/pulumi`, `@pulumi/aws`, `@pulumi/awsx` (IaC)
- **AWS resources**: VPC, EC2 launch template, S3 bucket, IAM roles (OIDC + Lambda), API Gateway, Cognito User Pool, Lambda function, CloudWatch alarms — all tagged `Project: joachim`
- **Secrets required**: `AWS_ROLE_ARN_PULUMI_DEPLOY` (OIDC role for Pulumi), `githubWebhookSecret` (Pulumi config)
- **Shared account**: Same AWS account as druum, separate Pulumi stacks. Shares the existing OIDC provider.
