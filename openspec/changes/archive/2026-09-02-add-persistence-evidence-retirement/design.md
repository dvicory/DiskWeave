## Context

See `proposal.md` for the motivation and selected direction. The current recovery snapshot stores an append-only `fences` vector. Protected-write completion, session close, integrity validation, dirty-region cleanup, coded-capture handling, and recovery reopen consume different exact fence values or store-fence bindings. Recovery transactions already carry an expected generation and topology epoch, apply mutations atomically, and clone a candidate snapshot before durable publication. The SQLite adapter persists the complete semantic manifest but does not own its meaning.

The current `DirtyRegionRecord` embeds a complete last-clean certificate, valid integrity state stores one store-fence reference, writable-session state embeds a global fence, and coded captures retain exact release certificates and closure evidence. These values are not interchangeable identity forms. The current coded-clean owner also requires exact retained clean-region evidence until its phase-specific cleanup is authorized.

The existing dirty-integrity owner remains authoritative for coded-capture membership, later-cut history, release receipts, and capture retirement. Store-operation-contracts remains authoritative for store-incarnation, capability, watermark, and fence admissibility. The healthy-portable-io service composes those facts. The active writable-session target and open `dwv-x6y.2.3` remain future/current-owner seams for closed-mutation-set session supersession, not inputs to this change's new policy.

## Goals / Non-Goals

**Goals:**

- Give recovery-state semantics one explicit exact-fence occurrence identity and retention boundary.
- Define every current and unresolved root with its owner, exact binding, creation, discharge, and rebind conditions.
- Preserve exact topology, store incarnation, capability, fence-domain, watermark, target, and generation bindings through retirement and reopen.
- Keep normal settled-write retention bounded independently of lifetime write count while retaining current clean/integrity/session claims as long as they remain current.
- Keep unrelated-fence retirement possible when one dependency remains unresolved.
- Make successor over-bound rejection conservative and observable.
- Make legacy interpretation an explicit semantic migration rather than a serde-default reinterpretation.

**Non-Goals:**

- No watermark-only dominance, aggregate coverage summary, or external evidence archive.
- No generic garbage-collection framework, retention service, or cross-owner lifecycle abstraction.
- No change to coded-capture membership/history compaction or future closed-mutation-set session supersession.
- No historical/audit retention product; only a bounded `legacy-unreconciled` migration root is introduced.
- No pre-mutation serving reservation protocol in this change; `dwv-x6y.2.2` remains the prerequisite for enabling ordinary serving implementation.
- No claim that process-local reachability, serialized DTO equality, a timer, or a latest-N policy can authorize durable deletion.

## Decisions

### 1. Use stable semantic occurrence identity

Each complete persisted composite certificate receives a monotonic `FenceOccurrenceId` within its recovery lineage. The identity is persisted, never reused, and is the only identity used by roots and retirement. Equal certificate values remain distinct occurrences; vector position and serialized bytes are not identity. Existing per-store `FenceId` values remain store evidence bindings and do not replace the composite occurrence identity.

New certificates use canonical ordering for store references and captured region/integrity pairs and reject duplicate identities rather than silently deduplicating them. Migration preserves legacy multiplicity and assigns occurrence identities from the validated legacy sequence. A root binds both the occurrence identity and the exact certificate facts it relies on, preventing stale-ID or altered-value reuse.

### 2. Define owner-complete root classes

Recovery stores the exact retention registry and validates root transitions, but it does not infer another owner's completion. The semantic root contract is:

- Dirty-integrity creates a current-clean root for each durable `CLEAN` region, bound to occurrence, region, and clean generation. A transition to `Dirty` or `Indeterminate` discharges or rebinds that root in the same successor; a copied last-clean value in a non-clean record is non-authorizing.
- The checksum owner creates an integrity root when a valid digest is installed, bound to occurrence, extent, profile/set, content generation, and digest validity. Stale/absent or newer-valid installation discharges or rebinds it.
- A persisted session-lifecycle root is created conservatively whenever a supported successor contains a writable-session global fence, including `CloseWritableSession`, migration, and reopen/import. It binds occurrence, session identity, and close generation. Replacing a closed session with `BeginWritableSession` carries that prior root forward rather than discharging or rebinding it. No current canonical session-lifecycle discharge owner exists, so this change neither discharges nor rebinds the root; closed-set/session-close supersession remains downstream.
- Dirty-integrity creates coded-capture roots for exact clean-closure and release-certificate bindings. Its existing phase-specific cleanup, membership compaction, and retirement authorities alone discharge them; open, commit-pending, and commit-unknown captures retain them.
- Recovery/adapters create an unresolved-recovery-commit root only when the adapter durably preserves an exact prior/proposed manifest and commit-intent identity for a may-have-committed recovery transaction. Known non-commit, exact durable successor, or authoritative reopen reconciliation discharges it; process-local operation state is not a durable root.
- The explicit schema migration creates only `legacy-unreconciled` roots for old occurrences and ambiguous copied references. This change stops at conservative read-only migration: it does not provide a migration-owner inventory proof, current-owner rebind, reset, or rebuild path. A migrated manifest with retained legacy roots remains stale/inspection-only until a separately scoped owner decision; callers cannot create arbitrary historical roots.

The liveness contract is therefore bounded by current owner roots and admitted unresolved work, not by the number of successful writes. A clean region that is never rewritten remains a legitimate live root; cleanup is not required to reduce the registry to zero.

### 3. Rebind before removing the last satisfying occurrence

Supersession is evaluated per root, not globally. Each rebind may target its own successor occurrence; a predecessor with multiple roots may therefore rebind those roots atomically to different retained successor occurrences. A newer occurrence can replace an older one only when it has the same topology epoch, fence domain, required store set, store incarnations, capability evidence, and exact target identity, and its generation/watermark evidence covers that particular root. Partial byte-range overlap, a scalar watermark, or generic certificate equality is insufficient. All root rebinds and occurrence removal happen in one successor proposal.

### 4. Reuse the existing atomic recovery boundary with exhaustive outcomes

Retirement is an ordinary generation/topology-checked recovery mutation carrying the exact predecessor, occurrence retirement set, root rebind/discharge set, and complete proposed successor. The outcomes are deliberately exhaustive:

- `durable successor`: successor is authoritative;
- `known rejected` or `known not committed`: predecessor remains authoritative;
- `lost`, `corrupt`, or `unclassifiable` acknowledgement that may have committed: neither candidate is assumed until exact reopen reconciliation establishes prior or proposed state.

The retirement transition never publishes a local successor before durable commit and never grants dependent data/parity, `CLEAN`, integrity, session, or release authority from an unresolved acknowledgement. A representability/export-bound rejection preserves the predecessor. The separate serving admission work must reserve capacity before the write-recovery record and retain that reservation through uncertainty before any protected mutation is enabled; this change neither defines nor implements that reservation.

### 5. Version legacy meaning explicitly

The current semantic schema is version 6. The new occurrence/root meaning requires a new schema version and an explicit migration step after version 6. Older manifests are migration-required before writable use. Migration assigns identities without coalescing duplicates, preserves exact certificate values, and retains ambiguous copied references as conservative legacy-unreconciled roots until an owner-qualified rebind. Interrupted migration uses the same prior/proposed reconciliation outcomes as retirement.

### Delegated bounded relation and evidence boundary

One shared executable model serves three disjoint owning requirement surfaces.
Its source-local markers are added when those target requirements become
canonical:

- `req.recovery-state-semantics.exact-persistence-evidence-follows-owner-qualified-claim-liveness`
  owns the bounded registry-liveness projection conditioned on supplied
  owner-qualified roots: predecessor retention, exact reachability, complete
  root discharge/rebind, and preservation of unrelated roots.
- `req.recovery-state-semantics.fence-occurrences-have-stable-exact-identities-and-coverage`
  owns monotonic non-reused occurrence identity, immutable certificate versus
  mutable owner-fact separation, exact root/fence binding, componentwise
  store/claim compatibility, and per-root rebind/discharge validation with an
  independent successor occurrence per rebind.
- `req.recovery-state-semantics.fence-retirement-preserves-an-exact-durable-predecessor`
  owns generation/topology-checked preparation, durable successor installation,
  known rejection, stale rejection, exact-intent unknown observation, and
  prior/proposed/neither reconciliation.

`models/quint/PersistenceEvidenceRetirement.qnt` is the sole exact authority
only within those named bounded surfaces. It receives owner-qualified
certificate facts, owner facts, admissions, rebind/discharge proofs, and
reopen observations as inputs. Their production and qualification remain
owned by the named requirements and their dependencies.

Owner liveness policy, serialized canonical ordering and malformed-input
duplicate detection, semantic schema migration, export and capacity bounds,
physical durability, adapter mechanics, and implementation conformance are
not delegated to the model. The focused, wide, and mutation analysis modules
are evidence-only configurations: the small analysis fixes four shapes, four
occurrences, four root IDs, and epochs 0 through 8; its plan, unknown, and
generic transition relations have separate bounded checks; the wide analysis
supplies five direct scenarios, not exhaustive coverage; and mutation
scenarios are complementary negative evidence, not implementation proof.

### 6. Keep the first implementation proportional to the owner boundary

The implementation may use a bounded exact index or an exact occurrence vector, but it must not add a generic retention framework. It must update all current exact lookup paths and remove the implicit rule that the top-level fence vector is permanent history. The first implementation remains whole-snapshot clone/serialization compatible, so serving is not enabled until `dwv-x6y.2.2` defines and verifies pre-mutation reservation and the retained-root envelope is measured.

### 7. Quint Connect correspondence gate

The Connect check is an implementation correspondence witness, not a second
semantic authority. The bridge may project finite inputs and production
results, but it cannot manufacture owner facts, durable outcomes, or root
liveness. The following table is the required map; rows marked non-claim are
excluded from Connect evidence rather than simulated.

| Quint declaration | Bridge direction | Exact production seam | Authority boundary |
| --- | --- | --- | --- |
| `Snapshot` | production → Quint | `RecoverySnapshot` and `RecoveryInspection` | Connect projects the full bounded durable snapshot; roots, certificates, store facts, owner facts, generation, topology, and allocated identity high-water marks are compared. Root topology/generation are additionally asserted by the bridge; canonical serialized ordering remains production validation. |
| `RetentionState`, `Phase`, `Outcome` | action-only / explicit non-claim | Quint scheduling actions and process-local recovery state | The model actions remain canonical for their relation, but Connect does not project phase, outcome, pending intent, or unresolved sidecar fields. |
| `Fence`, `CertificateBinding`, `StoreFact` | production → Quint | `FenceCertificate` and `StoreFenceRef`, including occurrence, topology, domain, store incarnation, capability, watermark, target, and coverage | Connect compares normalized exact fields; production snapshot validation remains responsible for canonical serialized ordering. |
| `Root`, `ClaimBinding`, `OwnerFact`, `Admission` | production → Quint | The exact owner-class paths in the table below, ending in a `RecoverySnapshot` load/diff | Connect includes only owner variants with a named capability or owner transaction and a post-commit root observation. It never derives authority from `IntegrityRecord` or another constructible DTO, calls recovery-internal proof builders as an owner, or mints facts. Recovery and legacy owner inputs remain explicit non-claims. |
| `init` | harness → Quint | bounded fixture construction from a known `RecoverySnapshot` | Model setup only; not production initialization or recovery authority. |
| `submitFence` | Quint input → production transaction, then production → Quint | `RecoveryMutation::RecordDataParityFence` through `RecoveryStateStore::commit_durable` | Recovery allocates the occurrence; Connect projects the assigned result instead of choosing an authoritative occurrence. |
| `createAnyFence` | explicit non-claim | none | Nondeterministic model convenience; excluded from correspondence. |
| `admitRoot` | production → Quint | owner transition result containing `RecoveryClaimRoot` | There is no generic production root-admission action. Connect must not add one or synthesize an admission. |
| `prepareUnboundRetirement` | recovery-validated staging → production transaction | The exact loaded predecessor, successor, and retired occurrence are retained in a process-local `RecoveryRetirementPlan`; production validates that plan when `RecoveryMutation::RetireFences` commits | This is not an owner-prepared capability. Connect never manufactures owner proofs or calls private recovery helpers as an owner. |
| `preparePlannedRetirement` | explicit non-claim until owner seam exists | No ordinary external producer currently exposes all per-root owner proofs | The model analysis covers planned proofs; Connect cannot claim them without a real owner-issued capability. |
| `commitPrepared` | production → Quint | `RecoveryStateStore::commit_durable` or `commit_durable_receipt` returning durable success | `Committed` is projected only after the production durable result. |
| `rejectPrepared` | production → Quint | `SqliteCommitFailurePoint::BeforeCommitIntentRename` drives the real adapter commit path to `CommitNotDurable(Rejected)` | The bridge observes the adapter's known rejection; it never calls `commit_observed` with a caller-selected outcome. |
| `rejectStale` | production → Quint | A real intervening `RecordMaintenanceCheckpoint` commit advances generation, then the prepared retirement returns `GenerationMismatch` or `TopologyMismatch` | The model schedule represents a production interleaving, not arbitrary snapshot mutation. |
| `observeRetirementUnknown` | production → Quint, service scope only | SQLite `AfterCommitIntent`/`AfterManifestWrite` return lost acknowledgement; `LostAckRecovery` injects service reactions | `LostAckRecovery` is a test double for service reaction, not adapter durability evidence. Corrupt acknowledgement has no Connect seam and is excluded. |
| `reconcilePrior` | production → Quint | `SqliteCommitFailurePoint::AfterCommitIntent`, then `SqliteRecoveryStore::open` and `RecoveryInspection` | The sidecar is reconciled to the exact prior manifest; no process-local choice. |
| `reconcileSuccessor` | production → Quint | `SqliteCommitFailurePoint::AfterManifestWrite`, then `SqliteRecoveryStore::open` and `RecoveryInspection` | The sidecar is reconciled to the exact proposed manifest; no process-local choice. |
| `reconcileNeither` | explicit non-claim | No maintained production fault seam produces an exact neither state | The model retains this conservative branch; Connect evidence does not claim it. |
| `advanceGeneration` | production interleaving → Quint | real `RecordMaintenanceCheckpoint` commit between retirement preparation and submission | No direct generation setter is mapped. |
| `advanceTopology` | production interleaving → Quint | real `PrepareTopology`/`CommitTopology` transition between preparation and submission | No direct topology setter is mapped. |
| `step` | explicit non-claim | none | The model scheduler is not a production executor or fairness claim. |

The root projection is intentionally owner-class-specific:

| Model owner fact | Owner-issued capability or transaction | Durable post-commit observation |
| --- | --- | --- |
| `Clean` / `CleanRegion` | `evaluate_recovery_clean` returning `RecoveryCleanDecision::Clear(RecoveryCleanPermit)`, followed by the production `RecoveryMutation::MarkRegionClean` transaction | Diff `RecoverySnapshot.dirty_regions` and `RecoverySnapshot.roots` for the exact `RecoveryRootFact::CleanRegion`, occurrence, generation, and certificate. |
| `Integrity` / `ValidIntegrity` | `ChecksumAuthority::commit` returning `CommitOutcome::Committed(ChecksumRecord)`, followed by the production `RecoveryMutation::InstallIntegrityDigest` transaction | Diff `RecoverySnapshot.integrity_records` and `RecoverySnapshot.roots` for the exact `IntegrityState::Valid` and `RecoveryRootFact::ValidIntegrity`; the DTO is evidence emitted by the checksum owner, not authority by itself. |
| `Session` / `WritableSession` | `RecoveryMutation::CloseWritableSession` submitted through `RecoveryStateStore::commit_durable_receipt` with the owner-produced global `FenceCertificate` | Diff `RecoverySnapshot.writable_session` and `RecoverySnapshot.roots` for the exact closed session, close generation, root occurrence, and fence. |
| `Capture` / `CodedCapture` | `CodedLifecycleAuthority::authorize_clean` returning `CodedCaptureCleanAuthorization`, then `prepare_clean_commit` returning `PreparedCodedOwnerTransition<PreparedCodedCleanCommit>`, then the production `ApplyCodedTransition` durable transaction and `confirm_clean_commit` | Diff `RecoverySnapshot.coded_captures` and `RecoverySnapshot.roots` for the exact capture phase, capture occurrence, coded root, and closure transition. |

`Recovery` / `UnresolvedRecoveryCommit` and `Legacy` /
`LegacyUnreconciled` have no ordinary owner-issued Connect producer in this
change and are excluded from the profile. A fixture may load them for a
non-authorizing negative check, but it must not present them as admitted owner
facts.

Connect uses a partial durable projection for actions whose private pending
state has no exact public observation. It compares:

- `prepareUnboundRetirement`: the exact predecessor, successor, and retired
  occurrence are retained in the recovery-validated plan; only the durable
  snapshot remains in the Connect projection, not `PendingState` or
  `RetirementIntent`.
- `commitPrepared` and `rejectPrepared`: production outcome plus the
  pre/post `RecoverySnapshot` generation, topology, fences, roots, and
  owner records; root topology/generation are checked by the bridge, while
  private intent payload remains excluded.
- `observeRetirementUnknown`, `reconcilePrior`, and `reconcileSuccessor`:
  the production inspection disposition and final loaded snapshot; it does
  not compare `UnresolvedState` or sidecar internals.

This partial projection is a stated non-claim, not an allowance to infer
private state from equal serialized values. Direct projection of
`PendingState`, `RetirementIntent`, and `UnresolvedState` remains excluded.

The Connect profile must use only the mapped focused bound: four shapes, four
occurrences, four root IDs, epochs 0–4, four fences, and four roots. The
canonical analysis uses its separate epochs 0–8 bound. Wide analysis and
mutation profiles remain separate evidence. Accepted paths must use the same
production recovery transaction and owner producers as ordinary code. A
Connect run must exclude `LegacyAdmission`, `RecoveryAdmission`,
`preparePlannedRetirement`, corrupt acknowledgement, and neither
reconciliation unless a later implementation adds and reviews the missing
production seams.
An adversarial review of this table, every production seam, and every
non-claim is a hard gate. Task 8.3 and its evidence remain blocked until the
reviewer returns `GO` with no blocking finding.

## Risks / Trade-offs

- **[Risk]** A certificate copied into a clean-region, integrity, session, or capture record can hide a live dependency from a top-level occurrence scan. **Mitigation:** every owner supplies an exact root binding; successor validation checks all root records and copied bindings before removal.
- **[Risk]** Equal certificate values or legacy duplicate positions can cause value-based retirement to remove the wrong occurrence. **Mitigation:** persist monotonic occurrence identity, preserve multiplicity, reject new internal duplicates, and migrate ambiguous old references conservatively.
- **[Risk]** An old clean-region value remains serialized after the region becomes dirty. **Mitigation:** the dirty transition discharges or rebinds the root atomically; the stale copied value is non-authorizing and cannot retain evidence alone.
- **[Risk]** A lost acknowledgement may follow a durable successor, so treating every known failure as predecessor-authoritative can use deleted evidence. **Mitigation:** distinguish known rejection/non-commit from lost/corrupt/unclassifiable may-have-committed outcomes and reconcile exact prior/proposed state.
- **[Risk]** Exact evidence can still exhaust finite capacity during genuinely unresolved dependent work. **Mitigation:** isolate unrelated retirement, preserve the causal root, and leave pre-mutation reservation/refusal to the explicit `.2.2` prerequisite.
- **[Risk]** Full-snapshot cloning and serialization remain proportional to retained roots during the first cutover. **Mitigation:** require stable retained-root counts and complete-successor byte measurements before enabling serving; avoid a generic index unless evidence requires it.
- **[Risk]** A semantic v7 migration may leave at-cap legacy roots that cannot be discharged automatically. **Mitigation:** report migration-required/reconciliation-required without writable mutation or truncation and provide owner-qualified next actions.

## Migration Plan

1. Land the canonical recovery-state delta and relationship edges for occurrence identity, root liveness, retirement outcomes, and explicit legacy migration.
2. Add the next semantic schema version and migration step. Existing version-6 manifests remain readable only through conservative migration/inspection; ordinary payload files are untouched, and this change accepts a migrated manifest remaining read-only.
3. Assign stable identities to legacy fence occurrences and preserve duplicate/ambiguous references under `legacy-unreconciled` roots. Do not use serde defaults to claim current semantics or claim that an inventory proof has discharged a root.
4. Implement exact retirement preparation and successor validation in the recovery owner; publish no local successor before durable confirmation.
5. Add owner-specific root creation, discharge, and rebind at current CLEAN and checksum boundaries, and preserve session-lifecycle roots on close successors, migration/import, and closed-session replacement without generic discharge. Connect coded-capture roots only through existing dirty-integrity authorities; leave closed-set session supersession to `dwv-x6y.2.3`.
6. Exercise known rejection, durable successor, lost/corrupt acknowledgement, reopen-prior, reopen-proposed, reopen-neither, stale generation, topology change, and bound-rejection cases before enabling normal retirement.
7. Keep serving implementation blocked on the separate `.2.2` pre-mutation reservation contract. After that prerequisite, enable settled-write retirement and measure root count, manifest bytes, refusal causes, and latency.
8. Roll back by disabling retirement preparation and reopening the validated predecessor or supported prior schema. No rollback path deletes payload bytes or silently drops recovery evidence.

Legacy-owner reconciliation remains outside this change. This change's migration
boundary is complete at stable occurrence identity, duplicate/ambiguous
preservation, and stale/read-only classification. It never deletes payload or
parity data and adds no reset, rebuild, or inventory-proof machinery.
Future work, separate from this change, targets no planned write pause for valid
supported legacy ledgers using scalable representation and bounded temporary
ledger space; degraded-upgrade behavior and the support window remain open there.

## Quint Connect implementation evidence

- `verification/quint/PersistenceEvidenceRetirementConnect.qnt` imports the
  reviewed retention relation and fixes the focused bound to one shape, four
  occurrences, four root IDs, and epochs 0–4. Its path covers a validated
  CLEAN root, a second occurrence, a private pending interval, and retirement
  of only the unbound occurrence.
- `crates/dwv-service/src/service/tests/persistence_evidence_retirement_connect.rs`
  drives `MemoryRecoveryStore` through `RecoveryStateStore`, obtains a
  `RecoveryCleanPermit`, commits production fence/CLEAN/retirement mutations,
  retains the exact recovery-validated retirement plan, and compares every
  field represented by the durable `Snapshot` projection: generation,
  topology, allocated occurrence/root identities, exact fence
  shape/store-fact/claim fields, and exact root id/occurrence/binding/fact/
  certificate fields. The bridge additionally asserts the exact CLEAN
  `DirtyRegionRecord` state, clean generation, last-clean fence, and root
  topology/generation. It does not project phase/outcome, private pending
  intent, SQLite fault seams, or physical payload durability, and it does not
  claim serialized ordering; production snapshot validation owns canonical
  order.
  Model fence claims are deterministic projections of captured certificate
  facts and loaded owner roots, not a second production registry.
- Verification evidence: `quint typecheck
  verification/quint/PersistenceEvidenceRetirementConnect.qnt`, bounded
  `quint run` with seed `22082026`, and the two-seed
  `cargo test -p dwv-service --lib persistence_evidence_retirement_connect
  -- --test-threads=1` run all passed. The profile is correspondence evidence
  for this mapped path, not arbitrary-width or physical-durability proof.

## Open Questions

Future online migration is a separate design question. This change does not
choose a support window, degraded-upgrade behavior, or scalable ledger layout;
the separate representability and range-local work must define those before
promising no planned write pause.
