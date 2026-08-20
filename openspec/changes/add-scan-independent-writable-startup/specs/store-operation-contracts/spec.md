## ADDED Requirements

### Requirement: Admission stabilization evidence is typed and store-scoped
<!-- dwv:req req.store-operation-contracts.admission-stabilization-evidence-is-typed-and-store-scoped -->
<!-- dwv:requires req.store-operation-contracts.capability-evidence-determines-the-allowed-safety-profile -->
<!-- dwv:requires req.store-operation-contracts.stores-report-exact-range-outcomes-and-persistence-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence -->
<!-- dwv:requires req.store-operation-contracts.store-failures-are-conservative-and-testable -->
<!-- dwv:requires req.file-backed-stores.single-writer-ownership-and-endpoint-aliasing-are-explicit -->
<!-- dwv:requires req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch -->

The store-operation boundary SHALL be the canonical producer of the cross-custody physical no-late fact and SHALL report it as typed, store-scoped `AdmissionStabilizationEvidence`. The applicable file-backed/store ownership owner SHALL be the sole allocator and revalidator of the exact current writer claim or equivalent exclusion and SHALL expose only an opaque exact claim token to stabilization consumers. Before stabilization, that owner SHALL acquire the claim and exact stable backing-resource/alias binding, then hold both continuously while the observation is established and through every later admission-premise revalidation. The store boundary SHALL bind the exact current claim token, current store incarnation, current ordering-domain identity, stable backing-resource identity and alias set, selected capability/profile, that profile's declared and evidence-supported physical reachability universe, assignment slot and instance, assignment generation, protected geometry, topology epoch, monotonic stabilization boundary, and exact predecessor physical-reachability/closure scope within that universe. Consumers SHALL separately hold and freshly revalidate writable-recovery authority through every boundary; this requirement does not allocate, release, or reconcile that authority.

For that exact current binding, `success` SHALL mean only that every pre-boundary effect in the exact predecessor physical-reachability/closure scope within the selected capability/profile's declared physical reachability universe has completed or ceased before the stabilization boundary, so none can mutate the stable backing resource after the boundary. This is a quiescence observation only: it does not establish that already-visible bytes reached persistent media or strengthen any persistence or durability evidence. A new store incarnation, process loss, lease acquisition, or a fence limited to the current ordering domain SHALL NOT establish this result. The boundary SHALL be monotonic while the exact current claim and stable-resource/alias binding remain continuously held: a later successful boundary covers at least the physical routes and effects covered by an earlier successful boundary, without covering effects initiated after the named boundary. It SHALL NOT be wall-clock time, a guessed timestamp, an accepted-write watermark, a persistence fence, or a cross-store fence. The bounded scope SHALL identify or otherwise certify closure of the complete declared physical reachability universe for the selected profile; it SHALL NOT enumerate payload, require persistent per-operation history, or imply an unbounded history ledger. Routes or actors explicitly outside that selected profile's declared universe remain explicit profile assumptions and non-claims; stabilization SHALL NOT silently include them, exclude an in-profile route, or broaden the selected profile.

The result SHALL have exactly one disposition: `success`, `unsupported`, `failed`, or `uncertain`. `unsupported` means the bound store/profile cannot identify and close, or otherwise certify closure of, every capable physical route/effect required by its declared physical reachability universe. `failed` means the attempt definitely did not establish that closure. `uncertain` means interruption, disappearance, timeout, claim loss, resource replacement or remap, alias-set change, an unknown or unattributed in-profile route, incomplete physical-source coverage, or an unclassifiable adapter observation prevents proving whether it was established. None of these three dispositions supplies success. An interrupted attempt SHALL remain uncertain unless the store owner independently establishes a successful exact observation. Startup SHALL NOT automatically retry or upgrade an uncertain observation. Any later attempt SHALL have the ownership owner revalidate the binding, acquire and continuously hold the exact then-current claim and resource/alias binding, and produce fresh evidence with a fresh boundary; prior uncertain evidence does not transfer to that attempt.

Writer-claim acquisition alone SHALL NOT satisfy stabilization: crash-releasing exclusion prevents a concurrent current writer but does not prove that effects issued through predecessor or unattributed physical routes within the selected profile's declared universe have completed or ceased. Recovery or operation history SHALL NOT substitute for stabilization: history can preserve intent, identity, disposition, or uncertainty but cannot by itself observe that old kernel/device queues, DMA, raw/out-of-band paths, aliases, remaps, or other physical effects included by that universe have stopped reaching the stable backing resource. Still-owned in-process requests, children, resources, completions, and reconciliation remain owned by `OperationSlotTable` and its related owners. Stabilization SHALL respect those live facts when they can produce effects within the selected profile's universe and when determining the exact physical reachability/closure scope, but it SHALL NOT copy their policy, infer their outcome or release, or require their identities to survive a reboot in a persistent ledger.

`evidence_strength` has no local policy meaning beyond preserving provenance, exact-binding checks, mismatch detection, and non-transfer; it SHALL NOT select, upgrade, or downgrade an admission profile unless another named owner defines that policy. Admission evidence SHALL remain specific to its opaque exact current claim token, current incarnation, current ordering domain, stable backing-resource/alias binding, selected capability/profile and declared physical reachability universe, and startup premise. Predecessor physical closure is coverage supporting only that bound stabilization result; it SHALL NOT transfer current authority to or from any predecessor source. The ownership owner SHALL freshly revalidate that the exact claim token and resource/alias binding remain continuously held after stabilization, after each checksum, range-role basis, separate coded-range-authority, and recovery `CLEAN` owner gate, session begin, epoch commit, admission return, frontend handoff, and publication; any loss, replacement, remap, alias change, release, or reacquisition invalidates the evidence and fails closed.

Successful `AdmissionStabilizationEvidence` proves only the bounded physical no-late fact. It SHALL NOT prove the outcome of any operation, persistence or durability, payload readability or correctness, parity agreement, checksum validity, recovery `CLEAN`, lineage or custody continuity, range basis, `Current`, current protection, operation-slot reclamation, transaction release, or any other owner's state transition.

#### Scenario: A store establishes the physical no-late boundary

- **WHEN** the exact current claim token and stable backing-resource/alias binding remain continuously owner-held, every later premise boundary is freshly revalidated, and the store reports success for its current incarnation, ordering domain, selected capability/profile and declared physical reachability universe, assignment and topology generation, monotonic boundary, and exact physical reachability/closure scope within that universe
- **THEN** startup may consume that evidence only for the matching current binding and premise and may conclude only that every covered physical route/effect ceased before the boundary and cannot mutate the stable backing resource afterward

#### Scenario: Writer exclusion exists without physical stabilization

- **WHEN** startup acquires the current claim token and stable-resource/alias binding, including for a new incarnation or ordering domain, but has no successful exact closure observation covering every physical route/effect within the selected profile's declared universe that is still capable of reaching the same stable backing resource
- **THEN** writable admission remains unavailable because current exclusion and identity alone do not prove that predecessor effects have completed or ceased

#### Scenario: Recovery history records a prior operation

- **WHEN** recovery or operation evidence names a prior operation and its last known disposition but the store has no successful exact stabilization observation
- **THEN** the history does not establish physical quiescence and startup neither scans payload nor creates a persistent operation ledger to replace stabilization

#### Scenario: Stabilization names another claim, incarnation, or scope

- **WHEN** a result names a different current claim token, store identity, incarnation, ordering domain, stable backing resource or alias set, assignment, topology epoch, capability/profile, boundary domain, or effect scope than the freshly revalidated startup premise
- **THEN** the result is stale or mismatched and cannot satisfy baseline writable admission

#### Scenario: An unknown physical route can still reach the backing resource

- **WHEN** an adapter binds the exact current claim, incarnation, ordering domain, stable resource, and selected capability/profile but cannot identify and close, or otherwise certify closure of, an old kernel/device queue, DMA, raw or out-of-band path, alias/remap, unattributed effect, or any other route within that profile's declared physical reachability universe that may still reach the resource
- **THEN** it reports `unsupported` or `uncertain` as applicable, baseline writable admission remains unavailable, and no current authority is granted to or inferred from predecessor coverage

#### Scenario: A predecessor domain can still reach the backing resource

- **WHEN** an adapter identifies the current claim, incarnation, and ordering domain but cannot close, or otherwise certify closure of, a predecessor claim, incarnation, or ordering domain whose effects may still reach the same stable backing resource
- **THEN** it reports `unsupported`, `failed`, or `uncertain` as applicable, baseline writable admission remains unavailable, and no current authority is granted to or inferred from predecessor coverage

#### Scenario: The binding changes after successful stabilization

- **WHEN** the claim is lost, released, or reacquired, or the stable resource is replaced, remapped, or gains a different alias after stabilization and before or during canonical checksum or range-role basis checks, separate coded-range-authority or recovery `CLEAN` owner checks, session begin, epoch commit, admission return, frontend handoff, or publication
- **THEN** fresh revalidation fails, the evidence is invalid, no writable endpoint is published, and the caller obtains fresh stabilization under a newly exact binding or follows the applicable owner reconciliation without treating either separate coded-range or recovery `CLEAN` observation as a substitute

#### Scenario: A store cannot establish the required profile

- **WHEN** a required store reports `unsupported`, `failed`, or `uncertain`, or its evidence is incomplete, stale, conflicting, or non-monotonic within its declared domain
- **THEN** baseline write-safe admission is refused or limited only by an independently named weaker profile, and no caller infers the physical no-late fact

#### Scenario: One store is stabilized and another is not

- **WHEN** all but one required store have successful evidence under the captured startup premise
- **THEN** the complete baseline set remains unavailable and partial evidence does not authorize writable admission or publication

#### Scenario: Stabilization is interrupted

- **WHEN** process loss, store disappearance, timeout, claim loss, or an unclassifiable adapter result interrupts stabilization
- **THEN** the observation is uncertain, baseline writable admission stops, and any later attempt reacquires and holds its exact current claim token and resource/alias binding and separately reacquires/holds writable-recovery authority, producing fresh evidence rather than reusing or automatically upgrading the interrupted result

#### Scenario: Stabilization succeeds without stronger claims

- **WHEN** the physical no-late boundary succeeds but operation outcome, durability, payload correctness, parity, checksum, `CLEAN`, continuity, basis, `Current`, protection, or release evidence is absent
- **THEN** those dimensions remain unchanged and under their existing independent owners
