## Context

The current `xtask` loads a large set of JSON registries from `docs/model`, extracts only explicitly registered source units, stores all inventories/tasks/provenance under `docs/state`, and emits scenario/assurance/contributor/reference indexes that are not the primary human pages. Existing accepted blocks have block-level hashes but no fragment preimages or claim-level requirement links. Canonical OpenSpecs use stable `Requirement` headings but no separate identity registry. The portable simulator and normalized-trace crates already provide deterministic facts that the projections can consume.

## Goals / Non-Goals

**Goals:**

- Make current canonical OpenSpecs discoverable and requirement-addressable without copying their text into a second registry.
- Keep only small audience/pedagogy intent and evidence manifests as durable documentation inputs.
- Put reconstructible state under an ignored workspace root while keeping accepted Markdown and evidence reviewable.
- Render facts and relationships for scenario, assurance, contributor, reference, and agent consumers.
- Prove causal teaching, exact provenance, no-op preservation, narrow PATCH, context relevance, reconstruction, and historical isolation.

**Non-Goals:**

- Changing DiskWeave storage/runtime semantics or the production `dwv` command.
- Adding a provider, model SDK, network access, embeddings, vector search, or RAG service.
- Deleting v0.6-v0.8 handoffs; they remain checked-in historical inputs for explicit audits.
- Requiring every explanatory sentence to be machine-generated; accepted human prose remains valid when provenance and preservation checks pass.

## Decisions

1. **One intent-only curriculum.** The permanent human-maintained file is `docs/curriculum.toml`, using a chapter/entry model rather than architecture-summary blocks. It contains only audience, concrete reader question, starting knowledge, prerequisites, misconceptions, concepts introduced/deferred, canonical requirement IDs, executable scenario IDs, teaching devices, and projection paths/options. It contains no free-text claims, source paths, package facts, evidence prose, bounds, execution policy, migration state, generated plans, or block registries. `deny_unknown_fields` validation keeps it from silently growing a second authority.
2. **Canonical relationships are IDs, not copied semantics.** Requirement IDs are extracted from current OpenSpecs. Scenario IDs resolve to checked-in executable fixtures. Verification evidence manifests own evidence tier, fault model, scope, non-claims, and the canonical requirement IDs they exercise. Assurance is an inversion of that graph plus only irreducible, endpoint-validated support judgments; it is not a second claim registry. Usefulness cases name an audience, projection, reader question/task, canonical rubric requirement IDs, and misconceptions/overclaims to detect; generated answers and grades are evidence under `target/dwv-docs` or `docs/verification`.

3. **Intrinsic requirement IDs.** A canonical requirement is identified as `req.<capability>.<stable-slug>`, where the capability directory is a stable semantic namespace and the slug is derived from the normalized `Requirement:` title. The implementation rejects duplicate IDs, records the original heading as display text, and keeps the ID independent of line positions, work numbers, archive paths, and formatting. A future title change is an explicit semantic identity change rather than a silent reassignment.
4. **Amendment and supersession, not replacement.** The pre-normalization `architecture-contract` and v0.8 handoff are migration inputs. The section-level audit records every old normative unit and its disposition before the final `openspec/specs/architecture-contract/spec.md` is reduced to timeless product boundaries. Handoff artifact rules, work sequencing, agent workflow, and documentation mechanics are historical/tooling owners; topology, geometry, transaction, integrity, repair, frontend, security, and evidence details remain in their named capability specs or verification records.
5. **Ignored workspace state.** Inventories, plans, coverage, generated indexes, tasks, responses, diagnostics, scenario fact caches, and block state live beneath ignored `target/dwv-docs/`. Accepted Markdown, the curriculum, prompt contracts, schemas, canonical sources, and verification evidence remain reviewable under `docs/`.

6. **Facts before prose.** Scenario renderers consume executable fixtures and normalized traces. Assurance inverts verification-owned evidence support edges into requirement-to-evidence views. Contributor and reference projections consume Cargo metadata and the canonical source graph directly. Human Guide chapters retain accepted narrative but expose compact generated provenance.

7. **Fragment-preserving application.** Accepted chapters may contain stable fragment markers. `PATCH` responses carry fragment IDs and preimage hashes; `KEEP` never rewrites prose; `REPLACE` requires an explicit reason. Provider/model/prompt metadata cannot stale accepted prose by itself.

8. **Lifecycle commands.** Keep `cargo xtask docs` model/network-free. Add deterministic extraction, planning, projection, evaluation, and reconstruction checks; retain bootstrap/retirement only as explicit historical audit operations. `check` validates coverage, provenance, executable artifacts, safe rendering, privacy, and preservation, while qualitative usefulness remains generated review evidence rather than proof.

9. **Migration evidence.** Generate the v0.8 normalization report from its headings and explicit dispositions. It is verification evidence, not a permanent source registry. Any `MISSING` current unit gets the smallest canonical owner before historical demotion.

## Risks / Trade-offs

- Deriving IDs from requirement titles means a deliberate title rename requires an explicit migration; this is preferable to a hidden second registry and is detectable by coverage. Reordering and formatting remain stable.
- Keeping accepted Markdown under `docs/book/src` means pages are durable projections rather than fully reproducible text. Their block provenance and preservation state make intentional prose changes auditable; deterministic fact sections are regenerated from fixtures.
- Automatic contributor facts cannot infer every semantic ownership nuance. The projection reports Cargo/source facts as authoritative and marks missing optional navigational bindings rather than inventing them.
- Moving state to `target` makes a clean checkout require one bootstrap/extract pass. The `reconstruct` command and README provide the explicit path and test it in a temporary copy.
