#!/usr/bin/env bash
set -euo pipefail

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI is required" >&2
  exit 1
fi

if [ -z "${GH_TOKEN:-}" ]; then
  echo "GH_TOKEN is required" >&2
  exit 1
fi

STACK_DIR="${1:-infra/pulumi/ci-runner}"
STACK_NAME="${2:-prod}"

OUTPUT_JSON=$(pulumi stack output --json --stack "$STACK_NAME" --cwd "$STACK_DIR")

set_var() {
  local name="$1"
  local value="$2"
  gh variable set "$name" --body "$value"
  echo "Set $name"
}

set_var "AWS_ROLE_ARN_CI_RUNNER" "$(echo "$OUTPUT_JSON" | node -e 'const o=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write(o.AWS_ROLE_ARN_CI_RUNNER);')"
set_var "CI_RUNNER_AMI_ID" "$(echo "$OUTPUT_JSON" | node -e 'const o=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write(o.CI_RUNNER_AMI_ID);')"
set_var "CI_RUNNER_INSTANCE_TYPE" "$(echo "$OUTPUT_JSON" | node -e 'const o=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write(o.CI_RUNNER_INSTANCE_TYPE);')"
set_var "CI_RUNNER_SUBNET_ID" "$(echo "$OUTPUT_JSON" | node -e 'const o=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write(o.CI_RUNNER_SUBNET_ID);')"
set_var "CI_RUNNER_SECURITY_GROUP_ID" "$(echo "$OUTPUT_JSON" | node -e 'const o=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write(o.CI_RUNNER_SECURITY_GROUP_ID);')"
set_var "CI_RUNNER_INSTANCE_PROFILE" "$(echo "$OUTPUT_JSON" | node -e 'const o=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write(o.CI_RUNNER_INSTANCE_PROFILE);')"
set_var "SCCACHE_S3_BUCKET" "$(echo "$OUTPUT_JSON" | node -e 'const o=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write(o.SCCACHE_S3_BUCKET);')"
