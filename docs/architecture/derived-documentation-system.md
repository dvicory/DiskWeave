# Traceable documentation and knowledge system

**Status:** Current architecture

## Authority and boundary

Current canonical OpenSpecs under `openspec/specs/*/spec.md` and the small architecture constitution own intended DiskWeave behavior. Human guides, generated needs, Rust markers, evidence views, contributor maps, API pages, context packets, and AI-maintained prose are projections. They can explain or link authority; they cannot strengthen it.

Normal discovery excludes handoffs, archived changes, generated output, and temporary state. Historical material is available only through an explicit archaeology workflow and never resolves a current conflict.

## Permanent inputs

The maintained surface is deliberately small:

- canonical OpenSpecs with one colocated `dwv:req` semantic ID per requirement;
- sparse `dwv:req` Rust doc markers on meaningful ownership boundaries;
- `verification/manifest.toml` and bounded executable fixtures;
- `docs/curriculum.toml` reader intent;
- `docs/reviewed-requirements.toml` reviewed semantic fingerprints and explicit dispositions;
- ordinary MyST Markdown under `docs/sphinx/`;
- repository-local tool pins in `mise.toml`;
- the knowledge workflow at `.agents/skills/diskweave-knowledge/SKILL.md`.

Everything under `target/dwv-docs/` is reconstructible and ignored.

## Identity and semantic change

A requirement marker has the form:

```text
<!-- dwv:req req.<capability>.<stable-slug> -->
```

The ID is independent of work-item numbers, line numbers, archive paths, heading wording, and generated state. Extraction rejects missing, malformed, duplicate, or wrong-capability IDs.

Each requirement has two fingerprints. The formatting-stable local fingerprint covers its normalized normative prose and scenarios while excluding intrinsic/relationship markers and non-semantic Markdown formatting. The effective fingerprint additionally imports every sorted `requires`/`refines` target and that target's effective fingerprint after cycle validation, so owner changes invalidate the full dependent closure without changing unrelated requirements.

Sphinx receives a deterministic collision-checked ID (`R_` plus a bounded BLAKE3 prefix). Ignored interchange records both identities, making the mapping reversible through the exported table without putting opaque IDs into OpenSpecs or Rust.

## Relationship spine

Relationships are sparse and typed:

- a current requirement **requires** an independently owned semantic prerequisite;
- a current requirement **refines** an owner's policy through a narrower specialization or adapter realization;
- OpenSpec requirements **define** intended behavior;
- Rust module/type/operation/test **implements** or **exercises** it;
- scenario fixtures **exercise** bounded transitions or fault models;
- evidence artifacts **verify** only their declared claims and tiers;
- human prose **explains** it;
- non-claims **bound** what evidence and prose do not establish.

Sphinx-Needs renders generated engineering objects, tables, filters, and backlinks. Sphinx-CodeLinks owns bounded Rust discovery and scope association. Rustdoc owns API extraction. Small DiskWeave adapters only validate semantic IDs, normalize fingerprints, map identities, import artifact metadata, select bounded context, normalize revision-provider change boundaries, and coordinate readiness.

## Reviewed and suspect lifecycle

`docs/reviewed-requirements.toml` stores the last reviewed local and effective fingerprints for each current requirement. A local semantic edit makes that requirement suspect; a changed prerequisite or relationship makes every affected dependent report `semantic-prerequisite-changed`. A new requirement is uncovered. A retired requirement and all of its reviewed state must be removed from current inputs.

Allowed resolutions are `reviewed`, `reference-only`, `deferred`, and `superseded`. Resolution is atomic, records an individual reason, updates only the named current semantic unit, and leaves repeated no-op resolution byte-stable. Formatting-only changes do not trigger review.

The single readiness gate fails closed on:

- uncovered, local-fingerprint-suspect, or semantic-prerequisite-changed requirements;
- orphaned reviewed state;
- unknown or historical references;
- semantic/Sphinx identity collisions;
- out-of-root or symlinked source discovery;
- broken Sphinx links or warnings;
- missing evidence explicitly required by a changed claim.

A renderer success alone is not readiness.

At the change boundary, the same command reports canonical local, relationship, and dependency-closure impact plus removed or reassigned implementation/evidence links from an available repository revision baseline. Implementation-link additions and edits with unchanged semantic relationships remain context only. Baseline absence is explicit; no persistent relationship-diff registry is maintained.

## Human and agent projections

The Human Guide is causal and event-first: interrupted write, recovery uncertainty, parity versus integrity authority, then product/evidence boundary. Scenario pages render actual fixture facts and bounded outcomes. Assurance views preserve evidence tier, fault model, scope, and non-claims. Contributor and reference views derive package/source facts from Cargo metadata, CodeLinks, rustdoc, and current knowledge objects.

Agents begin with `.agents/skills/diskweave-knowledge/SKILL.md`, inspect exact semantic owners, request bounded graph-selected context when needed, and run `docs check` once at completion. The skill contains no revision-provider commands. Diagnostic `affected` and `knowledge doctor` commands are used only when the completion report identifies impact or an ID was lost. Payload bytes, credentials, private paths, runtime handles, and unbounded source text are excluded from interchange and context by default.

Usefulness is sampled separately from deterministic readiness when a major documentation architecture change warrants it. A reader exercises the Guide and a fresh agent performs a real task from graph-selected context; a human reviewer records concrete observations in the change completion evidence. This is not a permanent corpus, scoring system, or evaluator command.

## Toolchain and reconstruction

Sphinx, MyST, Sphinx-Needs, Sphinx-CodeLinks, sphinx-autobuild, and uv are pinned in repository-local mise configuration. Python packages remain development-only; production storage crates have no Python or Sphinx dependency. Builds run offline with warnings as errors and link checking enabled.

A clean-room check copies permanent inputs into a temporary repository without `target/`, rendered output, handoffs, archived architecture, model/provider state, or global Python packages. It must recreate equivalent knowledge objects, reviewed status, generated views, rustdoc links, and Sphinx HTML using only the pinned open toolchain. Generated inventories, page plans, block state, response queues, and model transcripts are not durable architecture.
