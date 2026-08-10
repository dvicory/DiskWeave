## Why

The current metadata-loss specification already says certificate-gated cases are non-authorizing until a validator-issued receipt exists, but the public Rust verification enum lets any caller construct `CertifiedCleanEnvelope` and obtain authorization. Current metadata semantics also still depend on historical work IDs and handoff section numbers instead of current canonical owners.

## What Changes

- **BREAKING** Remove the caller-constructible certified-envelope verification value and make every certificate-gated matrix case return the stable `CertificateReceiptUnavailable` outcome for every currently constructible verification value, including operator-confirmed calls.
- Keep every certificate-gated case as an inspectable bounded plan; do not synthesize a receipt API, producer, validator, feature flag, token, wrapper, or alternate authorization path.
- State the canonical prerequisites for any future receipt: producer and validator authority, exact array/topology/profile/session/envelope bindings, generation and freshness, replay or expiry rules, and evidence/failure semantics.
- Replace OS numbers, named gates, handoff/architecture section numbers, and other historical authority wording in current metadata-loss semantics and implementation diagnostics with self-contained current behavior.
- Preserve all stable metadata-loss requirement IDs, the complete 18-case matrix, exhaustive matching, uniquely verified separate-target repair, validated backup/replica handling, explicit operator-confirmed data rebaseline, fail-closed ambiguity, fresh-state rules, and portable bounded dry runs.
- Add only consequential current `requires`/`refines` relationships to the existing topology/identity, parity verification, checksum repair, parity-envelope/session, recovery-state, and evidence-boundary owners.
- Bump the versioned metadata-loss matrix output because its certificate evidence label and authorization outcome change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `metadata-loss-recovery`: make absent certificate-receipt authority explicit and non-forgeable, remove historical semantic dependencies, and declare current owner relationships.

## Impact

- Canonical spec: `openspec/specs/metadata-loss-recovery/spec.md` through this change's delta.
- Implementation: `crates/dwv-recovery/src/metadata_loss.rs` and affected tests/callers; `dwv-service` behavior remains the current verifier-to-metadata classification boundary.
- Reviewed semantic fingerprints and ownership output change for the five metadata-loss requirements and their dependency closure.
- The public `MetadataLossVerification` enum loses `CertifiedCleanEnvelope`; consumers must use an actually executable current evidence path.

## Non-goals

- Selecting SQLite, changing SQLite durability or lost-acknowledgement semantics, or redesigning the recovery adapter.
- Creating a certificate receipt API or implementing future envelope validation authority.
- Changing service request identity, topology resolution architecture, operation admission, buffer/resource ownership, or Linux translation.
- Broadening or narrowing unrelated metadata authorization paths, implementing P/Q recovery, or adding platform/hardware claims.
