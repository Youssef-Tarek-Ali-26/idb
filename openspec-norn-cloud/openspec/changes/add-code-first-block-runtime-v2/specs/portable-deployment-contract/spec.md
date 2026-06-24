## ADDED Requirements

### Requirement: Deployment Intent Is Portable Across Substrates
The platform SHALL define deployment intent independently from any single cloud or infrastructure vendor.

#### Scenario: Target multiple substrates
- **WHEN** the same application is deployed to bare metal, a VM cluster, Kubernetes, or a cloud provider
- **THEN** the deployment contract SHALL preserve the same application-level model for blocks, flows, secrets, capabilities, and scaling intent

### Requirement: Single-Node, Clustered, and Serverless-Style Modes Share One Model
The platform SHALL treat single-node, clustered, and serverless-style operation as scheduling and lifecycle modes over the same block contract.

#### Scenario: Switch from single-node to clustered deployment
- **WHEN** an operator changes deployment mode from single-node to clustered
- **THEN** the platform SHALL preserve the same block and flow definitions while changing placement, replication, and lifecycle policy

#### Scenario: Run a scale-to-zero workload
- **WHEN** a deployment enables serverless-style scale-to-zero behavior
- **THEN** the platform SHALL apply that behavior as a runtime policy on the same block contract instead of requiring a separate function product model

### Requirement: Edge Integration Is Part of the Platform Contract
The platform SHALL expose ingress, routing, and load distribution as part of the deployment contract rather than leaving them entirely to external handwritten infrastructure glue.

#### Scenario: Declare an HTTP-triggered flow
- **WHEN** a developer marks a flow as ingress-triggered
- **THEN** the platform SHALL be able to derive route registration and edge execution metadata from the application model
