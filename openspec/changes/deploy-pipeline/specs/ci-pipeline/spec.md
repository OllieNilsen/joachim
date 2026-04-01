## ADDED Requirements

### Requirement: PR CI triggers
On pull request to `main`, the CI pipeline SHALL run fmt, clippy, audit, and all workspace tests.

#### Scenario: PR opened
- **WHEN** a PR is opened or updated
- **THEN** static checks (fmt, audit) run on GitHub-hosted runner AND compile+test runs on ephemeral EC2 runner

### Requirement: Main branch CI triggers
On push to `main`, the CI pipeline SHALL run the same checks.

#### Scenario: Merge to main
- **WHEN** a PR is merged to main
- **THEN** the full CI suite runs before any deploy step

### Requirement: Static checks on free runner
Formatting and security audit SHALL run on `ubuntu-latest` (free GitHub runner) since they are lightweight.

### Requirement: Compile and test on EC2
Clippy and tests SHALL run on an ephemeral self-hosted EC2 runner with sccache for compilation caching.

#### Scenario: EC2 runner with sccache
- **WHEN** the compile-and-test job runs
- **THEN** it SHALL use sccache backed by the S3 cache bucket

### Requirement: Concurrency control
Only one CI run per branch SHALL execute at a time. In-progress runs SHALL be cancelled when a new push arrives.

### Requirement: Reusable workflow
The Rust CI logic SHALL be a reusable workflow (`.github/workflows/reusable-rust-ci.yml`) callable from the main pipeline and other workflows.
