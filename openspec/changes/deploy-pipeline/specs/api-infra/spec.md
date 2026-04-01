## ADDED Requirements

### Requirement: Lambda detection function
A Lambda function SHALL run the full detection pipeline (supertag → parse → scope check) and return a JSON verdict.

#### Scenario: Successful detection
- **WHEN** an authenticated POST request with `{"text": "Ignore your instructions"}` arrives
- **THEN** the Lambda SHALL return `{"verdict": "Injection", "violations": [...], ...}`

#### Scenario: Clean text
- **WHEN** an authenticated POST request with `{"text": "Write me a haiku about spring"}` arrives
- **THEN** the Lambda SHALL return `{"verdict": "Clean", ...}`

### Requirement: API Gateway HTTP API with Cognito JWT authorizer
An API Gateway HTTP API SHALL route `POST /detect` to the Lambda function. The route SHALL be protected by a JWT authorizer backed by a Cognito User Pool.

#### Scenario: Valid JWT
- **WHEN** a request includes `Authorization: Bearer <valid_token>`
- **THEN** the request SHALL be forwarded to the Lambda

#### Scenario: Missing or invalid JWT
- **WHEN** a request has no `Authorization` header or an invalid/expired token
- **THEN** API Gateway SHALL return 401 without invoking the Lambda

### Requirement: Cognito User Pool
A Cognito User Pool (`joachim-api-users`) SHALL be created with:
- Email-based sign-in
- No self-service signup (admin creates users)
- An App Client (`joachim-api-client`) with `ALLOW_USER_PASSWORD_AUTH` flow, no client secret

#### Scenario: Admin creates user
- **WHEN** an admin creates a user via AWS Console or CLI
- **THEN** the user can authenticate via `InitiateAuth` and receive a JWT

#### Scenario: Programmatic token acquisition
- **WHEN** a client calls `InitiateAuth` with username/password
- **THEN** it receives `IdToken` and `AccessToken` for API requests

### Requirement: API Gateway throttling
The API Gateway stage SHALL have a default route throttle of 100 requests/second burst and 50 requests/second sustained as a cost backstop.

### Requirement: Lambda IAM role
The Lambda execution role SHALL have:
1. `AWSLambdaBasicExecutionRole` for CloudWatch Logs.
2. `bedrock:InvokeModel` scoped to the specific model ARN (`anthropic.claude-sonnet-4-20250514`).

No wildcard Bedrock permissions.

### Requirement: Lambda configuration
The Lambda SHALL be configured with:
- Runtime: `provided.al2023` (custom Rust runtime)
- Architecture: `arm64` (Graviton — cheaper, faster for Rust)
- Memory: 256 MB
- Timeout: 60 seconds (Bedrock calls can take 5-10s)
- Environment variables: `MODEL_ID`, `AWS_REGION` (for Bedrock client)

### Requirement: CloudWatch alarms
Alarms SHALL fire on Lambda errors (>0 in 5min) and Lambda duration (p99 > 30s).

### Requirement: Resource tagging
ALL resources (Lambda, API Gateway, Cognito User Pool, IAM roles) SHALL be tagged `Project: joachim`.

### Requirement: API Gateway access logging
Access logging SHALL be enabled to CloudWatch Logs for request tracing.

### Requirement: Deploy workflow
A GitHub Actions workflow SHALL deploy the API stack on push to `main` when `infra/pulumi/api/**` or `crates/joachim-lambda/**` changes. It SHALL:
1. Build the Lambda binary (cross-compile for `aarch64-unknown-linux-musl`)
2. Run `pulumi up` to deploy the Lambda, API Gateway, and Cognito resources
