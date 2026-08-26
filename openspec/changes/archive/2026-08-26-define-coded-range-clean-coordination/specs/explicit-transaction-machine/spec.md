## ADDED Requirements

### Requirement: Coded parity-range authority covers shared parity conflicts
<!-- dwv:req req.explicit-transaction-machine.coded-range-authority-covers-shared-parity-conflicts -->
<!-- dwv:requires req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics -->
<!-- dwv:requires req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->
<!-- dwv:requires req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments -->
<!-- dwv:requires req.xor-reference-model.xor-parity-uses-explicit-protected-geometry -->
<!-- dwv:requires req.explicit-transaction-machine.reference-traces-are-deterministic-and-implementation-independent -->
<!-- dwv:requires req.explicit-transaction-machine.transactions-emit-normalized-semantic-actions -->
<!-- dwv:requires req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations -->
<!-- dwv:requires req.store-operation-contracts.resource-admission-and-identity-remain-bounded-and-explicit -->
<!-- dwv:requires req.store-operation-contracts.store-failures-are-conservative-and-testable -->
<!-- dwv:requires req.healthy-portable-io.generation-qualified-release-authorization-composes-owner-approved-lifecycle-facts -->

The transaction boundary SHALL own the product conflict meaning for a bounded coded-range authority claim derived from the exact validated semantic mutation unit, captured topology epoch, assignment instances and generations, protected geometry, and selected coding profile. A semantic mutation unit MAY be a validated request-decomposition unit rather than an undecomposed frontend request, but it SHALL carry a complete mapped coded claim covering every dependent basis observation and mutation before admission. Under one captured topology and profile, operations SHALL conflict when their mapped parity/codeword claims intersect, even when they target different member identities; per-member identity, store range, dirty region, or checksum extent SHALL NOT establish disjointness. Classification is scope-local; no global lock, counter, queue, connected-component interpretation, or physical authority mechanism is required.

Partial physical acquisition is pre-admission and authorizes no dependent read or mutation. The healthy service SHALL perform no dependent basis I/O or protected mutation until complete coded authority and bounded operation/resource admission accept the semantic unit; acquisition order and mechanism remain implementation choices. An otherwise valid complete claim that overlaps held coded authority SHALL remain pending through the existing wait, queue, backpressure, or admission behavior until the contention clears. Only actual bounded authority/resource exhaustion MAY return bounded refusal. Both outcomes occur before dependent I/O; incomplete mappings or stale topology/profile/generation are refused before dependent I/O.

Mutations and relevant mutation-basis reads SHALL use the same coded-range conflict meaning. A relevant basis observation SHALL be correlated to its consuming semantic mutation unit and complete coded claim, remain coherent through a consuming mutation, or be discarded/reconciled before release. Independent overlapping observations coordinate under the same claims; no release-then-use gap or unmodeled claim upgrade is accepted. Basis-read lifecycle and coherence evidence remain supplied by existing transaction, store, recovery, and healthy-service owners.

The coded owner SHALL consume, not manufacture, captured topology/profile and mapping validity, bounded operation/resource admission, recovery and store observations, authoritative reopen/reconciliation, and exact external `ReleaseAllowed(operation-generation)` from the canonical healthy-portable-io lifecycle-release owner. The coded owner SHALL NOT inspect or reconstruct the contributing lifecycle predicates, decide terminality, duplicate release policy, redefine `RecoveryProtocol.releaseRange`, decide persistence admissibility, or create product claims. CLEAN-capture uncertainty neither supplies nor negates `ReleaseAllowed`; independently releasable operation state may release while dirty-integrity retains bounded capture evidence.

Request, coded parity/codeword, exact store, dirty-region, and checksum-extents geometries SHALL remain separately observable. The coded claim SHALL come from the captured topology/profile mapping and SHALL NOT be substituted with another geometry. The exact bounded coded relation is delegated to `models/quint/CodedRangeClean.qnt`, which receives externally supplied complete validated claims and owner-approved observations as inputs and is the sole authority for their overlap/disjoint classification, admission-before-dependent-effect, and coded removal conditional on exact `ReleaseAllowed`; external authority, topology mapping, geometry construction, lifecycle terminality, basis coherence, persistence, resources, sessions, and product claims remain outside it. Every consequential delegated distinction requires the projection-feasibility seam or named external owner/evidence.

#### Scenario: Complete coded claims are classified

- **WHEN** complete valid claims under one captured topology and profile intersect or remain disjoint
- **THEN** the coded owner classifies intersection as a coded conflict; disjoint claims are not a coded conflict and may coexist subject to other owners, with any bounded refusal returned before dependent I/O and other owners retaining their obligations

#### Scenario: Admission scope is incomplete or partially acquired

- **WHEN** a complete mapped coded claim is missing, stale, or only partial physical authority has been acquired
- **THEN** admission is refused before dependent basis I/O or mutation, and no later claim upgrade is inferred

#### Scenario: Basis and release facts remain externally owned

- **WHEN** a relevant basis observation needs authority through a consuming mutation, or the canonical lifecycle-release owner supplies exact `ReleaseAllowed` while a separate `CLEAN` capture remains unresolved
- **THEN** the observation is held coherently or discarded/reconciled before release, and the coded claim may be removed only under that authorization while dirty-integrity retains bounded capture evidence and CLEAN uncertainty remains independent
