# JOACHIM CI Runner Infrastructure

Pulumi stack for ephemeral EC2 CI runners, adapted from druum's battle-tested pattern.

## What it provisions

- **VPC**: Public subnets, no NAT gateway (cost saving), egress-only security group
- **Launch template**: IMDSv2 enforced, no SSH key
- **sccache S3 bucket**: Compilation cache with lifecycle policies (14d PR, 90d main)
- **IAM**: Runner instance role (scoped S3 access) + OIDC controller role (GitHub Actions)
- **Webhook ingest**: Lambda + API Gateway for GitHub webhook processing (signature validation, DynamoDB dedup)
- **GC Lambda**: Terminates orphaned runner instances on `workflow_job.completed`
- **Spot interceptor**: Terminates spot-interrupted instances, requests workflow rerun
- **CloudWatch**: Alarms on Lambda errors + DLQ backlog, reliability dashboard

All resources tagged `Project: joachim`.

## Bootstrap

Before first `pulumi up`, create the state bucket:

```bash
aws s3 mb s3://joachim-pulumi-state --region us-east-1
aws s3api put-bucket-versioning --bucket joachim-pulumi-state --versioning-configuration Status=Enabled
```

## Deploy

```bash
cd infra/pulumi/ci-runner
npm ci
pulumi up --stack prod
```

Or via the `deploy-ci-runner.yml` GitHub Actions workflow on push to main.
