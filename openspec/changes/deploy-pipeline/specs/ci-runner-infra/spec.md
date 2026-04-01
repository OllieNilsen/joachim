## ADDED Requirements

### Requirement: VPC isolation
CI runners SHALL run in a dedicated VPC with public subnets (no NAT gateway for cost saving). Security group SHALL allow egress only — no ingress.

### Requirement: Hardened launch template
The launch template SHALL enforce IMDSv2-only, no SSH key, and use the runner security group.

### Requirement: sccache S3 bucket
A dedicated S3 bucket SHALL store sccache artifacts with lifecycle policies: 14 days for PR caches, 90 days for main caches.

### Requirement: OIDC controller role
An IAM role assumable via GitHub OIDC SHALL allow the CI workflow to spawn and terminate EC2 runner instances. The OIDC sub claim SHALL be restricted to `repo:OllieNilsen/joachim:ref:refs/heads/*`.

### Requirement: Runner instance role
EC2 runner instances SHALL assume a role with scoped S3 access: read/write to `pr-*` prefixes, read-only to `main/` prefix.

### Requirement: GC Lambda
A Lambda function SHALL terminate orphaned runner instances on `workflow_job.completed` events via EventBridge. Instances must be tagged `ManagedBy: github-actions-ci` and older than 5 minutes.

### Requirement: Spot interceptor
A Lambda function SHALL terminate spot-interrupted instances and optionally rerun the GitHub workflow.

### Requirement: Webhook ingest
A Lambda behind API Gateway SHALL validate GitHub webhook signatures, enforce replay window, deduplicate via DynamoDB, and forward to EventBridge.

### Requirement: CloudWatch alarms
Alarms SHALL fire on GC Lambda errors, spot interceptor errors, and DLQ backlog.

### Requirement: Resource tagging
ALL resources in the stack SHALL be tagged `Project: joachim`. This SHALL be enforced via Pulumi stack transformation.

### Requirement: Pulumi outputs as GitHub variables
Stack outputs SHALL be synced to GitHub repository variables via `sync-ci-runner-vars.sh`.
