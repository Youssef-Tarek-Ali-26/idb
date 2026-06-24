## ADDED Requirements

### Requirement: Secret Usage Is Declared in Source but Resolved at Runtime
The platform SHALL let repos declare secret references while resolving secret values only at execution time from linked providers.

#### Scenario: Bind a provider-backed secret reference
- **WHEN** a block declares a required secret reference
- **THEN** the deployment model SHALL link that reference to a configured secret provider without storing the secret value in source artifacts

### Requirement: Capability Declarations Gate Secret and Resource Access
The platform SHALL require capability declarations for privileged resource access, including secret use.

#### Scenario: Block requests an outbound credential
- **WHEN** a block declares a secret reference and a privileged resource capability
- **THEN** the runtime SHALL enforce both the secret binding policy and the capability policy before execution proceeds

### Requirement: Runtime Injection Must Support Redaction and Rotation
The platform SHALL inject secrets in a way that supports value rotation and automatic redaction from logs, traces, and replay metadata.

#### Scenario: Secret rotates without source changes
- **WHEN** a secret value is rotated in the backing provider
- **THEN** subsequent block executions SHALL receive the updated value without requiring source-level contract changes

#### Scenario: Observability records a failed execution
- **WHEN** traces or logs are emitted for a block that used secrets
- **THEN** the platform SHALL redact secret material from operator-facing telemetry and replay artifacts
