## 1. CI Runner Infra (Pulumi)

- [x] 1.0 Bootstrap: create S3 bucket `joachim-pulumi-state` with versioning enabled (one-time manual step, documented in README)
- [x] 1.1 Create `infra/pulumi/ci-runner/` with `Pulumi.yaml` (name: `joachim-ci-runner`, runtime: nodejs, backend: `s3://joachim-pulumi-state`)
- [x] 1.2 Create `package.json` with `@pulumi/pulumi`, `@pulumi/aws`, `@pulumi/awsx` dependencies
- [x] 1.3 Add Pulumi stack transformation for mandatory `Project: joachim` tag on all AWS resources
- [x] 1.4 Implement VPC: public subnets, no NAT, 2 AZs
- [x] 1.5 Implement security group: egress-only, no ingress
- [x] 1.6 Implement launch template: IMDSv2 enforced, no SSH key, runner SG
- [x] 1.7 Implement sccache S3 bucket (`joachim-ci-sccache-us-east-1`) with lifecycle policies (14d PR, 90d main)
- [x] 1.8 Implement runner instance IAM role + instance profile with scoped S3 access
- [x] 1.9 Implement OIDC controller role restricted to `repo:OllieNilsen/joachim:*` (matches branch pushes and PR events)
- [x] 1.10 Implement controller IAM policy: `ec2:RunInstances`, `ec2:TerminateInstances`, `ec2:DescribeInstances`, `iam:PassRole`, `ec2:CreateTags`
- [x] 1.11 Implement webhook ingest Lambda: signature validation, DynamoDB dedup, EventBridge forwarding
- [x] 1.12 Implement webhook API Gateway HTTP API + route + stage
- [x] 1.13 Implement DynamoDB deliveries table with TTL
- [x] 1.14 Implement GC Lambda: terminate tagged orphan runners on `workflow_job.completed`
- [x] 1.15 Implement GC EventBridge rule targeting the GC Lambda
- [x] 1.16 Implement spot interruption interceptor Lambda
- [x] 1.17 Implement spot interruption EventBridge rule
- [x] 1.18 Implement CloudWatch alarms: GC DLQ, GC errors, spot interceptor errors
- [x] 1.19 Implement CloudWatch reliability dashboard
- [x] 1.20 Export stack outputs: `controllerRoleArn`, `launchTemplateId`, `primarySubnetId`, `securityGroupId`, `sccacheBucketName`, `runnerInstanceProfileArn`, `CI_RUNNER_*` variables
- [x] 1.21 Create `scripts/sync-ci-runner-vars.sh` (adapted from druum)

## 2. CI Workflows (GitHub Actions)

- [ ] 2.1 Create `.github/workflows/reusable-rust-ci.yml`: static-checks (fmt, audit on ubuntu-latest) + compile-and-test-ec2 (clippy, test on self-hosted EC2 with sccache)
- [ ] 2.2 Create `.github/workflows/joachim-ci.yml`: main orchestrator — on PR and push to main, call reusable-rust-ci with `manifest-path: Cargo.toml`, `use-ec2-runner: true`
- [ ] 2.3 Create `.github/workflows/deploy-ci-runner.yml`: on push to main when `infra/pulumi/ci-runner/**` changes, run `pulumi up` + sync vars
- [ ] 2.4 Add concurrency groups to all workflows (cancel in-progress on new push)

## 3. API Infra (Pulumi)

- [ ] 3.1 Create `infra/pulumi/api/` with `Pulumi.yaml` (name: `joachim-api`, runtime: nodejs, backend: `s3://joachim-pulumi-state`)
- [ ] 3.2 Create `package.json` with Pulumi dependencies
- [ ] 3.3 Add Pulumi stack transformation for `Project: joachim` tagging
- [ ] 3.4 Implement Cognito User Pool (`joachim-api-users`): email sign-in, no self-service signup, tagged `Project: joachim`
- [ ] 3.5 Implement Cognito App Client (`joachim-api-client`): `ALLOW_USER_PASSWORD_AUTH` flow, no client secret, no OAuth flows
- [ ] 3.6 Implement Lambda execution IAM role: `AWSLambdaBasicExecutionRole` + `bedrock:InvokeModel` scoped to `anthropic.claude-sonnet-4-20250514`
- [ ] 3.7 Implement Lambda function: `provided.al2023` runtime, `arm64` architecture, 256MB memory, 60s timeout, env vars (`MODEL_ID`, `AWS_REGION`)
- [ ] 3.8 Implement API Gateway HTTP API with `POST /detect` route → Lambda integration
- [ ] 3.9 Implement API Gateway JWT authorizer: issuer = Cognito User Pool URL, audience = App Client ID
- [ ] 3.10 Attach JWT authorizer to `POST /detect` route (all requests must present valid Bearer token)
- [ ] 3.11 Set API Gateway stage default route throttle: 100 burst, 50 sustained
- [ ] 3.12 Enable API Gateway access logging to CloudWatch Logs
- [ ] 3.13 Implement CloudWatch alarms: Lambda errors (>0 in 5min), p99 duration (>30s)
- [ ] 3.14 Implement smoke test user: Pulumi dynamic provider or `local.Command` to `admin-create-user` + `admin-set-user-password` in Cognito
- [ ] 3.15 Store smoke test credentials in Secrets Manager (`joachim/smoke-test-user`)
- [ ] 3.16 Grant deploy workflow OIDC role `secretsmanager:GetSecretValue` for `joachim/smoke-test-user`
- [ ] 3.17 Export stack outputs: `apiUrl`, `lambdaFunctionName`, `lambdaRoleArn`, `userPoolId`, `userPoolClientId`, `smokeTestSecretArn`

## 4. Lambda Handler Crate

- [ ] 4.1 Create `crates/joachim-lambda/Cargo.toml`: depend on `joachim-core`, `joachim-supertag`, `lambda_http`, `lambda_runtime`, `tokio`, `serde_json`
- [ ] 4.2 Add `joachim-lambda` to workspace members
- [ ] 4.3 Implement request types: `DetectRequest { text: String }`, `DetectResponse { verdict, violations, prompt_version, timed_out }`
- [ ] 4.4 Implement `main()`: pre-initialize `Supertagger::new().await` then `run(service_fn(|event| handler(event, &tagger)))`
- [ ] 4.5 Implement handler function: parse request → supertag → parse → scope check → return response
- [ ] 4.6 Implement error mapping: `SupertaggerError::InputTooLong` → 400, other `SupertaggerError` → 502, bad request → 400
- [ ] 4.7 Write unit test: `DetectRequest` deserializes from valid JSON
- [ ] 4.8 Write unit test: `DetectResponse` serializes to expected JSON structure
- [ ] 4.9 Write unit test: missing `text` field in request body produces 400 error mapping
- [ ] 4.10 Write unit test: malformed JSON request body produces 400 error mapping
- [ ] 4.11 Note: full pipeline test (supertag → parse → scope check) is covered by deploy smoke test, not unit tests (requires Bedrock)

## 5. Deploy API Workflow

- [ ] 5.1 Create `.github/workflows/deploy-api.yml`: on push to main when `infra/pulumi/api/**` or `crates/joachim-lambda/**` changes
- [ ] 5.2 Add step: install Rust toolchain + `aarch64-unknown-linux-musl` target + `pip install cargo-zigbuild`
- [ ] 5.3 Add step: cross-compile Lambda binary (`cargo zigbuild --release --target aarch64-unknown-linux-musl -p joachim-lambda`)
- [ ] 5.4 Add step: package binary as `bootstrap` in zip for Lambda `provided.al2023`
- [ ] 5.5 Add step: OIDC auth + `pulumi up` for the api stack
- [ ] 5.6 Add step: read smoke test credentials from Secrets Manager (`joachim/smoke-test-user`)
- [ ] 5.7 Add step: acquire Cognito token via `aws cognito-idp initiate-auth` using smoke test credentials
- [ ] 5.8 Add step: authenticated smoke test — `curl -H "Authorization: Bearer $TOKEN" -X POST $API_URL/detect -d '{"text":"test"}'` and assert 200
- [ ] 5.9 Add step: unauthenticated smoke test — `curl -X POST $API_URL/detect -d '{"text":"test"}'` and assert 401

## 6. Documentation

- [ ] 6.1 Add README to `infra/pulumi/ci-runner/` explaining the stack
- [ ] 6.2 Add README to `infra/pulumi/api/` explaining the stack
- [ ] 6.3 Add README to `crates/joachim-lambda/` with usage and local testing instructions
- [ ] 6.4 Update ROADMAP.md to reflect deploy-pipeline completion
- [ ] 6.5 Document cost tracking: how to use AWS Cost Explorer with `Project: joachim` tag
- [ ] 6.6 Document authentication: how to create a Cognito user, acquire a token via `InitiateAuth`, and call the API
