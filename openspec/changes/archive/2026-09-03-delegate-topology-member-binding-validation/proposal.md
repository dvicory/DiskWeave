## Why

The current topology and service-assembly rejection rules are expressed in prose and repeated in Rust validation paths, which leaves the exact bounded relation difficult to review and easy to drift. Delegate only that pure relation to Quint, without changing any accepted or rejected production input or transferring discovery, admission, transition, recovery, or I/O policy.

## What Changes

- Delegate the exact finite validity relation over supplied, owner-accepted topology snapshots and member bindings to a new canonical `models/quint/TopologyMemberBindingValidation.qnt` model.
- Limit the delegated relation to current observable structural and correspondence decisions: complete assignment coverage relative to a supplied accepted profile; unique logical slots, active coding positions, and assignment-instance identities; exact assignment-to-binding coverage and field equality; opened-store identity/epoch and supplied-geometry correspondence; rejection of owner-classified aliases or ambiguity; and semantic lookup independent of assignment, binding, discovery, or collection order.
- Treat accepted profiles and geometry, candidate observations, owner-qualified identity comparisons, store facts, and bindings as supplied inputs. Their construction or deserialization validation, discovery, trust, assessment precedence, selection, authorization, and physical truth remain with their current owners.
- Keep unresolved array-identity comparison and assignment-to-store authorization, invalid profile/geometry ingress, the implementation's assignment-count resource ceiling, the healthy service's supported-profile and read/write/flush capability policy, request admission, writable assembly authorization, topology transitions and publication, payload correctness, recovery, persistence, and physical I/O outside the delegated relation.
- Add bounded exhaustive analysis, positive deterministic scenarios, mutation canaries that each start from a valid state, and production correspondence through direct deterministic comparisons and `quint-connect` where it adds observable coverage. Finite configurations and evidence modules do not become semantic authority.
- Require an adversarial model review to return `GO` before any prose cutover. Then remove only prose made redundant by the accepted exact relation while retaining requirement identity, product context, refusal timing, scenarios, and every non-delegated decision.
- Require a fresh adversarial review of the exact post-cutover repository snapshot to return `GO` before evidence currentization, canonical synchronization, archive, or later workspace adoption.
- Preserve current admission outcomes. This is a semantic-ownership cutover, not a product behavior change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `anchorless-topology-identity`: Make `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments` retain product ownership while delegating only its exact bounded snapshot/member-binding validity relation to Quint.
- `healthy-portable-io`: Replace repeated validation-predicate narration with consumption of the delegated result while retaining service orchestration, capability policy, refusal timing, request admission, mutation barriers, and stable semantic lookup behavior.

## Impact

- Planned canonical model: `models/quint/TopologyMemberBindingValidation.qnt`.
- Planned evidence-only assets: bounded analysis, deterministic scenario, mutant, and optional Connect modules under `verification/quint/`; focused Rust correspondence beside `TopologySnapshot::validate` and service assembly validation; entries in `verification/manifest.toml` and maintained verification evidence.
- Planned production correspondence: owner-accepted `CodingProfile`/`ProtectedGeometry` projection, `crates/dwv-core/src/topology.rs::TopologySnapshot::validate`, semantic slot/position lookup, and `crates/dwv-service/src/service.rs::validate_assembly`; unchecked profile/geometry deserialization remains an explicit non-delegated conformance gap.
- Proposed canonical prose changes are confined to the two modified capabilities. No `.qnt`, current canonical spec, production, test, verification, maintained-documentation, Bead, or workspace-adoption file changes during this proposal phase.
