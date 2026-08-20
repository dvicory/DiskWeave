## Why

DiskWeave's current behavior is coherent, but several names hide distinct facts. Recovery-specific `intent`, `home`, claim-level `fence`, recovery `checkpoint`, write-lifecycle states, and Guide naming make the write/recovery path harder to understand than the semantics require. Numeric verification and planning labels also force readers to look up meaning that can be stated directly.

This change preserves product/runtime semantics while migrating vocabulary and identities. It does not change the portable protocol, delegated `RecoveryProtocol` relation, recovery authority, persistence requirements, range ownership, or release behavior. It adds one documentation-maintenance rule: Concepts and Terminology is maintained incrementally from current provenance instead of being regenerated wholesale.

## What changes

- Use **write-recovery record** for the recovery-specific prerequisite currently described as intent.
- Use **data/parity write** for protected block writes; use **data/parity change** only when the category is genuinely broader than a write.
- Use **persistence evidence** for claim-level durability evidence while preserving exact store, incarnation, ordering, watermark, range, topology, capability, and generation scope.
- Call the recovery-state operation **commit recovery state `CLEAN`**; retain unrelated SQLite and rebuild checkpoints as their own concepts.
- Rename the delegated Quint state, outcome, field, action, and invariant labels so state, uncertainty, lifecycle, reconciliation, ownership, and release are explicit.
- Replace opaque current verification/evidence/planning labels with descriptive names or existing semantic IDs. Preserve meaningful planning categories as record structure, not opaque codes.
- Rename the affected `req.*` identities while preserving review outcomes and reasons after fail-closed graph/model equivalence.
- Use one disposable Python migration for scenario-heading and other one-time mechanical rekeys that OpenSpec cannot express losslessly.
- Rename Human Guide to **DiskWeave Guide**. Add a focused documentation requirement that keeps Concepts and Terminology as persistent checked-in explanatory Markdown maintained incrementally from provenance.
- Normalize every still-active OpenSpec change before it resumes so current work no longer teaches retired vocabulary.

## Scope and gates

The affected canonical capabilities are represented by the delta specs in this change. Capability directory names and the Guide source directory are intentionally unchanged.

Before Gate #1, this change is planning only. Current canonical specs, the delegated model, implementation, public/schema surfaces, verification evidence, Guide, Beads, active changes, current architecture/planning, and `docs/reviewed-requirements.toml` remain unchanged.

After Gate #1, implementation prepares the complete target in a non-authoritative candidate tree. Gate #2 reviews that exact target and its equivalence evidence. Only after Gate #2 may the real canonical cutover occur.

## Non-goals

- No new runtime behavior, transition, invariant, authority, persistence claim, recovery rule, ownership rule, or release rule.
- No permissive alias layer for retired current names.
- No permanent rename/rekey subsystem for this one migration.
- No global replacement of narrower request `intent`, low-level fences, rebuild/SQLite checkpoints, genuine terminal child results, payload nouns, generation, baseline, lineage, or currentization.
- No capability-directory or Guide-source-directory rename.
- No decision about future filesystem semantics, replication, hardware failure guarantees, currentization, lineage, retention policy, or stable-format graduation.
