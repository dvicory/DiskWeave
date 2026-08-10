## 1. Canonical documentation contract

- [x] 1.1 Reconcile the five existing documentation-authority requirements and the new active-roadmap requirement against current implementation and evidence
- [x] 1.2 Apply the documentation delta to the current canonical specification while preserving every existing stable requirement ID
- [x] 1.3 Add implementation and verification links for the new active-roadmap requirement without creating a second semantic registry
- [x] 1.4 Resolve every changed effective fingerprint individually after inspecting its current context and ownership

## 2. Planning record and archive cutover

- [x] 2.1 Move the seven retained predecessor records one-to-one to `docs/milestones/m1.md` through `m7.md`
- [x] 2.2 Add the historical-planning non-authority banner and preserve technical chronology in every moved record
- [x] 2.3 Delete the complete superseded incomplete archive without a tombstone, redirect, alias, or duplicate
- [x] 2.4 Rename the completed milestone-seven acceptance archive and preserve its checked tasks, accepted delta, evidence meaning, and chronology
- [x] 2.5 Rewrite all inbound planning paths, titles, sequence labels, completion labels, and retained archive references to milestone terminology

## 3. Repository-wide identifier migration

- [x] 3.1 Rewrite retained planning references in current documentation, ADRs, prompts, agent guidance, tests, fixtures, comments, and generated-input configuration
- [x] 3.2 Change Linux acceptance VM, fixture-root, mountpoint, and guest-evidence names to the functional names and update focused script tests
- [x] 3.3 Leave retained machine-generated Linux evidence and traces untouched for replacement by the final live run
- [x] 3.4 Preserve ordinary English/product uses and `Goals / Non-goals` headings

## 4. Deterministic enforcement and isolation

- [x] 4.1 Extend the existing bounded readiness validator to reject retired planning paths and textual identifiers with exact path and line diagnostics
- [x] 4.2 Add focused positive and negative scanner tests without exempting a production source path
- [x] 4.3 Prove milestones remain excluded from requirement discovery, relationships, context/ownership references, affected-path analysis, review freshness, and clean-room inputs
- [x] 4.4 Add only missing focused isolation regressions and keep existing source-root allowlists authoritative
- [x] 4.5 Self-sanitize `m8.md`, record the active-roadmap requirement decision, and retain it only as non-authoritative completion history

## 5. Validation and archive

- [x] 5.1 Run strict validation for this change and all OpenSpec artifacts
- [x] 5.2 Run the xtask suite, retired-identifier scan, maintained Markdown link validation, and exact path inventory checks
- [x] 5.3 Run documentation readiness, consistency, build, and clean-room reconstruction
- [x] 5.4 Run complete workspace tests, formatting check, and strict workspace Clippy
- [x] 5.5 Record exact cutover evidence and residual non-claims in `docs/verification/m8.md`
- [ ] 5.6 Archive the approved change, confirm no active OpenSpec change remains, and rerun strict OpenSpec and documentation gates
