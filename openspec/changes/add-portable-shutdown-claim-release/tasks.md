## 0. Writable-Session Prerequisite

- [ ] 0.1 Complete the recovered `define-portable-writable-session-lifecycle` external target prerequisite under Bead `dwv-hg0.4`, then add its normal owner edge only after the proposed lifecycle completes its canonical transition; before product implementation, update this change to consume durable begin/close meaning, exact admissible close evidence, and failure/restart consequences without treating the target requirement as current authority.

## 1. Portable Shutdown Contract

- [ ] 1.1 Implement the portable shutdown ordering and result boundary from `req.healthy-portable-io.portable-shutdown-preserves-operation-ownership-and-claim-release-ordering`: close admission; quiesce; establish the shutdown close frontier; require every operation generation still owned at that frontier to reach safe `Reclaimable` with every child terminal and required reconciliation recorded; require each such operation's applicable operation/media effect to be terminal or authoritatively reconciled; cover already released mutation history with the closed-mutation-set owner's exact bounded lower-frontier/coverage evidence rather than an unbounded retained operation-slot ledger; obtain owner-approved recovery-`CLEAN` and durable session-close evidence that consumes those facts; withdraw endpoints; then release each claim only when its independent owner permits it and no writable alias remains.
- [ ] 1.2 Preserve the existing frontend-lifecycle, operation-slot, operation/media-effect, transaction, dirty/restart, store-watermark, persistence-evidence, and generation-qualified release-authorization owners; do not let session close or recovery `CLEAN` manufacture child terminality, reconciliation, `Reclaimable`, or effect disposition, and do not cancel abandoned work, infer clean state, retry uncertain writes, or release live ownership early.
- [ ] 1.3 Apply the Linux frontend refinement so OS descriptor claims, owned ublk endpoint removal, stale cleanup, process death, and alias refusal conform to the portable owner without duplicating generic shutdown policy.
- [ ] 1.4 Preserve explicit non-authorizations: shutdown does not establish custody continuity, current or historical protection, lineage, recovery authority, payload integrity, publication, currentization, retention, repair, or stable-format status.

## 2. Focused Observable Evidence

- [ ] 2.1 Prove a clean stop is reported only after admission closure; the shutdown close frontier is established; every operation generation still owned at that frontier is safe `Reclaimable` with every child terminal and required reconciliation recorded; each such operation's applicable operation/media effect is terminal or authoritatively reconciled; already released mutation history has the closed-mutation-set owner's exact bounded lower-frontier/coverage evidence; exact recovery-`CLEAN` and durable session-close evidence is accepted without manufacturing those facts; endpoints are withdrawn; and every released claim satisfies its independent owner predicate.
- [ ] 2.2 Prove that missing operation-slot, child-terminal, reconciliation, `Reclaimable`, or operation/media-effect evidence for a generation still owned at the shutdown close frontier, missing exact bounded lower-frontier/coverage evidence for already released mutation history, or missing persistence, recovery-`CLEAN`, session-close, endpoint-withdrawal, or alias evidence produces failure or reconciliation-required state and never a clean-close claim.
- [ ] 2.3 Prove frontend abandonment, forced stop, and owner-process loss retain operation ownership and preserve dirty/indeterminate or stale-endpoint consequences until the owning lifecycle permits reclamation.
- [ ] 2.4 Prove the observed result makes no recovery, protection, lineage, integrity, destructive, compatibility, or stable-format claim beyond the shutdown contract, using focused scenarios rather than source-text assertions or broad snapshots.

## 3. Strict Verification and Canonicalization

- [ ] 3.1 Run the focused product checks and executable smoke scenarios for the portable and Linux shutdown boundaries; resolve failures without expanding into startup, post-gap publication, recovery mutation, or the independent recovery-inspection change.
- [ ] 3.2 Run strict OpenSpec verification against proposal, delta specs, design, tasks, implementation, and focused evidence; resolve every contract or incomplete-task finding before archival.
- [ ] 3.3 Archive the verified change only after verification succeeds, add canonical implementation/evidence links for the new portable owner and Linux refinement, and review every affected requirement dependent individually with a concrete reason.
- [ ] 3.4 Run the required knowledge readiness, documentation check, and documentation build gates at the change boundary; keep evidence claims scoped to the exercised shutdown tier.

## 4. Campaign and Bead Closure

- [ ] 4.1 Update only the **portable shutdown, endpoint withdrawal, and claim-release ordering** campaign row and selected-capability/resumption sections with the durable OpenSpec and Bead anchors, rejected-candidate reasons, target/current posture, inherited-obligation result, and remaining open boundaries; do not claim canonicalization before archival.
- [ ] 4.2 Export the shared Beads database and refresh the viewer JSONL after every dependency transition; confirm the exported graph keeps `dwv-hg0.1` blocked on `dwv-hg0.4`, then preserves implementation-to-evidence-to-canonicalization ordering after the prerequisite closes.
- [ ] 4.3 Close the evidence and canonicalization Beads only after their acceptance criteria, archive, canonical links, dependent reviews, campaign update, documentation gates, and viewer export complete; close the feature Bead last and leave all unrelated campaign chains open.
