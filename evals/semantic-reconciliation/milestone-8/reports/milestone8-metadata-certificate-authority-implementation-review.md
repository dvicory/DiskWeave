# Milestone 8 Metadata Certificate-Authority Implementation Review

Date: 2026-08-10
Change: `metadata-certificate-authority-gate`
Review boundary: Milestone 8 WP-8.2 only. This change is implemented and validated but intentionally remains active and unarchived pending this review.

## 1. Requested verdict

Review whether this implementation closes the certificate-authority defect without inventing a receipt capability, weakening another metadata-loss refusal, or reintroducing historical semantic authority.

Approval means all of the following are true:

1. no ordinary caller can manufacture certificate authority;
2. every certificate-receipt-gated case remains inspectable but returns `MetadataLossError::CertificateReceiptUnavailable(case)` through both authorization entry points;
3. no receipt type, validator, producer, token, boolean capability, string token, wrapper, or public verification-enum substitute was introduced;
4. operator confirmation cannot bypass the unavailable receipt;
5. all non-certificate authorization/refusal/fresh-state/payload-write policies remain intact;
6. current metadata-loss requirements are self-contained, retain their five stable IDs, and declare their exact canonical dependencies;
7. reviewed fingerprints are individually resolved with concrete reasons and no review gate remains stale;
8. evidence claims and mappings describe the current implementation rather than the old forgeable path;
9. all exact validation evidence below is sufficient and reproducible.

If any condition is false, reject with exact file/line or requirement evidence.

## 2. Scope and predecessor state

The working copy also contains the already approved `canonical-semantic-ownership` prerequisite and its archival/milestone-record changes. That predecessor is outside this verdict except where the combined diff is needed to understand the stacked, uncommitted working copy.

Current review target:

- `openspec/changes/metadata-certificate-authority-gate/**`
- `openspec/specs/metadata-loss-recovery/spec.md`
- `crates/dwv-recovery/src/metadata_loss.rs`
- `crates/dwv-recovery-sqlite/src/lib.rs` (one version assertion only)
- `docs/reviewed-requirements.toml` (five metadata requirements only)
- `docs/verification/os-015-metadata-loss-recovery.md`
- `verification/manifest.toml` (`evidence.os-015-metadata-loss-recovery` only)

No service API, request identity, topology resolution, operation admission, buffer/resource identity, Linux translation, payload implementation, SQLite schema, or production durability claim changed.

The only active OpenSpec change is `metadata-certificate-authority-gate`: 12/12 tasks complete. It remains deliberately active pending external approval. Do not archive during review.

## 3. Problem and source fix

### Before

`MetadataLossVerification` was a public serializable enum containing:

```rust
CertifiedCleanEnvelope
```

`MetadataLossPlan::authorize` and `authorize_with_operator_confirmation` accepted that caller-constructible value as satisfying `EvidenceRequirement::CertifiedCleanGate`. A caller could therefore satisfy a certificate gate without a validator-issued, non-forgeable receipt.

### After

- `MetadataLossVerification::CertifiedCleanEnvelope` is deleted with no compatibility alias or replacement.
- `EvidenceRequirement::CertifiedCleanGate` is renamed to the descriptive `ValidatedCertificateReceipt`; its stable dry-run ID is `validated-certificate-receipt`.
- `MetadataLossPlan::requires_session_certificate_gate()` is renamed to `requires_certificate_receipt()`.
- `MetadataLossError::CertificateReceiptUnavailable(MetadataLossCase)` is added.
- `MetadataLossPlan::authorize_inner` checks `required_evidence == ValidatedCertificateReceipt` before action refusal, evidence matching, or operator confirmation and returns the stable unavailable error.
- `MetadataLossVerification::evidence_satisfies(ValidatedCertificateReceipt)` is always false.
- The public verification enum now has nine variants; none describes or stands in for a certificate receipt.
- Matrix version increments from 1 to 2. Dry-run rows expose `certificate_receipt_required=<bool>` and `certificate_receipt_available=false` separately.
- No receipt API was synthesized. A future receipt capability requires its own canonical change defining producer/validator authority and exact array/topology/profile/session/operation scope.

This is an intentional early-development breaking API/serialized-format cutover. Retaining a compatibility shim would retain the authority defect.

## 4. Exact certificate-gated case table

All seven rows remain in `MetadataLossCase::ALL`. In every row below, `requires_certificate_receipt()` is true, the descriptive evidence requirement is `ValidatedCertificateReceipt`, no receipt capability exists, and every one of the nine public `MetadataLossVerification` values returns `CertificateReceiptUnavailable(case)` through ordinary and operator-confirmed authorization. The failure occurs before any authorization token or fresh semantic state can be created.

| Stable case ID | Planned action | Payload-write policy | Why non-authorizing now |
|---|---|---|---|
| `all-data-p-certified-clean` | `RecreateFromCertifiedEnvelope` | `None` | validated certificate receipt required; capability absent |
| `all-data-pq-certified-clean` | `RecreateFromCertifiedEnvelope` | `None` | validated certificate receipt required; capability absent |
| `one-data-missing-p-certified-clean` | `ReadOnlyDecodeToReplacement` | `SeparateReplacementTargetOnly` | validated certificate receipt required; capability absent |
| `one-data-missing-p-uncertified` | `RefuseAutomaticDecode` | `NoAutomaticWrite` | validated certificate receipt required; capability absent; operator confirmation is checked later and cannot bypass it |
| `one-data-missing-pq-certified-clean` | `ReadOnlyDecodeToReplacement` | `SeparateReplacementTargetOnly` | validated certificate receipt required; capability absent |
| `one-data-missing-pq-uncertified` | `RefuseAutomaticDecode` | `NoAutomaticWrite` | validated certificate receipt required; capability absent; operator confirmation is checked later and cannot bypass it |
| `two-data-missing-pq-certified-clean` | `ReadOnlyDecodeToReplacement` | `SeparateReplacementTargetOnly` | validated certificate receipt required; capability absent |

The focused regression executes 7 cases × 9 public verification values × 2 authorization entry points = 126 unavailable-receipt assertions. For each gated case it also proves `fresh_manifest` cannot be reached from those calls.

## 5. Exact outcome classes for all 18 matrix cases

“Authorizable now” means the plan can produce a private `MetadataLossAuthorization` only when the listed current evidence and confirmation conditions are supplied. It does not imply that a payload action, P/Q engine, live frontend, or production backend exists.

### Authorizable now (8)

| Stable case ID | Required current evidence | Operator confirmation | Fresh state |
|---|---|---:|---:|
| `all-data-p-uncertified` | `ExhaustiveMatches` or `ExhaustiveIdentifiedRepairs`; `ExplicitDataAuthoritativeRebaseline` is also accepted with confirmation | only for explicit rebaseline | yes |
| `all-data-pq-uncertified` | same as above | only for explicit rebaseline | no |
| `all-data-parity-missing` | `ExplicitDataAuthoritativeRebaseline` | yes | no |
| `all-metadata-lost-all-data-present` | `ExplicitDataAuthoritativeRebaseline` | yes | no |
| `recovery-backup-valid` | `ValidatedBackup` | no | no |
| `recovery-replicas-disagree` | `ReconciledReplicas` | yes | no |
| `checksum-evidence-survives` | `ExhaustiveIdentifiedRepairs` | no | no |
| `checksum-evidence-unavailable` | `ExplicitDataAuthoritativeRebaseline` | yes | no |

### Non-authorizing because certificate receipt is unavailable (7)

Exactly the seven rows in Section 4. Their stable terminal error is `MetadataLossError::CertificateReceiptUnavailable(case)`, including the two `RefuseAutomaticDecode` plans because receipt unavailability is the first gate.

### Non-authorizing for another existing reason (3)

| Stable case ID | Existing terminal reason |
|---|---|
| `two-data-missing-pq-coding-lost` | `RefusedCase(case)`; required coding positions are lost |
| `topology-identifiers-ambiguous` | `RefusedCase(case)`; identity/topology is ambiguous |
| `parity-candidate-ambiguous` | `RefusedCase(case)`; parity identity is ambiguous |

The pre-existing focused tests still prove uniquely verified repair, backup/replica, operator rebaseline, ambiguous identity, incomplete/conflicting evidence, stale generation, and non-creating fresh-state behavior.

## 6. Stable requirement-ID disposition

No metadata-loss requirement ID was added, removed, renamed, or reused. All five current requirements were rewritten to be self-contained and received exact dependency markers.

| Requirement ID | Disposition | Final local fingerprint | Final effective fingerprint |
|---|---|---|---|
| `req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative` | preserved; current matrix/receipt/envelope ownership made explicit | `1bb03a1f665dcd37deaea4161ba4a7785e26da473abd415e085a5265b6d406ef` | `058cff476ecd4dd9d66044deaa928df8bfe4552feefc9c83eb87962383592281` |
| `req.metadata-loss-recovery.evidence-gates-control-recovery-authorization` | preserved; current authorization inputs and unavailable-receipt outcome made explicit | `a6e48d9c37be80490812dadb2e71c5658c0703143e0883eee3ee46b439a3d8d7` | `2be0c494963c4f9e701e297ba1f95077375b15bd0bc71194873d53ec0f254479` |
| `req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed` | preserved; current topology owner and writable fail-closed refinement made explicit | `ac362314e25684d086d96d3a77dd3afd7f97008bdc153adb208a9eb0c0564b7b` | `16e9886eff8c309749574def35cc40ee6c5eed279fad1a93ce09a79c4467e8fa` |
| `req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit` | preserved; current topology/recovery-state composition made explicit | `15a34349e1cbba462eeadcc24fd3360ad30815fb82d7a2d0a9242f582baa12b2` | `8b3601fa5323abbd62708dcdad67e0af14cde105658b8bbecc7a6dc4d21a397f` |
| `req.metadata-loss-recovery.dry-run-reporting-is-bounded-and-portable` | preserved; receipt requirement/availability and evidence-boundary composition made explicit | `759d361712b9c366efd1a4de44207c7990e87f160e43dc8f07fd77c692f90d5a` | `56e1b29f90a1751ce92a92d9dcc02c78bfe6b7a8c58efd07925fb6429e488c62` |

Historical OpenSpec changes and unrelated historical verification records were not rewritten. Current semantic surfaces no longer tell readers to consult OS-014, Gate H, handoff sections, or architecture sections for the current matrix, authorization, topology, fresh-state, or dry-run contract. The Rust module comment, fresh-topology diagnostic, and affected test name were also changed to descriptive current behavior.

## 7. Exact relationship changes

This phase adds 17 `requires` edges and 2 `refines` edges to the five metadata-loss owners.

| Metadata owner (suffix) | Kind | Exact target requirement ID |
|---|---|---|
| matrix | `requires` | `req.metadata-loss-recovery.evidence-gates-control-recovery-authorization` |
| matrix | `requires` | `req.parity-envelope-profiles.envelope-copies-are-independently-inspectable` |
| matrix | `requires` | `req.parity-envelope-profiles.envelope-disagreement-resolves-conservatively` |
| matrix | `requires` | `req.parity-envelope-profiles.session-transitions-have-ordered-recovery-semantics` |
| authorization | `requires` | `req.parity-verification-repair.exhaustive-verification-is-a-read-only-full-scan` |
| authorization | `requires` | `req.parity-verification-repair.mismatch-classification-requires-independent-evidence` |
| authorization | `requires` | `req.checksum-scrub-verified-repair.automatic-repair-requires-one-unique-verified-solution` |
| authorization | `requires` | `req.checksum-scrub-verified-repair.repairs-use-a-separate-target-and-verified-readback` |
| authorization | `requires` | `req.evidence-boundaries.unknown-and-ambiguous-evidence-fail-closed` |
| identity/topology | `requires` | `req.anchorless-topology-identity.identity-evidence-is-assessed-from-multiple-observations` |
| identity/topology | `requires` | `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments` |
| identity/topology | `refines` | `req.anchorless-topology-identity.writable-assembly-fails-closed-on-unresolved-identity` |
| fresh state | `requires` | `req.metadata-loss-recovery.evidence-gates-control-recovery-authorization` |
| fresh state | `requires` | `req.anchorless-topology-identity.topology-identities-are-explicit-and-immutable-within-an-epoch` |
| fresh state | `requires` | `req.anchorless-topology-identity.topology-validation-rejects-ambiguous-or-inconsistent-assignments` |
| fresh state | `refines` | `req.recovery-state-semantics.semantic-export-and-health-are-independent-of-storage-engine-layout` |
| dry run | `requires` | `req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative` |
| dry run | `requires` | `req.evidence-boundaries.evidence-scope-is-explicit` |
| dry run | `requires` | `req.evidence-boundaries.verification-artifacts-are-deterministic-and-bounded` |

Graph delta from the verified post-ownership-archive Milestone 8 baseline: requirements 152 → 152; relationships 49 → 68; `requires` 41 → 58; `refines` 8 → 10. Final metadata cluster: 5 requirements, 19 outgoing relationships.

The final `cargo xtask docs check` invocation reports `added_relationships: 1` against its Jujutsu `@-` baseline because this is a stacked, uncommitted working copy containing the predecessor ownership cutover, which removed earlier duplicate/historical relationships before this phase re-added the exact current ones. That combined-parent number is not the WP-8.2 phase delta. The phase boundary is the recorded and re-verified post-archive 49-edge state in `docs/verification/m8.md`; the final exported current graph is 68.

## 8. Fingerprint and review closure

Only the five metadata-loss owners received new local/effective fingerprints in this phase. Each was resolved individually, not bulk accepted:

- matrix: reviewed against current envelope and authorization owners; local case enumeration remains local while receipt and envelope facts are prerequisites;
- authorization: reviewed against parity verification, checksum repair, and evidence boundaries; it composes their outcomes and adds only metadata-loss admission;
- identity/topology: reviewed against anchorless topology; it consumes identity assessment and only refines writable fail-closed behavior;
- fresh state: reviewed against topology and recovery-state owners; it binds validated topology and refines semantic export without owning storage layout;
- dry run: reviewed against the matrix and evidence-boundary owners; it remains bounded, deterministic, diagnostic, and non-authorizing.

Final readiness/check gate counts are all zero:

- `historical-reference`: 0
- `local-fingerprint-suspect`: 0
- `mapping-collision`: 0
- `orphaned-state`: 0
- `semantic-prerequisite-changed`: 0
- `uncovered`: 0
- `unknown-reference`: 0

Final state: 152 requirements; all 152 reviewed; no retired Linux correction-program ID; no unreviewed metadata owner.

## 9. Correction-evidence remap

The existing `evidence.os-015-metadata-loss-recovery` record remains the evidence owner. It was refreshed rather than duplicated.

- Path remains `docs/verification/os-015-metadata-loss-recovery.md`.
- Requirement mapping now includes all five metadata-loss IDs; the previously omitted dry-run requirement was added.
- Claim now states that the implementation enumerates the conservative matrix, rejects every certificate-receipt-gated case while no receipt capability exists, and creates audited fresh state only from authorized current evidence.
- Non-claims explicitly exclude a certificate-receipt capability, P/Q execution, degraded reads, live frontend behavior, and hardware durability.
- The evidence document directly maps Sections 2–6 to the five current requirement IDs and records matrix v2, the stable unavailable error, the no-mutation proof, and exact acceptance results.

No correction evidence maps to OS-014, Gate H, a handoff section, an architecture section, or another historical authority source.

## 10. OpenSpec and design conformance

Artifacts:

- proposal: complete
- delta spec: complete
- design: complete
- tasks: 12/12 complete; change remains active for external review

Observed implementation matches the design:

- current metadata owner split retained;
- forgeable verification value deleted rather than wrapped;
- no receipt API invented;
- gated plans remain visible;
- receipt refusal precedes evidence and confirmation;
- other policies retained;
- dry-run evidence versioned;
- historical authority removed only from current surfaces;
- exact relationships and individual review closure applied.

Reported bounded design clarifications made during implementation:

1. The actual receipt-gated set contains seven stable cases, not five; the design was corrected to seven.
2. The dry-run evidence ID is `validated-certificate-receipt`; unavailability is a separate `certificate_receipt_available=false` field. The design was corrected to match the implementation/spec distinction.
3. The plan predicate checks the exact evidence requirement enum rather than a renamed boolean accessor. The design was corrected to match.

No unresolved semantic divergence remains.

## 11. Exact validation commands and observed results

All commands ran from `/Users/daniel.vicory/src/DiskWeave` on the final implementation unless noted.

| Command | Observed result |
|---|---|
| `openspec validate metadata-certificate-authority-gate --strict` | passed |
| `openspec validate --strict --all` | 26 passed, 0 failed |
| `cargo test -p dwv-recovery metadata_loss` | 7 passed, 41 filtered out |
| `cargo test -p dwv-service metadata_loss` | 3 passed, 26 filtered out |
| `cargo test -p dwv-recovery-sqlite metadata_loss` | 1 passed, 9 filtered out |
| `cargo test -p xtask` | 32 passed across 3 suites |
| `cargo xtask docs knowledge readiness` | ready; 152 requirements; every gate count 0 |
| `cargo xtask docs check` | success; ready; 152 requirements; every gate count 0 |
| `cargo xtask docs build` | success; 152 objects |
| `cargo xtask docs clean-room` | equivalent=true; object digest `1272ec5c70816efa0e9429a80367e65e90ead765960039456618e8a0c12b4c66` |
| `cargo test --workspace` | 289 passed across 34 suites; 1 ignored |
| `cargo fmt --all -- --check` | passed with no output |
| `cargo clippy --workspace --all-targets --no-deps -- -D warnings` | passed |
| `cargo run -q -p dwv-recovery --example metadata-loss-dry-run` | passed; emitted all 18 matrix-v2 rows with stable case IDs and `certificate_receipt_available=false` |
| five `cargo xtask docs knowledge ownership <metadata ID>` commands | success; each showed exact typed owner relationships and reviewed state |
| exported graph count over `target/dwv-docs/knowledge/objects.json` | 25 capabilities; 152 requirements; 68 relationships = 58 `requires` + 10 `refines`; metadata cluster 5 requirements/19 relationships |
| current-surface historical identifier scan over metadata spec/recovery/service/SQLite files | no `OS-014`, `OS-015`, `Gate H`, `Section 12.5`, `handoff`, `CertifiedCleanEnvelope`, `certified-clean-session-gate`, or `requires_session_certificate_gate` match |

The SQLite focused test initially failed because its persisted semantic summary correctly changed from matrix version 1 to 2; its single expected-version assertion was updated and the test then passed. No SQLite production code or schema changed.

## 12. Current diff and artifact pointers

Complete current combined working-copy diff, captured after implementation, documentation, task, and design corrections:

- `artifact://498`
- filesystem artifact: `/Users/daniel.vicory/.omp/agent/sessions/-src-DiskWeave/2026-08-09T09-29-46-736Z_019fe5db-5df0-7000-8e1e-12f7b37a48f3/498.bash-original.log`

The diff is intentionally combined because the user required review of the full current working tree; use Section 2 to distinguish the approved predecessor from this verdict’s target.

Primary review anchors:

- active change: `openspec/changes/metadata-certificate-authority-gate/`
- current canonical spec: `openspec/specs/metadata-loss-recovery/spec.md`
- authorization implementation/tests: `crates/dwv-recovery/src/metadata_loss.rs`
- reviewed state: `docs/reviewed-requirements.toml`
- evidence: `docs/verification/os-015-metadata-loss-recovery.md`
- evidence mapping: `verification/manifest.toml`
- predecessor milestone boundary/count record: `docs/verification/m8.md`

## 13. Archive and successor boundary

Do not archive this change unless this review approves it. Do not start `normalized-service-request-boundary` as part of this review. After approval, the main agent will archive through the normal OpenSpec workflow, re-run post-archive strict/current-state checks, update the Milestone 8 verification record, and only then determine whether another final Milestone 8 review is still required.

End the external verdict with exactly one of these sentences:

- `APPROVED FOR ARCHIVE`
- `REJECTED — CORRECTIONS REQUIRED`
