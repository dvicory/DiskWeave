## Context

See `proposal.md` for the motivation and scope. The current knowledge implementation has two different concepts mixed together:

- broad non-current source isolation for hidden paths, archived changes, archived architecture, generated output, and temporary material; and
- a special `docs/milestones/**` branch in Markdown collection, reference scanning, impact-candidate handling, affected-path validation, roadmap scanning, milestone fixtures, and clean-room reporting.

The numbered files have been removed. The canonical documentation-knowledge requirements still name milestones as a source or planning concept in several current blocks, and the OpenSpec configuration still mentions milestone allocation. The existing retired-planning classification and diagnostic contract is the smallest existing semantic seam for rejecting reintroduction rather than silently supporting a retired directory. The implementation must verify the complete knowledge call graph rather than assume that any one entry point or complete repository-wide scan is an adequate boundary.

## Goals / Non-Goals

**Goals:**

- Make numbered planning retirement explicit in the canonical documentation contract.
- Remove special knowledge behavior whose only purpose was to treat `docs/milestones/**` as an excluded historical source.
- Preserve general historical and generated-state isolation.
- Fail closed if `docs/milestones/**` is recreated, with a diagnostic that directs work to Beads or a functional planning artifact, before any normal knowledge output accepts it.
- Keep actual maintained historical source classes concrete, opt-in, and non-authoritative; do not invent an abstract current source class such as "retired planning material".
- Keep architecture-roadmap selection and non-authoritative candidate/history behavior intact.

**Non-Goals:**

- No product storage, recovery, Rust API, serialized-format, or delegated-model change.
- No rewriting of archived changes, architecture versions, or historical outcomes.
- No broad removal of the existing retired-project-identifier scanner; only its milestone-specific guidance and the retired-path case change.
- No new planning registry or replacement numbered namespace.

## Decisions

1. **Use one shared retired-path behavior and diagnostic boundary.** Reuse the existing retired-planning classification and diagnostic contract for `docs/milestones/` and its descendants. Prefer a bounded retired-path predicate/preflight that each relevant knowledge consumer invokes before reading or accepting the path; `planning_nomenclature` SHALL use that same boundary. The invariant is one behavior/diagnostic boundary, not one complete repository-scan invocation. Readiness and other consumers SHALL surface the existing `retired_planning_identifier` error with the `retired-planning-path` diagnostic, directing the maintainer to remove the path and use Beads or a functional planning artifact.

   The implementation must trace and exercise every current entry point that can collect Markdown, scan references, validate affected paths, calculate readiness, or reconstruct a clean room. None may read, accept, or silently exclude a recreated `docs/milestones/**` path before the fail-closed diagnostic.

   Alternatives rejected:
   - Keep `excluded()` support: silently preserves the retired workflow's knowledge semantics.
   - Add a separate full-repository scanner solely for path rejection: duplicates repository traversal, bounds, symlink, and diagnostic handling instead of reusing the existing boundary.
   - Require every entry point to invoke a complete `planning_nomenclature` repository scan: couples each consumer to an unnecessarily broad traversal without strengthening the shared behavior boundary.
   - Reject every occurrence of the word `milestone`: would flag historical architecture/provenance and ordinary non-workflow prose.

2. **Remove only the `docs/milestones` special cases.** Delete the early return in Markdown collection and the `excluded()` path clause. Relevant consumers must invoke the shared bounded preflight before normal scanners read or accept the path. Remove the milestone-specific affected-path rejection; a path must not receive a hidden special status. Keep exclusions for actual historical, archived, generated, hidden, and temporary source classes.

3. **Generalize canonical wording without changing stable requirement identities.** Preserve the existing `req.documentation-knowledge-architecture.*` identities because they still govern current semantic discovery, context, and historical isolation. Update every current requirement block that names milestone-specific identity inputs, packet contents, relationship sources, historical-source records, or roadmap requests:
   - use concrete `docs/milestones/**` rejection for the retired path;
   - keep actual historical architecture and archived-change sources opt-in and non-authoritative;
   - call deliberate whole-roadmap selection a `roadmap-context` request;
   - remove the retired word from ordinary packet and relationship-source lists where the broader rule already covers it.

   OpenSpec 1.8 rejects a `MODIFIED` block that drops or renames an existing scenario, so this delta preserves the existing scenario headings rather than silently losing contract coverage. Retiring those headings is a separate, explicitly approved requirement replacement or tooling change, not an implementation-side text edit.

4. **Replace milestone-specific tests with contract tests.** Remove tests that prove milestone files are ignored, milestone edits are semantically invisible, or milestone paths are rejected as excluded affected inputs. Keep the general historical/hidden/archived exclusion assertions. Add focused regressions proving that a recreated `docs/milestones/m11.md` fails closed with an exact retired-path diagnostic and that every relevant knowledge entry point observes that failure. Existing clean-room reconstruction tests continue to prove active architecture retention and candidate/archive removal without treating the retired path as a historical source.
5. **Remove milestone wording from current planning guidance.** Keep Beads as the current planning mechanism and state that numbered planning documents are not created. Update `openspec/config.yaml`, `docs/README.md`, `docs/reviewed-requirements.toml`, and current agent guidance where the wording is independently about the retired workflow; preserve reviewed outcomes while updating affected reason text to concrete historical-source wording as required by docs checks. Do not alter historical architecture or archived change prose.

6. **Audit residual current occurrences before review.** Search current non-historical OpenSpec/config/planning and agent-context surfaces after the edits. Remove each independently retired-workflow occurrence or record why it remains (for example, a legacy scenario heading retained by the OpenSpec loss guard or non-authoritative architecture/history outside the acceptance surface). Do not expand this change into the broader OS/semantic-ID migration.

## Risks / Trade-offs

- **[Risk]** A future maintainer recreates the retired directory and one less-obvious knowledge entry point accepts it. → **Mitigation:** trace the call graph, route every relevant consumer through one shared bounded diagnostic boundary, and exercise collection, references, affected paths, readiness, direct knowledge commands, and clean-room reconstruction.
- **[Risk]** Generalizing the canonical requirement could hide a missing historical source boundary. → **Mitigation:** name actual retained source classes explicitly, state that `docs/milestones/**` is prohibited rather than historical, and run readiness, docs check, clean-room reconstruction, and focused tests.
- **[Risk]** The retired-path preflight may report a path before its contents are examined. → **Mitigation:** path rejection is the intended fail-closed result; one exact diagnostic is more useful than duplicate content diagnostics for an invalid retired path.
- **[Risk]** Existing scenario headings still expose retired terminology after this delta. → **Mitigation:** do not pretend a `MODIFIED` delta can remove them; carry the exact OpenSpec validation failure/constraint into Gate #1 and obtain a separate decision before any requirement replacement or scenario-rename tooling work.

## Migration Plan

1. **Stop this session before implementation.** Daniel reviews the proposal, design, delta, and tasks, including the scenario-heading constraint, concrete source/path boundary, and entry-point coverage, before any `openspec apply`, source edit, test edit, or current-spec authority transition.
2. After explicit approval, apply the delta through the normal OpenSpec workflow.
3. Remove the special collector/reference/affected-path and clean-room wording branches.
4. Add the shared retired-path diagnostic/preflight and entry-point regressions.
5. Update current planning guidance and run OpenSpec validation, knowledge readiness/check/build, clean-room reconstruction, and focused `xtask` tests.
6. Report every remaining current `milestone` occurrence and its disposition before the post-implementation review. Stop before sync/archive for independent user review. Sync/archive remains the authority transition after review; until then this change remains proposed target behavior. Reverting the change restores the prior implementation and delta without changing product data or formats.

## Open Questions

None. The remaining choices are implementation details covered by the decisions above. The separate OpenSpec rename-mechanics investigation requested for `dwv-o2g.2` is not part of this milestone-workflow retirement change.
