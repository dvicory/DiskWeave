---
name: diskweave-knowledge
description: Inspect DiskWeave requirement ownership, evidence, explanations, and readiness before changing correctness-sensitive behavior or documentation.
---

# DiskWeave knowledge workflow
<!-- dwv:req req.documentation-knowledge-architecture.agent-knowledge-workflows-use-one-complete-retrieval-path -->

Use this skill for product code, canonical OpenSpecs, verification evidence, or maintained documentation.

## Before editing

1. Name the canonical semantic requirement IDs involved. If unknown, search current `openspec/specs/*/spec.md`; do not infer authority from handoffs or archived changes.
2. Run `cargo xtask docs knowledge readiness` once at the applicable change boundary.
3. For normal correctness work, run one `cargo xtask docs knowledge context <requirement-id>...` command with every initially relevant current requirement ID.
4. A successful context packet contains complete canonical units for every selected requirement, transitive prerequisite, and direct dependent in its reading order. Do not reread canonical specs merely to reconstruct semantic context already present in the packet or add `inspect`/`ownership` retrieval for the same need. Open the located canonical source normally when the task requires editing or source-level inspection.
5. If a bound failure returns `suggested_requests`, run every suggested request; the set is complete only as a whole. No suggestions means a required semantic scope cannot be split without omission.
6. Use `knowledge inspect <requirement-id>` alone for an exact local-unit lookup, `knowledge ownership <requirement-id>` alone for direct ownership and endpoint troubleshooting, `knowledge affected <requirement-id>` after an owner or relationship changes, and `knowledge audit-context <requirement-id>` only for a deliberate whole-component audit.
7. Never request overlapping `inspect`, `ownership`, and `context` packets for the same need, and never choose packet fields, graph depth, relationship direction, or closure policy.
8. Treat the marked active architecture roadmap, `docs/architecture/archive/**`, archived changes, and generated `target/**` content as non-authoritative unless the task explicitly requests roadmap planning or history.

## Editing rules

- OpenSpecs own intended behavior. Rust markers, evidence, and prose link to requirements; they do not redefine them.
- Put `/// dwv:req <requirement-id>` only on a Rust item that genuinely owns or represents that semantic boundary. Do not tag every helper.
- Keep user-facing prose causal and audience-specific. Preserve requirement modality and explicit non-claims.
- For a Human Guide chapter, read its `docs/curriculum.toml` entry first. Draft or review the `[[entries.sections]]` briefs in order: use the proposed heading, satisfy every `must_answer` from the linked current sources, and preserve every section `non_claims`. The briefs guide Markdown; they do not override canonical requirements.
- Do not place payload bytes, credentials, private paths, runtime handles, or unbounded source text in markers, evidence, context, or generated objects.
- A changed local fingerprint makes that requirement suspect. A changed effective fingerprint with stable local prose means an owner/refiner prerequisite changed; re-review every reported dependent individually and record a concrete reason. Never bulk-accept a dependency closure.
- `docs check` performs deterministic change-boundary impact discovery when a repository revision baseline is available. Follow only its `review_required` actions; unchanged implementation relationships are context, not an automatic documentation task.
- Each knowledge command has one complete purpose: `context` supplies bounded normal task context, `inspect` supplies one exact canonical unit, `ownership` supplies direct ownership and endpoint facts, `affected` supplies downstream change impact, and `audit-context` supplies the complete connected component for an explicit audit. `knowledge doctor --path` only recovers an implementation/evidence relationship lost from the current file. None issues a semantic-consistency verdict.
- Use `diskweave-semantic-reconciliation` when inspection reveals overlapping canonical policy ownership, contradictory scenarios, an unclear semantic owner, changed owner semantics with dependents, historical-authority leakage affecting interpretation, or consequential shadow architecture.

## Before completion

1. Run `cargo xtask docs check` once at the change boundary and follow its exact failed gates or review-required actions.
2. Run the focused product check for the changed contract.
3. Run `cargo xtask docs build` when requirement links, evidence, or maintained prose changed.
4. Report unresolved unknown IDs, uncovered requirements, suspect links, broken references, and unsupported claim boundaries. A green renderer alone is not readiness.
