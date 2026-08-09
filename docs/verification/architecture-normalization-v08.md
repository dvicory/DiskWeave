# Architecture normalization audit: v0.8 and OS-000 baseline

**Status:** migration/disposition evidence for the active normalization change; not a current architecture source.

**Inputs**

- v0.8 handoff: `docs/handoffs/diskweave-refined-architecture-v0.8.md`
  - 3,925 lines
  - SHA-256: `e0e80c0f926de3c6ca1aabc18165ab23c34c5033d36e7c3021159b7acfef188f`
- pre-normalization OS-000-style canonical architecture contract, recovered from the repository parent revision before this amendment
  - SHA-256: `694fe913f94c674248aa960917f0dfd9e88bd07b0bda9fb3ad03c6e8f14b25c9`

The old contract and v0.8 are migration inputs. They are not loaded as current semantic authority. This record preserves their identities, records where their normative material went, and identifies the executable owner or evidence boundary. Historical text remains available in repository/archive history.

## Disposition vocabulary

- **preserved** — the timeless product invariant remains in the normalized architecture constitution.
- **relocated** — capability-local semantics now live in a dedicated canonical OpenSpec, ADR, or verification record.
- **superseded** — the old implementation/workflow prescription is intentionally replaced by a less coupled current boundary.
- **historical** — sequencing, rationale, alternatives, proposed technology, or old agent instructions remain useful archaeology only.
- **gap** — no current owner exists; this status is forbidden for the completed audit.

## Existing architecture-contract baseline

| Baseline requirement | Disposition | Durable owner | Evidence / boundary |
|---|---|---|---|
| Architecture artifacts preserve the handoff contract | historical | repository history and OpenSpec/work-program records | The twenty-section artifact shape and decision taxonomy are process history, not product semantics. |
| OpenSpec dependencies and readiness are explicit | historical | OpenSpec change/task records and repository workflow | OS-001/OS-002/OS-003 sequencing is not an architecture invariant. |
| Portable boundaries and safety invariants are normative | preserved + relocated | normalized `architecture-contract`; `anchorless-topology-identity`; `normalized-block-semantics`; `store-operation-contracts`; `recovery-state-semantics`; `evidence-boundaries` | Product boundary, ordinary payloads, replaceable adapters, fail-closed uncertainty, and no unproven format promise remain global; geometry and protocol details remain capability-local. |
| Completion is evidence-backed and scope-accurate | relocated | `openspec/specs/evidence-boundaries/spec.md`, capability specs, and `docs/verification/**` | Evidence tier, unsupported claim boundaries, deterministic artifacts, and gated platform claims are verification semantics, not workflow machinery. |
| Agent workflow uses only OpenSpec artifacts and CLI | historical + relocated | `openspec/specs/documentation-knowledge-architecture/spec.md` and `docs/README.md` | Agent/context and OpenSpec process rules are documentation tooling; no product architecture clause remains. |

## v0.8 section dispositions

The section ID is the stable historical locator. Subsections and tables inherit the disposition of their containing section unless a capability owner is named explicitly below.

| Section | Disposition | Durable current owner | What is retained or moved |
|---|---|---|---|
| 1. Purpose and scope | preserved + historical | normalized `architecture-contract` | Block-level parity beneath conventional filesystems and the explicit product boundary remain; the claim that one handoff is a complete implementation source is historical. |
| 2. Decision authority and product invariants | preserved + relocated | normalized `architecture-contract`; capability specs; ADRs | Independent payloads, no file striping/namespace ownership, replaceable seams, and conservative uncertainty remain at the appropriate layer; ACCEPTED/PROVISIONAL/TUNABLE registers and user-decision lists are workflow/history. |
| 3. Architectural layering and dependency rules | preserved + relocated | normalized `architecture-contract`; `normalized-block-semantics`; `store-operation-contracts`; `recovery-state-semantics`; `explicit-transaction-machine` | Portable seams and dependency direction remain; exact codec, executor, simulator, placement, and lifecycle semantics are capability contracts. |
| 4. Fundamental architecture and product boundary | preserved + historical | normalized `architecture-contract`; `healthy-portable-io`; `macos-bridge-feasibility` | Frontend-neutral core, ordinary payloads, coherent group exposure, and direct-read boundaries remain; Linux stack diagrams and candidate deployment are provisional/platform evidence. |
| 5. Component, process, and dependency boundaries | preserved + relocated | normalized `architecture-contract`; `store-operation-contracts`; `normalized-block-semantics`; `security-boundaries` | Explicit adapter boundaries, unsafe/parsing seams, and authority separation remain; crate names, process layout, privilege sequence, and technology placement are implementation records. |
| 6. Physical stores, logical topology, and identity | relocated | `anchorless-topology-identity`; `file-backed-stores`; `recovery-state-semantics` | Orthogonal identity, immutable topology epochs, observations, aliasing, and staged transitions are now canonical capability requirements. |
| 7. Geometry, coding, and granularity | relocated | `normalized-block-semantics`; `xor-reference-model`; `parity-envelope-profiles`; `healthy-portable-io` | Checked ranges, logical geometry, XOR behavior, codec independence, envelope profile limits, and topology transitions are capability-local. |
| 8. Persistent state, parity format, and schema governance | relocated + superseded | `file-backed-stores`; `recovery-state-semantics`; `parity-envelope-profiles`; `checksum-plane`; `metadata-loss-recovery`; `normalized-trace-replay` | Ordinary data bytes and independent semantic exports remain; SQLite choices, envelope bytes, migration mechanics, exact reserve policy, and format candidates are adapter/experimental contracts, not constitution text. |
| 9. Portable semantic contracts | relocated | `normalized-block-semantics`; `store-operation-contracts`; `explicit-transaction-machine`; `transaction-engine-evidence-comparison`; `volatile-media-simulator` | Frontend-neutral request, exact range, lifecycle, operation ownership, deterministic authority, simulator, and trace semantics are canonical capability specs. |
| 10. Crash-consistency protocol | relocated | `dirty-integrity-invalidation`; `recovery-state-semantics`; `explicit-transaction-machine`; `healthy-portable-io`; `parity-envelope-profiles` | Durable intent before protected mutation, fence-gated clean state, uncertainty, cancellation, shutdown, and recovery transitions are protocol requirements. |
| 11. Concurrency, ownership, and resource bounds | relocated + preserved | `store-operation-contracts`; `explicit-transaction-machine`; `normalized-block-semantics`; `security-boundaries` | Bounded ownership, non-rollback abandonment, range coordination, checkpoint races, and resource limits are capability/security contracts; the provisional `procmachines` choice is historical/replaceable. |
| 12. Failure and degraded-operation semantics | relocated | `degraded-read-offline-rebuild`; `metadata-loss-recovery`; `parity-verification-repair`; `checksum-scrub-verified-repair`; `anchorless-topology-identity` | Known erasure, read-only degraded behavior, separate-target rebuild, metadata-loss matrix, repair authorization, and ambiguous topology are dedicated requirements. |
| 13. Integrity plane | relocated | `checksum-plane`; `checksum-scrub-verified-repair`; `parity-verification-repair`; `dirty-integrity-invalidation` | Independent checksum validity, pre-mutation invalidation, scrub classification, unique repair, and migration semantics are canonical. |
| 14. Namespace and placement semantics | preserved + historical | normalized `architecture-contract`; future namespace capability only if introduced | The product exclusion of namespace/allocation/file striping remains; mergerfs, movers, tiers, and placement algorithms are optional or future product material and are not current parity authority. |
| 15. Encryption, tiers, and deployment | preserved + historical | normalized `architecture-contract` for layer separation; future deployment/namespace ADRs | Encryption remains outside parity semantics; Linux boot, mount, tier, staging, and hardware certification recipes are platform/deployment evidence, not timeless core architecture. |
| 16. Technology decisions | historical + relocated | existing capability specs and ADRs (`docs/adr/**`) | Rust/tool/library statuses remain replaceable decision history. Verification tools remain evidence mechanisms under `evidence-boundaries`, never runtime dependencies. |
| 17. macOS portable reference implementation | relocated + historical | `macos-bridge-feasibility`; `macos-demo-cli`; `file-backed-stores`; `docs/verification/os-020-macos-bridge-feasibility.md` | Regular/sparse-file portable evidence and bridge feasibility boundaries remain; candidate installation, entitlements, DiskImages, APFS synchronization, and power-loss claims remain gated platform evidence. |
| 18. Alternatives and product-family boundaries | preserved + historical | normalized `architecture-contract`; ADR/history | The rejection of a filesystem-aware parity product and the boundary against namespace/allocator ownership remain; comparative rationale is historical. |
| 19. Security and operational hardening | relocated + preserved | `security-boundaries`; `anchorless-topology-identity`; `macos-demo-cli`; `store-operation-contracts`; `evidence-boundaries` | Role binding, read-only versus mutation authority, hostile-input bounds, privacy defaults, dependency seams, and resource exhaustion are dedicated cross-cutting safety contracts. |
| 20. Observability and operator model | relocated | `store-operation-contracts`; `recovery-state-semantics`; `macos-demo-cli`; verification evidence | Multidimensional state and evidence observability remain in capability outputs; command names, daemon lifecycle, and `dwv demo` acceptance are CLI/product evidence, not architecture constitution. |
| 21. Verification and deterministic evidence strategy | relocated | `evidence-boundaries`; `normalized-trace-replay`; `volatile-media-simulator`; `transaction-engine-evidence-comparison`; `docs/verification/**` | Evidence tiers, bounded replay, model/simulator roles, and non-claims are canonical verification semantics; the tool inventory and work plan are historical. |
| 22. OpenSpec decomposition and implementation ordering | historical | OpenSpec change history and work program | Phase 0–5 sequencing and OpenSpec IDs are not product architecture. |
| 23. Release and acceptance gates | relocated + historical | capability verification records and ADRs under `docs/verification/**` | Gate outcomes remain evidence records tied to capabilities; the v0.8 gate register is historical and cannot satisfy a current requirement by itself. |
| 24. Decision, validation, tuning, and format register | historical + relocated | ADRs, canonical capability specs, and format-specific evidence | Decision classifications and tuning lists are process history; persistent format candidates remain experimental capability records until independently accepted. |
| 25. Autonomous implementation-agent and OpenSpec operating contract | historical + relocated | `documentation-knowledge-architecture`; repository process files | Agent authority, task shape, handoff, and OpenSpec section rules are tooling/process semantics and are excluded from product architecture. |
| 26. Disaster-recovery stories | relocated | `metadata-loss-recovery`; `anchorless-topology-identity`; `degraded-read-offline-rebuild`; `checksum-plane`; `parity-verification-repair`; `recovery-state-semantics` | Recovery cases remain executable/operator-facing capability scenarios; the prose story collection is not a second architecture authority. |
| 27. Final recommendation and architecture self-review | historical | repository history and review evidence | Recommendation, coupling checklist, and completeness test are review history; capability contracts and tests retain the actual invariants. |

## Audit result

No `gap` disposition remains in the baseline mapping. The normalized architecture constitution retains only product-wide boundaries: the block-parity product boundary, independently readable ordinary payloads, explicit replaceable seams, and fail-closed interpretation of identity/durability/recovery/integrity/format evidence. Topology, geometry, transaction, checksum, repair, simulator, frontend, security, verification, and documentation details are owned by the named capability specs or evidence records.

This report is evidence for the normalization change and may be regenerated or superseded by a later explicit migration. It is not a source for runtime behavior, current requirements, or agent context by itself.
