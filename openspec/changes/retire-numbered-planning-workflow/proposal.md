## Why

The numbered planning workflow has been retired, but the knowledge contract and tooling still treat the concrete `docs/milestones/**` path as a special source. That dormant path teaches a workflow that no longer exists and leaves current semantic isolation dependent on a retired directory branch.

## What Changes

- **BREAKING** Remove `docs/milestones/**` as a specially recognized source from knowledge collection, reference scanning, affected-path handling, and clean-room reporting.
- Replace only the current canonical references that still name the retired path or legacy milestone-planning request. Keep actual maintained historical sources concrete, opt-in, and non-authoritative; a recreated `docs/milestones/**` path is prohibited current input and knowledge tooling must fail closed with a bounded retired-path diagnostic before the path can contribute to extraction, references, affected-path analysis, readiness, direct knowledge output, or clean-room reconstruction.
- Keep legacy scenario headings in the delta only because OpenSpec's loss guard treats dropped scenario names as an unsafe whole-block replacement. Replacing those headings requires a separately approved requirement replacement or supported scenario-rename operation; scenario bodies must state the concrete historical-source or retired-path boundary.
- Keep broad isolation for archived changes, historical architecture, generated state, and other explicitly non-current sources.
- Add a fail-closed retired-path diagnostic for reintroducing `docs/milestones/**`, rather than silently excluding a recreated directory. Verify that every relevant knowledge entry point invokes the shared boundary before reading or accepting the path.
- Generalize or remove milestone-specific tests; retain tests for concrete historical isolation, retired-path rejection, and the entry-point boundary.
- Keep Beads as the current shared planning mechanism and preserve architecture versions `v0.8` and `v0.9`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `documentation-knowledge-architecture`: remove the retired numbered-planning path from current knowledge authority, context, affected-path, and clean-room semantics while preserving general historical isolation and explicit roadmap-context selection.

## Impact

- `openspec/specs/documentation-knowledge-architecture/spec.md` and its delta.
- `xtask/src/knowledge.rs` collection, reference, affected-path, entry-point preflight, and retired-path diagnostics plus focused tests.
- `xtask/src/lib.rs` clean-room result reporting.
- `openspec/config.yaml`, `docs/README.md`, `docs/reviewed-requirements.toml`, current agent guidance, and any other maintained references that still describe numbered milestone documents.
- No product data format, storage behavior, Rust product API, or delegated model semantics changes.

