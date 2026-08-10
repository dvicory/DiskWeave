# Milestone 8 Canonical Request Boundary Implementation Review

## Review target

Active OpenSpec change: `normalized-service-request-boundary`

Review the full current working tree, not only the change artifacts. The tree contains earlier accepted Milestone 8 work plus this clean request-boundary cutover. Do not treat unrelated pre-existing edits as defects unless the current implementation breaks their accepted contracts.

## Verdict and external review

**APPROVED AFTER EXTERNAL REVIEW.** The implementation satisfies the change tasks and the Milestone 8 WP-8.4 acceptance boundary. Two independent final reviews of the current implementation found no remaining CRITICAL, HIGH, or MEDIUM defect. The change remains intentionally active and unarchived; `milestone-planning-cutover` has not started.

Final audit found and fixed three correctness gaps before staging this review:

1. `HealthyPortableService::flush` now rejects a parity-slot target just as read/write do.
2. Assembly now checks the opened `FileStore` topology epoch against the explicit member binding and captured topology.
3. A flush admission failure after partial child registration now reconciles and releases the child resources and operation slot.

The first independent review pass found two defects:

1. **HIGH:** admitted read/write/flush terminal failures reclaimed the operation slot without returning the exact canonical `BlockRequest`.
2. **MEDIUM:** the cutover removed lower-level `dwv_store::OperationId` and `StoreRequest::operation_id` despite the design retaining that backend-only identity.

Both were fixed before approval. Every post-reservation service error now owns the exact `BlockRequest` and exposes it through `ServiceError::request()`; short-read and admitted write/flush failure regressions assert whole-value equality. `OperationId` and `StoreRequest::operation_id` are restored at all constructors without becoming a second semantic request model. The request stored in terminal errors is boxed only to keep `ServiceError` below Clippy's large-error threshold; this allocation occurs on terminal error construction, not on the Linux write-payload path.

Both reviewers independently re-read the final implementation after these fixes and reported no remaining CRITICAL, HIGH, or MEDIUM defect. The current source was then rerun through the fresh Linux ublk/ext4 acceptance workflow so retained platform evidence matches the final implementation.

## Scope and non-scope

Implemented:

- one canonical semantic request type, `dwv_core::BlockRequest`, at the portable service boundary;
- a separately borrowed write payload;
- exact canonical request retention in operation slots and terminal service evidence;
- explicit stable topology/member bindings and order-independent lookup;
- request target resolution by stable slot and captured topology identity;
- frontend/request identity kept separate from operation-slot and child-operation identity;
- fail-closed operation, topology, assignment, role, generation, store identity, store epoch, capability, and payload validation;
- independent ordering/preflush and durability intent validation;
- abandonment as loss of delivery interest, not cancellation or early reclamation;
- copy-free Linux request-boundary payload translation;
- deterministic conformance coverage and refreshed Linux ublk/ext4 evidence.

Not changed:

- parity/update/recovery algorithms;
- persistent formats or recovery schemas;
- production SQLite selection;
- physical power-loss or hardware durability claims;
- FUA, discard, write-zeroes, zoned, degraded-write, or online-topology support;
- asynchronous service APIs or buffer frameworks;
- Milestone 8 planning-history migration (`milestone-planning-cutover`).

## Canonical requirement closure

The change preserves stable semantic IDs and modifies exactly these six current requirements:

| Semantic ID | Closure |
|---|---|
| `req.normalized-block-semantics.requests-have-validated-frontend-neutral-semantics` | `BlockRequest` is the sole service request; all fields remain observable through terminal reconciliation. |
| `req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch` | Every opened store is bound to one stable assignment identity and captured epoch. |
| `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments` | Missing, extra, duplicate, aliased, stale, and mismatched bindings fail closed. |
| `req.healthy-portable-io.assembly-and-request-admission-are-bounded-and-identity-safe` | Assembly and admission use explicit bindings and the canonical request without positional authority. |
| `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations` | Slots atomically retain the exact request, real buffer token, children, terminal state, and reconciliation ownership. |
| `req.linux-ublk-frontend.kernel-requests-preserve-normalized-semantics` | Translation returns the canonical request and borrows the exact bounded frontend write payload. |

The canonical inventory remains 152 requirements and 68 relationships. Every changed fingerprint and effective dependent was resolved individually in `docs/reviewed-requirements.toml`; all readiness gate counts are zero.

## API before and after

### Removed API

The prior service contract was:

```rust
pub fn read(
    &mut self,
    request: PortableRequest,
) -> Result<(Vec<u8>, OperationEvidence), ServiceError>;

pub fn write(
    &mut self,
    request: PortableRequest,
) -> Result<OperationEvidence, ServiceError>;

pub fn flush(
    &mut self,
    request: PortableRequest,
) -> Result<OperationEvidence, ServiceError>;
```

`PortableRequest` used a positional `usize` data slot, embedded owned write bytes, forced or reconstructed identity fields, and duplicated canonical operation meaning through `RequestOperation`. `PortableRequest::from_block_request`, synthetic request constructors, `RequestOperation`, `PortableRequest`, their exports, and `crates/dwv-service/src/request.rs` are deleted. No compatibility wrapper or replacement mirror request remains.

The prior operation-slot boundary was effectively:

```rust
pub fn reserve(
    &mut self,
    operation_id: OperationId,
    topology_epoch: TopologyEpoch,
) -> Result<OperationSlotToken, SlotError>;
```

It retained the weaker scalar operation identity and synthesized buffer identity rather than owning the admitted canonical request.

The prior service assembly used `MemberStore` plus positional data/parity collections. Semantic member selection depended on collection position.

### Current API

```rust
pub fn read(
    &mut self,
    request: BlockRequest,
) -> Result<(Vec<u8>, OperationEvidence), ServiceError>;

pub fn write(
    &mut self,
    request: BlockRequest,
    bytes: &[u8],
) -> Result<OperationEvidence, ServiceError>;

pub fn flush(
    &mut self,
    request: BlockRequest,
) -> Result<OperationEvidence, ServiceError>;
```

The operation slot now owns the exact request atomically with its real optional buffer token:

```rust
pub fn reserve(
    &mut self,
    request: BlockRequest,
) -> Result<OperationSlotToken, SlotError>;
```

`SlotSnapshot` exposes `request: BlockRequest`. `OperationEvidence` exposes:

```rust
pub struct OperationEvidence {
    pub request: BlockRequest,
    pub completion: CompletionEvidence,
    pub trace: dwv_transaction_ref::Trace,
}
```

Service assembly now accepts one order-insensitive binding collection:

```rust
pub fn open(
    topology: TopologySnapshot,
    members: Vec<MemberBinding>,
    recovery: R,
    config: ServiceConfig,
) -> Result<Self, ServiceError>;

pub fn MemberBinding::new(
    assignment: &TopologyAssignment,
    topology_epoch: TopologyEpoch,
    store_id: StoreId,
    store: FileStore,
) -> MemberBinding;
```

Each binding captures stable slot, role, coding position, assignment instance, assignment generation, topology epoch, store identity, and the opened store. `FileStore::topology_epoch()` makes the opened store epoch independently checkable during assembly.

Lower-level `dwv_store::OperationId` remains only where it identifies a backend `StoreRequest`; it is not a second logical request model. `OperationSlotToken` and generation-bearing `ChildOperationId` remain internal resource/backend identities.

## Invariant mapping

### Canonical request identity and fields

`BlockRequest` remains the sole semantic owner of:

- `RequestId`;
- `FrontendId`;
- `SlotId`;
- `TopologyEpoch`;
- `BlockOp`;
- checked `ByteRange`;
- optional real `BufferToken`;
- `SubmissionSequence`;
- `preflush`;
- `FenceDomain`;
- `DurabilityIntent`.

| Field | Admission owner | Terminal observability |
|---|---|---|
| `RequestId` | exact `BlockRequest` in `OperationSlotTable` | `OperationEvidence.request` or `ServiceError::request()` |
| `FrontendId` | exact `BlockRequest` in `OperationSlotTable` | same whole request; cross-frontend collision regression |
| `SlotId` | stable topology lookup before reservation | same whole request; selected binding checked independently |
| `TopologyEpoch` | request and member binding validation | same whole request |
| `BlockOp` | capability and endpoint validation | same whole request |
| checked `ByteRange` | request validation and range plan | same whole request plus completion range evidence |
| optional `BufferToken` | acquired atomically with the slot | retained until terminal reconciliation; same whole request |
| `SubmissionSequence` | exact slot-owned request | same whole request |
| `preflush` | independent ordering validation | same whole request |
| `FenceDomain` | independent fence validation | same whole request |
| `DurabilityIntent` | independent durability validation | same whole request and explicit persistence claim |

The same value passes through service validation, operation-slot reservation, execution, slot snapshots, successful terminal `OperationEvidence`, and admitted terminal failures through `ServiceError::request()`. Whole-value equality regressions cover all fields. Equal numeric request IDs from distinct frontends reserve distinct slots and retain distinct canonical requests.

### Stable member binding

Assembly:

- validates the `TopologySnapshot`;
- requires exactly one binding for every assignment and no extras;
- rejects duplicate slots and duplicate store IDs;
- rejects role, coding-position, assignment-instance, assignment-generation, and binding-epoch mismatch;
- rejects opened-store ID and opened-store topology-epoch mismatch;
- rejects aliased or ambiguous physical identities;
- requires compatible length, block size, read, write, and durable-flush capabilities.

Execution resolves `request.slot_id` through `TopologySnapshot::assignment_for_slot`, then finds the exact binding by slot/role/coding/assignment identity. Data and parity selection never use input-vector or discovery order. Checksum extent identity is derived from `CodingPosition`, not assignment index.

Read, write, and flush require a data-role target. A parity, missing, stale, mismatched, or unsupported target is rejected before reservation or payload mutation.

### Ordering and durability

`BlockRequest::validate` runs before reservation with the service capability profile. Preflush and durability remain independent fields. The current profile rejects unsupported preflush, FUA, discard, write-zeroes, zoned, malformed range/buffer, and invalid operation/durability combinations rather than weakening them. Flush requires an empty range, no buffer, explicit-flush intent, and a data-slot target. Successful service flush evidence remains `HostFenceOnly`; it does not claim physical durability.

### Abandonment and resource lifetime

Operation reservation acquires the operation slot and exact frontend buffer token atomically. Buffer-admission failure rolls back the slot. Abandonment only sets delivery interest false; admitted child work, frontend/internal buffers, terminal evidence, drain, and reconciliation obligations remain owned until safe reclamation. Stale generations and stale/duplicate completions fail closed. A partially admitted flush now reconciles registered children and releases all slot/backend resources on failure.

### Linux borrowed payload

`translate_request` returns the canonical `BlockRequest`; it does not create a service-local request. `borrowed_write_payload` returns a bounded subslice of the frontend buffer. The pointer/length regression proves identical storage. Linux dispatch keeps the ublk buffer alive across the synchronous service call. No request-boundary `Vec` copy or `bytes.to_vec()` remains in the frontend/service path. Per-chunk transaction-engine buffers remain intentionally out of scope.

## Main implementation surfaces

- `crates/dwv-core/src/lib.rs`: canonical request validation and generated round-trip coverage.
- `crates/dwv-store/src/lib.rs`: exact request ownership in `OperationSlotTable`/`SlotSnapshot`, generational child identity, resource lifetime tests, corrected semantic owner marker.
- `crates/dwv-store-file/src/file_store.rs`: opened-store topology epoch accessor used by assembly validation.
- `crates/dwv-service/src/admission.rs`: canonical request reservation and abandonment/reconciliation behavior.
- `crates/dwv-service/src/evidence.rs`: exact admitted request in terminal evidence.
- `crates/dwv-service/src/failure.rs`: exact admitted request retention and retrieval for post-reservation terminal failures.
- `crates/dwv-service/src/service.rs`: explicit `MemberBinding`, assembly validation, stable-slot resolution, canonical read/write/flush APIs, separate borrowed write bytes, fail-closed cleanup.
- `crates/dwv-service/src/lib.rs`: clean exports; obsolete request exports removed.
- `crates/dwv-service/src/request.rs`: deleted.
- `crates/dwv-frontend-ublk/src/lib.rs`, `fixture.rs`, `linux.rs`: canonical translation/dispatch and borrowed write payload.
- `src/demo.rs`: root demo caller migration.
- `openspec/changes/normalized-service-request-boundary/**`: proposal, design, tasks, and capability deltas.
- `docs/reviewed-requirements.toml`: individual changed-owner/dependent review outcomes.
- `docs/verification/linux-ublk-ext4-acceptance.md` and `verification/linux-ublk-*`: refreshed final-source Linux acceptance evidence.

## Deterministic regression evidence

Representative contracts now covered:

- every canonical request field survives slot reservation and terminal evidence;
- equal numeric request IDs from different frontends do not collide;
- buffer acquisition and slot acquisition are atomic;
- stale slot generations and stale/duplicate completions are rejected safely;
- abandonment before and throughout child completion retains resources until reconciliation;
- member binding order and topology assignment order do not change the selected stores;
- a two-data/one-parity fixture proves the non-zero/non-positional case;
- parity-slot redirection is refused for write and flush;
- stale assignment generation, assignment instance, store ID, and opened-store epoch are refused;
- write payload length mismatch is refused before mutation;
- unsupported operation, preflush, FUA, malformed range, and flush combinations fail closed;
- Linux write translation returns the exact borrowed frontend pointer and length;
- flush child-admission exhaustion leaves zero operation-slot and backend-submission usage.

Admitted short-read, write child-admission failure, and flush child-admission failure regressions additionally prove exact terminal request retention after slot reconciliation.

## Validation evidence

### Review diff capture

- `jj diff --stat`: 28 files changed, 4,625 insertions, 3,945 deletions.
- The full current-tree `jj diff` was captured for review; both independent reviewers were instructed to assess the complete tree, not a narrowed patch.

### Passed exactly as requested

- `openspec validate --all --strict`: 26/26 items valid.
- `openspec validate normalized-service-request-boundary --strict`: valid.
- `cargo check --workspace --all-targets`: passed.
- `cargo test --workspace --all-targets`: 298 passed across 22 suites; 1 hardware-dependent test ignored.
- Focused suites: `dwv-core` 15 passed; `dwv-store` 15 passed; `dwv-service` 35 passed with 1 hardware-dependent test ignored; `dwv-frontend-ublk` 12 passed; `cli_demo` 3 passed; `xtask` 32 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- `cargo xtask docs build`: passed; 152 objects.
- `cargo xtask docs check`: `ok: true`, `ready: true`; 152 requirements; every readiness gate count zero.
- `cargo xtask docs clean-room`: equivalent; digest `1272ec5c70816efa0e9429a80367e65e90ead765960039456618e8a0c12b4c66`.

The docs change-impact report still labels the deliberate `dwv-store` marker reassignment `review_required`, while the top-level readiness result is green. `cargo xtask docs knowledge affected --path crates/dwv-store/src/lib.rs` was run, and each changed owner/dependent was reviewed and resolved individually with a concrete reason. This is a reviewed relationship reassignment, not an uncovered semantic owner.

### Miri command divergence

The two required literal commands were executed:

```text
cargo miri test -p dwv-core
cargo miri test -p dwv-store
```

Both exit before compilation because stable `aarch64-apple-darwin` has no Miri component. The installed compatible nightly was then used:

```text
cargo +nightly-2025-11-21 miri test -p dwv-core
cargo +nightly-2025-11-21 miri test -p dwv-store
```

Result: 15/15 `dwv-core` tests and 15/15 `dwv-store` tests passed under Miri; both doc-test sets passed.

### Replay command divergence

The required literal command was executed:

```text
cargo xtask verify replay verification/traces/ve-002-normalized.json
```

It exits with the current xtask usage contract, `cargo xtask docs <operation>`; the requested `verification/traces/ve-002-normalized.json` route is not part of the current repository CLI. The authoritative retained Linux traces were replayed through the documented root CLI instead:

```text
cargo run -q --bin dwv -- demo disk trace-replay --trace verification/linux-ublk-trace-first.json
cargo run -q --bin dwv -- demo disk trace-replay --trace verification/linux-ublk-trace-second.json
```

Both passed clean deterministic replay: 452 and 81 records respectively.

## Fresh Linux ublk/ext4 acceptance

Executed from the current runtime source:

```text
tools/linux-disk-acceptance/run.sh verification/linux-ublk-ext4-acceptance.json
```

Result: passed on Ubuntu arm64, kernel `7.0.0-28-generic`, real `/dev/ublkb0`.

Retained evidence:

- source snapshot SHA-256: `161b38983951cbbddf31d2ab6d616aaed41222e5a7a8d4969806ce858bb92426`;
- first trace: 445,541 bytes, 453 records, SHA-256 `1ecf091313fdcdd1907298a20a0d5549a492e52b5ee2c1358133323945f05171`;
- second trace: 79,530 bytes, 81 records, SHA-256 `9793698021ba48cace5f2c3d4f115282918452964e6ef1919bb2642f7986b726`;
- acceptance JSON SHA-256: `34ccc737742b094ac978dcdb6baa7c8fb48e04292d33135b76ff6d73562732b9`;
- durable file content before/after restart: `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`;
- data/parity backing payload SHA-256: `7a088635b8c3680d0cb79f527a9f74ca326ffbe88c9e152c3979f84fd511718c`; byte equality passed;
- both service runs ended `stopped` after drain/checkpoint/removal/replay;
- guest `dwv-frontend-ublk`: 12/12 tests passed;
- guest partial multi-store fence regression passed;
- undersized geometry, unsupported topology, second-owner acquisition, conflicting cleanup, stale readiness, recovery-authority loss, unsupported discard, unknown cleanup, and owner death remained fail closed;
- protected payloads remained unchanged across pre-publication refusal cases.

The retained evidence continues to deny production durability, power-loss safety, multi-device publication, daemon recovery, FUA, discard, write-zeroes, online topology mutation, and hardware safety.

## Relationship closure

Canonical owner wording changed only where this cutover adds owner-local observability/borrow requirements. Existing request, topology, operation-slot, ordering/durability, abandonment, healthy-service, and Linux adapter owners remain authoritative. The `OperationSlotTable` implementation marker now points to `req.store-operation-contracts.operation-slots-own-backend-lifetimes-and-generations`; service assembly/admission remains linked from `dwv-service`. All effective dependent fingerprints were reviewed individually. No current requirement is uncovered, unknown, orphaned, or semantically stale.

## Reviewer acceptance criteria

Approve only if the current tree proves all of the following:

1. `BlockRequest` is the only service semantic request; no mirror/shim remains.
2. Write payload is separate and borrowed at the frontend/service boundary.
3. Every canonical request field survives admission and terminal evidence.
4. Frontend/request identity cannot collide with slot/child identity.
5. Member collection or topology assignment order cannot redirect I/O.
6. Every binding matches stable slot, role, coding position, assignment instance/generation, topology epoch, store ID, opened-store epoch, capabilities, and non-alias identity.
7. Read/write/flush reject non-data targets before admission or mutation.
8. Unsupported ordering/durability/operation combinations fail closed.
9. Abandonment and failures do not release owned resources before terminal reconciliation.
10. No Linux request-boundary payload allocation/copy remains.
11. Tests, strict specs, docs/readiness, Miri-equivalent nightly runs, retained trace replay, and fresh Linux evidence support the claims above.
12. No new persistent-format, SQLite-production, physical-durability, or broader frontend claim was introduced.

## Reviewer response format

Return findings ordered by severity with exact file/line and violated invariant. Explicitly state whether any CRITICAL or HIGH issue remains. Distinguish implementation defects from the two recorded command/environment divergences above. Do not request `milestone-planning-cutover` work in this review.

## Final status

Milestone 8 implementation remains pending: keep the independently approved `normalized-service-request-boundary` change active and unarchived until archive is explicitly authorized; do not start `milestone-planning-cutover`.
