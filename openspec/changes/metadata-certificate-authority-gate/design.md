## Context

See `proposal.md` for motivation. The five current metadata-loss requirements own the case matrix, authorization, identity/topology refusal, fresh-state creation, and dry-run output. Their current ownership views contain no semantic edges. Related current requirements already own identity assessment and topology validation, exhaustive verification and mismatch classification, checksum-authorized unique repair and separate-target verification, envelope inspection/session interpretation, missing-state recovery authority, and product-wide evidence bounds.

Canonical review found no current requirement contradiction. It found one implementation nonconformance: `MetadataLossVerification::CertifiedCleanEnvelope` is public and constructible, while the current matrix requirement says no caller-selected classification may satisfy the unavailable certificate gate.

## Goals / Non-Goals

**Goals:**

- Preserve the five stable metadata-loss requirement IDs and all 18 stable case IDs.
- Make the absent certificate-receipt authority impossible to forge through current public values.
- Keep operator confirmation limited to explicit data-authoritative rebaseline.
- Replace historical semantic references with current owner relationships and self-contained wording.
- Preserve all current non-certificate authorization behavior and deterministic reporting.

**Non-Goals:**

- Define or implement a receipt producer, validator, token, wrapper, registry, feature flag, or alternate gate.
- Select or redesign SQLite or change service request/topology/operation-lifetime boundaries.
- Implement P/Q, rebuild, missing-member, platform, or hardware behavior.

## Decisions

### 1. Preserve the current metadata owner split

| Requirement | Local ownership after this change |
|---|---|
| `req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative` | Complete case inventory and per-case plan fields |
| `req.metadata-loss-recovery.evidence-gates-control-recovery-authorization` | Metadata-loss authorization dispositions, certificate unavailability, and operator-confirmation separation |
| `req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed` | Metadata-loss-specific consequences of unresolved identity/topology |
| `req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit` | Fresh metadata-loss manifest contents and eligibility consequence |
| `req.metadata-loss-recovery.dry-run-reporting-is-bounded-and-portable` | Deterministic portable matrix projection and non-authorizing output |

The change is a clarification and implementation correction, not a new product authorization. Historical-reference removal preserves semantics. Relationship markers make existing composition explicit.

### 2. Remove the forgeable value; do not replace it

Delete `MetadataLossVerification::CertifiedCleanEnvelope`. `MetadataLossPlan::authorize_inner` checks for `EvidenceRequirement::ValidatedCertificateReceipt` before ordinary evidence matching or operator confirmation and returns `MetadataLossError::CertificateReceiptUnavailable(case)`.

No receipt parameter or public construction path is added. `MetadataLossAuthorization` keeps private fields, so fresh-state construction remains reachable only through successful current authorization. Every current verification variant is tested against every receipt-gated case through both authorization entry points.

### 3. Keep the plan visible

Rename the descriptive plan requirement from `CertifiedCleanGate` to `ValidatedCertificateReceipt`, expose the dry-run evidence ID `validated-certificate-receipt`, and report `certificate_receipt_available=false` separately. The seven receipt-gated cases and all stable case IDs remain in the matrix. Their plan metadata remains inspectable while authorization fails deterministically.

The matrix version increments from 1 to 2 because the versioned dry-run evidence label and public verification enum change. Existing stable case IDs do not change.

### 4. Preserve current executable authorization paths

The exhaustive-match, exact verified-repair, validated-backup, reconciled-replica, and explicit operator-confirmed data-rebaseline paths retain their existing predicates. Refused ambiguity/coding cases remain refused. No certificate case falls through to any of these paths.

### 5. Declare only direct consequential relationships

| Source | Kind | Target | Reason |
|---|---|---|---|
| matrix | `requires` | metadata evidence gates | Every row's disposition and authorization availability use that owner |
| matrix | `requires` | envelope copies independently inspectable | Receipt-gated rows depend on bounded validated envelope content |
| matrix | `requires` | envelope disagreement conservative | Conflicting/missing copies determine unavailable/unsafe receipt evidence |
| matrix | `requires` | session transitions ordered | A future clean receipt must bind the owned clean-session transition |
| evidence gates | `requires` | exhaustive verification full scan | Exhaustive matching is an independently owned evidence result |
| evidence gates | `requires` | mismatch classification independent evidence | Metadata authorization consumes, and does not redefine, culprit classification |
| evidence gates | `requires` | checksum repair unique verified solution | The checksum-backed authorization path needs one identified solution |
| evidence gates | `requires` | checksum repair separate target/readback | Accepted repair evidence needs verified separate-target completion |
| evidence gates | `requires` | unknown/ambiguous evidence fail closed | Product-wide uncertainty constrains every metadata authorization result |
| identity/topology ambiguity | `requires` | identity evidence assessment | Metadata recovery consumes the current deterministic assessment classes |
| identity/topology ambiguity | `requires` | topology validation | Metadata recovery consumes validated roles, coding positions, and geometry |
| identity/topology ambiguity | `refines` | writable assembly fail closed | It adds metadata-loss-specific refusal and new-lineage consequences to the same policy |
| fresh state | `requires` | metadata evidence gates | Fresh-state creation consumes an authorized metadata plan |
| fresh state | `requires` | explicit immutable topology identities | The new manifest records the owner's distinct identity and epoch values |
| fresh state | `requires` | topology validation | Fresh state accepts only a validated topology |
| fresh state | `refines` | semantic export and health | It is the metadata-loss-specific explicit plan that may establish a new generation after missing/unhealthy state |
| dry run | `requires` | metadata matrix | The projection enumerates the exact owned case set |
| dry run | `requires` | evidence scope explicit | Output must preserve portable claim boundaries |
| dry run | `requires` | verification artifacts deterministic and bounded | The diagnostic matrix is a bounded reproducible artifact |

No target requirement is changed. The combined graph must remain acyclic. Only the five source requirements should receive relationship/local fingerprint changes; any transitive stale dependent must still be reviewed individually.

### 6. Remove current historical authority without rewriting history

The current metadata-loss spec purpose, requirements, scenarios, Rust module comment, test name, fresh-topology diagnostic, and actively mapped metadata verification record use descriptive current behavior. Archived changes and other historical records remain historical evidence and are not rewritten in this change.

## Risks / Trade-offs

- Removing a serialized enum variant is a breaking early-development API/format cutover. This is intentional; retaining it would preserve the authority defect.
- The matrix continues to describe future receipt-backed plans that cannot execute. The explicit unavailable outcome prevents that bounded design inventory from becoming false authority.
- Relationship additions change effective fingerprints. Exact change-impact output and individual reviewed reasons are required before completion.
