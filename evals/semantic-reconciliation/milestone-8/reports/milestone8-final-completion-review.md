# Superseded Milestone 8 pre-archive agent review

## Provenance

**Agent self-review; not an external verdict. Superseded by the post-archive follow-up package.**

This file records the pre-archive assessment only. Its product findings remain historical review context; its repository-state claims are not final.

Current state is deliberately pre-archive:

- Archived changes:
  - `openspec/changes/archive/2026-08-09-canonical-semantic-ownership/`
  - `openspec/changes/archive/2026-08-10-metadata-certificate-authority-gate/`
  - `openspec/changes/archive/2026-08-09-normalized-service-request-boundary/`
- Active change: `milestone-planning-cutover`
- Tasks: **23/24 complete**
- Remaining task: archive the change after user approval
- No later roadmap work has begun.

## Review checkpoints

| Checkpoint | Provenance | Recorded result |
|---|---|---|
| Canonical semantic ownership initial review | Agent/subagent implementation review | Rejected pending correction of an inaccurate fingerprint description and bounded Linux evidence-remap verification. |
| Canonical semantic ownership follow-up | Agent/subagent implementation review | Production fingerprint behavior was already correct; regressions were added. Only the retired umbrella Linux requirement was removed from the evidence mapping. |
| Metadata certificate authority review | Agent/subagent implementation review | No caller-forgeable recovery certificate remained before archive. |
| Canonical request boundary review | Agent/subagent implementation review | No critical, high, or medium request-boundary defect remained before archive. |
| Planning cutover review | Agent/subagent implementation review | The incomplete superseded archive was deleted and stale machine evidence was replaced. |
| Final integrated review | Agent self-review | Recommended archive, subject to user approval. This was not external completion approval. |

## Finding dispositions

| Finding | Final disposition |
|---|---|
| **F-001 / CR-001** | Resolved. `dwv_core::BlockRequest` remains canonical through admission and terminal evidence. Stable `SlotId` resolves through immutable topology and `MemberBinding`; callers do not supply positional target authority. |
| **F-002** | Resolved. `MetadataLossVerification` exposes no caller-constructible certificate. Receipt-gated authorization remains private and fails with `CertificateReceiptUnavailable`. |
| **F-003** | Resolved. Current requirements use stable canonical identities and typed current relationships rather than historical planning, handoff, architecture-section, or gate identifiers. |
| **F-004** | Resolved by complete one-to-one migration of the seven predecessor planning records and the repository-wide terminology cutover. |
| **F-005** | Resolved through colocated `dwv:requires` and `dwv:refines` relationships, deterministic validation, and derived backlinks, ownership views, and reading order. `requires` identifies a semantic prerequisite; `refines` specializes a broader contract without becoming an ownership alias. |
| **F-006** | Resolved. Each write, dirty-intent, fence/checkpoint, abandonment, failure, and composition policy has one complete owner. Refiners and composers retain only local responsibility. |
| **F-007** | Resolved. The completed Linux correction-program requirement was retired while its evidence remains mapped to durable current requirements. |
| **F-008** | Resolved. The supported Linux profile is separated from the exact facts of the final acceptance environment. |
| **F-009** | Resolved. Metadata-loss recovery uses a canonical total case matrix plus current parity, checksum, identity, and evidence relationships. Historical gates do not authorize recovery. |
| **F-010** | Resolved. The incomplete superseded `knowledge-normalization` archive was deleted without a tombstone, redirect, alias, or duplicate. |
| **F-011** | Resolved. Linux runtime names are functional rather than milestone-numbered, and all retained machine evidence and traces came from the fresh final run. |
| **F-030** | Preserved and extended. Milestone isolation remains deterministic and covered by focused regressions. |
| **CR-012, frontend portion** | Resolved. ublk borrows the write payload; no request-boundary `bytes.to_vec()` copy remains. |

## Final semantic ownership

All seven required packets completed under `dwv.knowledge.ownership.v1`. There were no omissions or ownership conflicts.

| Canonical requirement | Role | Owned semantic |
|---|---|---|
| `req.dirty-integrity-invalidation.durable-intent-precedes-protected-mutation` | Owner | Durable dirty-region and stale-integrity intent before protected home mutation. |
| `req.store-operation-contracts.store-write-watermarks-are-real-monotonic-evidence` | Owner | Accepted-write and synchronized-through watermarks, including store incarnation and ordering domain. |
| `req.recovery-state-semantics.clean-and-valid-claims-require-typed-fence-evidence` | Owner | Admissibility of typed evidence used for durable clean, valid-integrity, and clean-session claims. |
| `req.normalized-block-semantics.frontend-lifecycle-events-have-explicit-abandonment-semantics` | Owner | Frontend abandonment and completion semantics; abandonment is not rollback and does not imply backend cancellation. |
| `req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch` | Owner | Stable slot, assignment, role, coding-position, generation, and immutable topology-epoch identity. |
| `req.metadata-loss-recovery.evidence-gates-control-recovery-authorization` | Owner | Conservative evidence-based recovery authorization without forgeable certificate authority. |
| `req.healthy-portable-io.writes-follow-the-reference-transaction-and-update-single-xor-parity` | Composer | Healthy portable write composition. It consumes the detailed owner contracts rather than restating or replacing them. |

Non-problems confirmed:

- High-level constitutional, security, and evidence constraints may constrain several capabilities without duplicating their detailed operational ownership.
- A composer can mention required outcomes while the detailed policy remains with the referenced owner.
- Generated ownership reports, implementations, milestone records, and historical architecture remain evidence or context—not current semantic authority.
- Tooling validates authored relationships but does not invent semantically missing relationships from prose.

## Metadata certificate authority

- `MetadataLossVerification` contains no certificate value.
- Receipt-gated authorization is represented only by private state.
- Callers cannot construct or inject authority.
- Unavailable authority produces `CertificateReceiptUnavailable`.
- Canonical metadata-loss requirements retain the total case matrix, evidence-gate, identity/topology, fresh-state, and bounded-reporting contracts.
- Focused verification: `cargo test -p dwv-recovery metadata_loss` — **7 passed; 41 filtered**.

## Canonical request boundary

- `dwv_core::BlockRequest` is the sole semantic request through validation, service admission, generational slot reservation, execution, slot snapshots, terminal evidence, and admitted terminal failures.
- Target authority is stable `SlotId`, resolved through the captured immutable topology and exact `MemberBinding`.
- Collection order is only an implementation index after semantic resolution.
- The ublk frontend borrows write payloads and does not allocate a second request-boundary payload.
- Verification:
  - `cargo test -p dwv-service` — **35 passed across 3 suites; 1 ignored**
  - `cargo test -p dwv-frontend-ublk` — **12 passed across 2 suites**

## Planning migration and isolation

- Seven predecessor records moved one-to-one to `docs/milestones/m1.md` through `docs/milestones/m7.md`.
- `docs/milestones/m8.md` and `docs/milestones/README.md` remain.
- Historical chronology and accepted technical evidence were preserved.
- The completed milestone-7 acceptance archive was renamed to `2026-08-09-close-milestone-7-acceptance-gaps` without an alias.
- The incomplete superseded documentation archive was deleted.
- `dwv.docs.planning-nomenclature.v1` reports zero retired project-planning identifiers while allowing ordinary English uses of “goal”.
- Focused regressions prove milestones are excluded from current requirement discovery, relationship extraction, reference and ownership context, affected-path analysis, review freshness, and clean-room reconstruction.
- Maintained Markdown links and exact path inventories pass.

### Active-roadmap disposition

`req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative` remains a valid current requirement because it specifies documentation-selection behavior—not product behavior from the roadmap itself.

It owns exact-one roadmap marker discovery, non-authoritative classification, exclusion from ordinary semantic context, deliberate planning-only selection, and explicit reporting when roadmap direction conflicts with current canonical requirements.

The marked v0.8 roadmap remains discoverable for planning, but it cannot satisfy requirement coverage or override `openspec/specs/**/spec.md`.

## Final canonical state

- Capabilities: **25**
- Requirements: **153**
- Relationships: **69**
  - `requires`: **59**
  - `refines`: **10**
- New stable IDs:
  - `req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived`
  - `req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative`
- Retired current ID: the completed Linux correction-program requirement
- All eight readiness gate counts: **zero**

Relevant schemas:

- `dwv.knowledge.objects.v2`
- `dwv.knowledge.reviewed-links.v2`
- `dwv.knowledge.ownership.v1`
- `dwv.knowledge.readiness.v2`
- `dwv.docs.planning-nomenclature.v1`
- `dwv.docs.cli.v1`

## Final validation

| Command | Observed result |
|---|---|
| `openspec validate milestone-planning-cutover --strict` | Passed |
| `openspec validate --strict --all` | **26 passed, 0 failed** |
| `cargo test -p xtask` | **37 passed across 3 suites** |
| `cargo xtask docs knowledge readiness` | Ready; **153 requirements**; all eight gates zero |
| `cargo xtask docs check` | Ready; no diagnostics or review-required entries |
| `cargo xtask docs build` | **153 objects rendered** |
| `cargo xtask docs clean-room` | Equivalent; digest `c39f8fbe29fb144ac6cbedc7fa60d9ce0b84cc59ba5fb4b71e75f2f47c081d7f` |
| `cargo test -p dwv-core` | **15 passed across 2 suites** |
| `cargo test -p dwv-store` | **15 passed across 2 suites** |
| `cargo test -p dwv-service` | **35 passed across 3 suites; 1 ignored** |
| `cargo test -p dwv-recovery metadata_loss` | **7 passed; 41 filtered** |
| `cargo test -p dwv-frontend-ublk` | **12 passed across 2 suites** |
| `cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected` | **1 passed; 16 filtered** |
| `cargo test --test cli_demo` | **3 passed** |
| `cargo test --workspace --all-targets` | **303 passed across 22 suites; 1 ignored** |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --no-deps -- -D warnings` | Passed |

### Model and bounded-proof evidence

- Java: Temurin 21.0.12
- TLC: official 2.19 from `tla2tools.jar` 1.7.4
- PlusCal translation: passed
- TLC: **234 generated states, 125 distinct states, complete depth 9, no errors**
- `tla-checker` 0.6.11: **125 states, 233 transitions, maximum depth 9, status `ok`**
- Kani 0.67.0 dirty-region harness: **0/584 checks failed; 7 unreachable**
- Kani 0.67.0 fence-coverage harness: **0/893 checks failed; 6 unreachable**

These finite and bounded proofs do not certify SQLite, filesystem, controller, FUA, or physical power-loss durability.

## Final Linux acceptance

- Source archive SHA-256: `56751be073f989b77f1d4589ec0d2c810be347facd33e5bab927bd0c1f9a2a4e`
- Host: Apple arm64 using the supported Lima workflow
- Guest: Ubuntu 26.04 arm64
- Kernel: `7.0.0-28-generic`
- Real endpoint: `/dev/ublkb0`
- Queue depth: **8**
- Maximum transfer: **131,072 bytes**
- Maximum trace records: **4,096**
- Exhausted records: **0**

| Artifact | Size / count | SHA-256 |
|---|---:|---|
| `verification/linux-ublk-ext4-acceptance.json` | 9,975 bytes | `60e8a02910415ec859e32626072d0f4c2e2dbc3ef0c824a9f131748d6213b95c` |
| `verification/linux-ublk-trace-first.json` | 445,544 bytes / 453 records | `517726606b6a5ef2bb4e2701f79d0adfaf23b909c4890ee106206a420a565b13` |
| `verification/linux-ublk-trace-second.json` | 79,515 bytes / 81 records | `1fdf9e699dd6a9950f45e92c478fa4b4821e99f2e6bd949ea0b490c2aa649a62` |

The workload covered ext4 format/mount, create, `fsync`, overwrite, rename, directory synchronization, read, delete, unmount, clean shutdown, restart, and read-only remount. Durable content SHA-256 was `d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7`.

Data and parity payloads both hashed to `2129f2a2577a3a919b721c78a435c24191ef4bf454bffcf5e5f8794f98eb8f3a`. Byte equality passed. The data member mounted independently read-only and exposed the same content. Both retained traces replayed through the root CLI with record counts **453** and **81**.

Negative paths failed closed for bounded-resource admission, unsupported topology, ownership conflict, conflicting cleanup, stale readiness, missing recovery authority, unsupported discard, unknown endpoint cleanup, owner-process death, and partial multi-store fence coverage.

## Jujutsu evidence

### Final log summary

```text
qxsmtwqv 0a29846d822b
nkntwxmq 132253e3e7bb chore: milestone 8 normalized-service-request-boundary review checkpoint
lzyrykpp 8820fa1293df chore: milestone 8 pre-request-boundary checkpoint
ymxkqywk 99cdfbeb254c chore: milestone 8 metadata-certificate-authority-gate implementation
qxsmmvvl a54d8df7349c chore: milestone 8 canonical-semantic-ownership implementation
xmvwrzss acb83424bb7f chore: milestone 8 implementation checkpoint 1
```

### Final diff summary

```text
M docs/README.md
R docs/{handoffs/goal.md => milestones/m1.md}
R docs/{handoffs/goal-v2.md => milestones/m2.md}
R docs/{handoffs/goal-v3.md => milestones/m3.md}
R docs/{handoffs/goal-v4.md => milestones/m4.md}
R docs/{handoffs/goal-v5.md => milestones/m5.md}
R docs/{handoffs/goal-v6.md => milestones/m6.md}
R docs/{handoffs/goal-v7.md => milestones/m7.md}
M docs/milestones/m8.md
M docs/reviewed-requirements.toml
M docs/verification/linux-ublk-ext4-acceptance.md
M docs/verification/m8.md
D openspec/changes/archive/2026-08-09-close-goal-v7-acceptance-gaps/proposal.md
R openspec/changes/archive/{2026-08-09-close-goal-v7-acceptance-gaps => 2026-08-09-close-milestone-7-acceptance-gaps}/.openspec.yaml
R openspec/changes/archive/{2026-08-09-close-goal-v7-acceptance-gaps => 2026-08-09-close-milestone-7-acceptance-gaps}/design.md
A openspec/changes/archive/2026-08-09-close-milestone-7-acceptance-gaps/proposal.md
R openspec/changes/archive/{2026-08-09-close-goal-v7-acceptance-gaps => 2026-08-09-close-milestone-7-acceptance-gaps}/specs/linux-ublk-frontend/spec.md
R openspec/changes/archive/{2026-08-09-close-goal-v7-acceptance-gaps => 2026-08-09-close-milestone-7-acceptance-gaps}/tasks.md
D openspec/changes/archive/2026-08-09-knowledge-normalization/design.md
D openspec/changes/archive/2026-08-09-knowledge-normalization/proposal.md
D openspec/changes/archive/2026-08-09-knowledge-normalization/specs/architecture-contract/spec.md
D openspec/changes/archive/2026-08-09-knowledge-normalization/specs/documentation-knowledge-architecture/spec.md
D openspec/changes/archive/2026-08-09-knowledge-normalization/tasks.md
M openspec/changes/archive/2026-08-09-linux-ublk-ext4-acceptance/proposal.md
M openspec/changes/archive/2026-08-09-linux-ublk-ext4-acceptance/tasks.md
M openspec/changes/archive/2026-08-09-normalize-coding-profile-transitions/proposal.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/.openspec.yaml
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/design.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/proposal.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/specs/anchorless-topology-identity/spec.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/specs/healthy-portable-io/spec.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/specs/linux-ublk-frontend/spec.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/specs/normalized-block-semantics/spec.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/specs/store-operation-contracts/spec.md
R openspec/changes/{normalized-service-request-boundary => archive/2026-08-09-normalized-service-request-boundary}/tasks.md
R openspec/changes/{archive/2026-08-09-knowledge-normalization => milestone-planning-cutover}/.openspec.yaml
A openspec/changes/milestone-planning-cutover/design.md
A openspec/changes/milestone-planning-cutover/proposal.md
A openspec/changes/milestone-planning-cutover/specs/documentation-knowledge-architecture/spec.md
A openspec/changes/milestone-planning-cutover/tasks.md
M openspec/specs/anchorless-topology-identity/spec.md
M openspec/specs/documentation-knowledge-architecture/spec.md
M openspec/specs/healthy-portable-io/spec.md
M openspec/specs/linux-ublk-frontend/spec.md
M openspec/specs/normalized-block-semantics/spec.md
M openspec/specs/store-operation-contracts/spec.md
M tools/linux-disk-acceptance/guest.sh
M tools/linux-disk-acceptance/run.sh
M verification/linux-ublk-ext4-acceptance.json
M verification/linux-ublk-trace-first.json
M verification/linux-ublk-trace-second.json
M verification/manifest.toml
M xtask/src/knowledge.rs
M xtask/src/lib.rs
```

### Final status

`jj status` reported the same working-copy path set above.

```text
Working copy  (@) : qxsmtwqv 0a29846d (no description set)
Parent commit (@-): nkntwxmq 132253e3 chore: milestone 8 normalized-service-request-boundary review checkpoint
```

## Deferred post-M8 work

These are explicit future product or maintenance decisions, not unresolved milestone defects:

- production recovery-adapter selection and the complete lost-acknowledgement/process-restart design;
- broader local-file identity and topology-authority cleanup;
- transaction-engine candidate retirement;
- macOS bridge-feasibility retirement;
- trace-schema compatibility removal;
- codec redesign or large-module decomposition;
- broader Linux topology, concurrency, and publication support;
- raw-device deployment;
- production FUA and physical power-loss certification.

## Approval boundary

After explicit approval, the remaining actions are:

1. Archive `milestone-planning-cutover`.
2. Confirm no active OpenSpec change remains.
3. Run strict post-archive OpenSpec validation.
4. Run post-archive documentation readiness.
5. Run the post-archive documentation consistency check.

Milestone 8 implementation remains pending: stop here and await explicit approval before archiving the final change.
