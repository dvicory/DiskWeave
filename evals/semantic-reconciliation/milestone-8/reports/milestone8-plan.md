# Milestone 8 execution plan

## Status entering this plan

- `canonical-semantic-ownership`, `metadata-certificate-authority-gate`, and `normalized-service-request-boundary` are approved, synchronized, archived, and post-archive validated.
- `milestone-planning-cutover` is the only active OpenSpec change.
- Canonical inventory: 25 capabilities, 152 requirements, 68 relationships before the planning delta.
- Remaining work is WP-8.5 planning cutover followed by WP-8.6 integrated evidence and final external review.

## 1. Complete `milestone-planning-cutover`

1. Validate the proposal/design/spec/tasks and the semantic ownership decision for the five existing documentation requirements plus the new active-roadmap requirement.
2. Sync the accepted delta into the current documentation-knowledge specification, retaining every existing stable ID and adding only `req.documentation-knowledge-architecture.active-roadmap-is-explicit-and-non-authoritative`.
3. Move the seven retained predecessor planning records one-to-one to `docs/milestones/m1.md` through `m7.md`; add the non-authority banner; preserve technical chronology.
4. Delete the superseded incomplete archive and rename the completed milestone-seven acceptance archive without aliases, redirects, tombstones, duplicate copies, or mapping files.
5. Rewrite all retained project-planning titles, paths, labels, archive references, prompts, tests, fixtures, comments, generated inputs, and Linux scratch names to milestone or functional terminology. Preserve ordinary English and `Goals / Non-goals` uses.
6. Extend the existing bounded readiness validator to reject retired planning path/text identifiers and report exact path/line diagnostics; add focused positive/negative tests.
7. Prove `docs/milestones/**` remains excluded from requirements, relationships, current context/ownership, affected-path analysis, review freshness, and clean-room reconstruction.
8. Self-sanitize `docs/milestones/m8.md`; replace migration inputs with observed completion history and the active-roadmap requirement disposition.
9. Resolve each stale requirement individually; run strict OpenSpec validation, xtask tests, docs readiness/check/build/clean-room, workspace tests, formatting, strict Clippy, residual scan, path inventory, and maintained-link validation.
10. Obtain external implementation approval, archive the change, confirm no active change remains, and rerun strict post-archive OpenSpec and documentation gates.

## 2. Regenerate final integrated evidence

1. Rerun final portable gates from the no-active-change source: strict OpenSpec, xtask, docs readiness/check/build/clean-room, workspace tests, formatting, Clippy, metadata certificate tests, and request/frontend tests.
2. Rerun the retained recovery TLC model, the independent TLA checker, dirty-region Kani, fence-coverage Kani, and deterministic partial multi-store fence regression. Record commands, versions, bounds, states, assumptions, unreachable properties, and explicit non-claims.
3. Run a fresh live Linux ublk/ext4 acceptance workflow using only functional runtime names. Replace acceptance JSON and both traces only from this run.
4. Replay both final traces through the root CLI. Record final revision/tree/source digest, host/guest/kernel/endpoint, queue and trace bounds, workload/refusal results, trace counts/sizes/digests, acceptance digest, payload/member digests, restart/read-only-member result, and non-claims.
5. Run final readiness/check, inspect every stale requirement, resolve each individually, repeat the ownership challenge across all Milestone 8 semantic clusters, and confirm history is unnecessary for current semantics.
6. Complete `docs/verification/m8.md` with all four archives, all finding dispositions including CR-001 and frontend CR-012, canonical inventory/schema counts, ownership table, certificate and request outcomes, migration/isolation proof, model/Linux evidence, deleted/renamed paths, residual scan, clean-room digest, and only actual deferred post-M8 work.

## 3. Final external review gate

1. Write a self-contained review package at `/Users/daniel.vicory/tmp/milestone8-final-completion-review.md` with exact commands/results and source-control summaries.
2. Request independent external review of the complete final tree.
3. Keep Milestone 8 pending and do not start later roadmap work until explicit approval is received.
4. End the pre-review report with exactly: `Milestone 8 implementation remains pending: stop here for external final-completion review; do not start later roadmap work.`
