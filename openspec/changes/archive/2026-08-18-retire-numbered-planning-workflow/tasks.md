## 1. Remove retired-directory special handling

- [x] 1.1 Trace all current knowledge entry points, then remove `docs/milestones/**` branches from Markdown collection, reference scanning, impact-candidate classification, affected-path validation, and any clean-room reporting while preserving broader historical and generated-source isolation.
- [x] 1.2 Reuse or extract one bounded fail-closed retired-path predicate/preflight for `docs/milestones/**`; every relevant knowledge entry point must invoke the shared boundary before reading or accepting the path, and `planning_nomenclature` must use the same boundary. Update the diagnostic action to direct work to Beads or current product terminology.
- [x] 1.3 Remove retired-path clean-room reporting and replace stale current planning/source guidance with Beads and concrete historical-source wording; update `openspec/config.yaml`, `docs/README.md`, `docs/reviewed-requirements.toml`, and current agent guidance where independently applicable, preserving reviewed outcomes.

## 2. Generalize canonical and test contracts

- [x] 2.1 Keep the canonical requirement identities stable while updating every affected documentation-knowledge requirement block: concrete retired-path rejection, actual historical-source isolation, explicit roadmap-context boundaries, architecture packet wording, and relationship-source wording; preserve existing scenario headings because OpenSpec rejects an unsafe whole-block scenario loss.
- [x] 2.2 Remove tests that treat `docs/milestones/**` as a silently excluded historical source and add focused regressions proving the shared boundary's exact retired-path failure through collection, references, affected paths, readiness, direct knowledge commands, and clean-room reconstruction.
- [x] 2.3 Validate the complete OpenSpec change and confirm every modified requirement block is complete and structurally valid.

## 3. Review boundary and post-approval verification

- [x] 3.1 Stop before `openspec apply`, source edits, test edits, or current-spec authority transition; present the complete proposed delta, concrete source/path boundary, entry-point coverage, and scenario-heading constraint for explicit review.
- [x] 3.2 After approval, run knowledge readiness, docs check/build, clean-room reconstruction, and focused `xtask` tests; record exact results and non-claims.
- [x] 3.3 Prepare the final proposed delta and implementation evidence for independent review; report residual current terminology with dispositions and do not sync current specs or archive this change until approval.
