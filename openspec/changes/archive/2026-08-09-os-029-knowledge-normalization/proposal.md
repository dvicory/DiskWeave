## Why

DiskWeave's current documentation tooling is deterministic and bounded, but it still depends on a second semantic registry under `docs/model`, treats a hand-authored architecture document as current input, emits mostly descriptive projection pages, and cannot prove fine-grained preservation or task-specific context quality. Goal v5 requires one current semantic truth: canonical OpenSpecs plus a small cross-cutting constitution, with human, evidence, contributor, reference, and agent views derived from that truth and with historical architecture isolated by default.

## What Changes

- **BREAKING** Replace the hand-maintained source/requirements/assurance/scenario/contributor/architecture registries with automatic canonical OpenSpec discovery plus one small intent-only `docs/curriculum.toml`. Evidence manifests and executable scenario facts remain verification-owned; Cargo metadata is derived directly.
- **BREAKING** Move reconstructible inventories, plans, coverage, generated indexes, task/response queues, diagnostics, scenario facts, qualitative answers/results, and block state to `target/dwv-docs`; keep only durable human documentation/configuration, prompt contracts, schemas, canonical sources, and inspectable verification evidence under `docs/`.
- Add intrinsic stable requirement identities for canonical `openspec/specs/*/spec.md` requirements and use those identities for provenance, freshness, context selection, and coverage.
- Audit every normative or correctness-sensitive unit in architecture v0.8, record a disposition and unresolved `MISSING` owners, then isolate v0.6/v0.7/v0.8 handoffs from normal extraction and context.
- Replace the large current architecture projection with a generated reference over canonical requirements and the small architecture constitution; retain v0.6-v0.8 as opt-in historical material.
- Render executable scenario facts, assurance claim/mechanism/evidence/non-claim chains, and Cargo-derived contributor ownership directly instead of rendering descriptions of those projections.
- Add and prove a causal interrupted-write chapter whose timeline facts come from executable simulator/trace artifacts and whose claims expose compact requirement/scenario/evidence provenance.
- Implement real preservation-first semantics: `KEEP` is byte-stable, `PATCH` changes only declared stable fragments with preimage checks, `REPLACE` is explicit, and model/provider/prompt metadata changes do not stale accepted prose.
- Select agent context by canonical requirement graph closure, implementation ownership, scenarios, evidence, and forbidden outcomes rather than concatenating whole source documents; record a reproducible baseline comparison.
- Add deterministic integrity, no-op, new-coverage, reconstruction, historical-isolation, and projection checks plus a small qualitative usefulness corpus. Keep `cargo xtask docs`, model/network-free checks, bounded source/privacy rules, atomic writes, and production `dwv` separation.
- Supersede the documentation-system decisions in OS-025/OS-027 and their derived ADRs where they encode permanent duplicate registries or handoff/current-source coupling; preserve their useful safety contracts and history.

## Capabilities

### New Capabilities

- `documentation-knowledge-architecture`: Canonical-spec discovery, stable requirement identities, simplified durable configuration/state, audience-specific projections, claim-level provenance, preservation-first maintenance, task-specific context, usefulness evaluation, reconstruction, and historical-source isolation.

### Modified Capabilities

- `architecture-contract`: clarify that the architecture constitution and canonical capability OpenSpecs are current authority while handoffs and work identifiers are historical/organizational inputs only.
