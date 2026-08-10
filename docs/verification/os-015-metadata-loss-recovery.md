# Metadata-loss recovery verification

## Claim boundary

This evidence covers the current portable metadata-loss case matrix, fail-closed authorization, explicit operator-confirmed rebaseline policy, bounded current-verification translation, audited fresh semantic state for completed exhaustive all-data/single-parity recovery, an evaluation-only SQLite target, and ordinary-file payload preservation. It does not claim a validator-issued certificate receipt, P/Q execution, degraded reads, parity/new-lineage rebuild execution, a live frontend, production SQLite durability, or hardware power-loss safety.

Certificate-receipt-gated cases remain inspectable plans but cannot authorize recovery because the current product has no receipt producer, validator, or non-forgeable receipt type.

## Current requirement mapping

- `req.metadata-loss-recovery.the-metadata-loss-matrix-is-total-and-conservative`: `MetadataLossCase::ALL` contains 18 stable cases, each with an action, disposition, evidence requirement, confirmation requirement, baseline disposition, and non-in-place payload-write policy.
- `req.metadata-loss-recovery.evidence-gates-control-recovery-authorization`: certificate-receipt-gated rows return `CertificateReceiptUnavailable`; exhaustive matches and exact run-bound verified repairs remain authorized where the matrix allows them; ambiguous, incomplete, and conflicting evidence fails closed; operator confirmation only enables explicitly data-authoritative rows.
- `req.metadata-loss-recovery.identity-and-topology-ambiguity-fails-closed`: recovery topology comes from validated array, slot, role, coding-position, assignment, and store-mapping evidence; ambiguous identities and lost coding positions refuse automatic recovery.
- `req.metadata-loss-recovery.fresh-recovery-state-records-a-new-baseline-and-audit`: only completed exhaustive all-data/single-parity recovery creates fresh state, with generation zero, an explicit checksum-baseline obligation, and a metadata-loss audit.
- `req.metadata-loss-recovery.dry-run-reporting-is-bounded-and-portable`: the deterministic portable dry run emits all 18 rows and reports certificate receipt as required or not required and unavailable.

## Implementation evidence

- `dwv-recovery::metadata_loss` owns the bounded matrix, exact per-row assertions, authorization rules, deterministic dry-run rendering, lineage audit, and fresh-manifest construction.
- `MetadataLossVerification` exposes current evidence classifications but no certificate receipt or caller-constructible equivalent. Every plan requiring `ValidatedCertificateReceipt` returns `CertificateReceiptUnavailable` from both ordinary and operator-confirmed authorization.
- Recovery topology can only be constructed from a validated `dwv-core::TopologySnapshot` plus a unique one-to-one runtime store mapping. Fresh-state lineage is derived from that topology rather than supplied independently.
- The semantic recovery schema is version 2. The metadata-loss matrix is version 2. The evaluation SQLite adapter preserves its physical `user_version=1` schema while persisting the semantic metadata-loss audit.
- `dwv-service::classify_metadata_loss_verification` translates bounded verification reports into current authorization evidence. Identified mismatches remain pending until exact run-bound repair receipts cover them.
- `dwv-recovery-sqlite::SqlitePrototype::recreate_from_metadata_loss` atomically reserves only an absent target, removes the reservation on failure, preserves any existing database or forensic source, and persists every fixed audit field in the semantic summary.

## Acceptance commands

- `openspec validate --strict --all`: passed 26/26 current specs and active changes.
- `cargo test -p dwv-recovery metadata_loss`: passed 7 tests.
- `cargo test -p dwv-service metadata_loss`: passed 3 tests across 2 suites.
- `cargo test -p dwv-recovery-sqlite metadata_loss`: passed 1 test.
- `cargo test -p xtask`: passed 32 tests across 3 suites.
- `cargo xtask docs knowledge readiness`: ready with 152 requirements and all seven gate counts zero.
- `cargo xtask docs check`: ready with no review-required requirement.
- `cargo xtask docs build`: passed with 152 projected objects.
- `cargo xtask docs clean-room`: equivalent reconstruction with object digest `1272ec5c70816efa0e9429a80367e65e90ead765960039456618e8a0c12b4c66`.
- `cargo test --workspace`: passed 289 tests across 34 suites; 1 test was ignored.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`: passed.
- `cargo run -q -p dwv-recovery --example metadata-loss-dry-run`: passed and printed matrix v2 with all 18 cases, explicit certificate-receipt requirements, and `certificate_receipt_available=false`.
