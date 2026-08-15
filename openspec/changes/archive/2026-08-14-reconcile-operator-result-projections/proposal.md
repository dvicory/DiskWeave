## Why

The repository needs one canonical owner for the shared rule that alternate representations preserve the same consequential result. Without that owner, generic projection wording can be duplicated across capabilities, and prose cleanup can accidentally weaken an existing refusal, uncertainty boundary, or action-relevant finding.

## What Changes

- Add a canonical architecture-contract requirement for preserving consequential meaning across external representations.
- Require structured output to retain the complete bounded result required by the applicable capability contract.
- Permit human output to select, summarize, aggregate, reorder, or omit only detail that is both non-consequential and not otherwise required by that capability contract.
- Forbid any representation from strengthening a claim, weakening a refusal or uncertainty boundary, or otherwise contradicting the evaluated result.
- Add dependency edges to affected capability requirements and remove only their duplicated generic projection wording.
- Preserve every existing exact bounded-exception, bounded-manifest, command-identity, causal-reason, refusal, uncertainty, status, and next-action obligation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `architecture-contract`: add the canonical requirement and scenarios for consequential meaning across result projections.
- `operator-recovery`: make the two affected requirements depend on the shared owner while retaining all local assessment and result-boundary facts.
- `independent-recovery-inspection`: make the inspection requirement depend on the shared owner while retaining the bounded manifest requirement.
- `macos-demo-cli`: make the versioned contract depend on the shared owner while retaining command identity, outcome, and bounded diagnostics.

## Impact

The change updates canonical OpenSpec ownership and documentation-knowledge dependency edges without narrowing existing behavior. It does not authorize implementation, renderer changes, format changes, compatibility changes, or operator-visible recovery behavior changes.
