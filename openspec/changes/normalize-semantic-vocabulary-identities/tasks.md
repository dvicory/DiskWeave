## Gate #1

- [ ] 0.1 Obtain Gate #1 approval for Design "Product vocabulary", "Delegated model rename map", "Identity policy and migration maps", "Canonical OpenSpec migration", "DiskWeave Guide and Concepts", and "One-time migration mechanics".
- [ ] 0.2 Freeze the approved files under `migration/` and the delta set. Keep current canonical specs, model, implementation, Guide, active changes, current architecture/planning, Beads, and reviewed state unchanged until Gate #1 is approved.

## Build the candidate target

- [ ] 1.1 Apply the canonical requirement/body/relationship migration defined in Design "Canonical OpenSpec migration".
- [ ] 1.2 Apply the delegated relation label map in Design "Delegated model rename map" without changing state cardinality, guards, transitions, invariants, bounds, or release behavior.
- [ ] 1.3 Apply the requirement, verification, evidence, and planning-name maps in Design "Identity policy and migration maps".
- [ ] 1.4 Normalize every still-active OpenSpec change and current architecture/planning surface according to Design "Active changes and current guidance"; remove current rules that require retired opaque planning IDs.
- [ ] 1.5 Prepare the exact terminology/identity updates for current actionable Beads according to Design "Active changes and current guidance"; preserve `dwv-*` IDs, graph, status, scope, and acceptance meaning, and do not apply the shared Bead edits before Gate #2.
- [ ] 1.6 Apply the DiskWeave Guide, projection-key, curriculum, and incremental Concepts maintenance contract in Design "DiskWeave Guide and Concepts"; do not move the Guide source directory.
- [ ] 1.7 Apply downstream Rust/API/trace/schema vocabulary changes required by the approved map, retaining narrower request-intent, low-level-fence, checkpoint, terminal-result, and other explicitly excluded concepts.

## One-time migration and equivalence

- [ ] 2.1 Run `migration/migrate.py` against the non-authoritative candidate target as specified in Design "One-time migration mechanics", including approved scenario headings, exact canonical explanatory text, and reviewed-state rekeys.
- [ ] 2.2 Prove the requirement graph and reviewed outcomes/reasons are equivalent under the approved requirement map; refresh fingerprints only through existing knowledge tooling.
- [ ] 2.3 Prove the delegated model is structurally equivalent after label canonicalization and run its bounded verification/Connect evidence.
- [ ] 2.4 Run the complete scenario-heading/body consistency check and fail on any retired current heading not covered by the migration map or an explicit precise-retention decision.
- [ ] 2.5 Define and test explicit versioned migration or refusal for any external/persisted discriminator touched by the rename.

## Gate #2 and cutover

- [ ] 3.1 Run strict OpenSpec validation, knowledge readiness, `cargo xtask docs check`, docs build, focused Rust/trace tests, and compatibility checks on the complete candidate target.
- [ ] 3.2 Present the exact candidate target and equivalence evidence for Gate #2. Do not modify current authority before approval.
- [ ] 3.3 After Gate #2, perform the real cutover in the order defined by Design "Gates and cutover": apply the reviewed non-spec target and reviewed current-Bead edits, sync the approved OpenSpec deltas once, run the disposable migration, and refresh reviewed fingerprints; do not commit a half-migrated canonical state.
- [ ] 3.4 Re-run the full validation/equivalence suite. If every check passes, archive the already-synced change without another spec sync; do not restore the old parser-locator scenario headings.
