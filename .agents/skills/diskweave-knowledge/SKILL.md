---
name: diskweave-knowledge
description: Inspect DiskWeave requirement ownership, evidence, explanations, and readiness before changing correctness-sensitive behavior or documentation.
---

# DiskWeave knowledge workflow

Use this skill for product code, canonical OpenSpecs, verification evidence, or maintained documentation.

## Before editing

1. Name the canonical semantic requirement IDs involved. If unknown, search current `openspec/specs/*/spec.md`; do not infer authority from handoffs or archived changes.
2. Run `cargo xtask docs knowledge readiness`.
3. Run `cargo xtask docs knowledge inspect <requirement-id>` for exact ownership and fingerprint data.
4. Run `cargo xtask docs knowledge context <requirement-id>` for the bounded current relationship packet.
5. Treat `docs/handoffs/**`, archived changes, and generated `target/**` content as non-authoritative unless the task explicitly requests history.

## Editing rules

- OpenSpecs own intended behavior. Rust markers, evidence, and prose link to requirements; they do not redefine them.
- Put `/// dwv:req <requirement-id>` only on a Rust item that genuinely owns or represents that semantic boundary. Do not tag every helper.
- Keep user-facing prose causal and audience-specific. Preserve requirement modality and explicit non-claims.
- For a Human Guide chapter, read its `docs/curriculum.toml` entry first. Draft or review the `[[entries.sections]]` briefs in order: use the proposed heading, satisfy every `must_answer` from the linked current sources, and preserve every section `non_claims`. The briefs guide Markdown; they do not override canonical requirements.
- Do not place payload bytes, credentials, private paths, runtime handles, or unbounded source text in markers, evidence, context, or generated objects.
- A changed requirement fingerprint makes linked explanations suspect. Re-review the affected fragment; never bulk-accept unrelated prose.
- `docs check` performs deterministic change-boundary impact discovery when a repository revision baseline is available. Follow only its `review_required` actions; unchanged implementation relationships are context, not an automatic documentation task.
- Use `knowledge affected` to inspect a reported requirement or path and `knowledge doctor --path` only to recover a relationship lost from the current file. Neither command infers whether prose is semantically correct.

## Before completion

1. Run `cargo xtask docs check` once at the change boundary and follow its exact failed gates or review-required actions.
2. Run the focused product check for the changed contract.
3. Run `cargo xtask docs build` when requirement links, evidence, or maintained prose changed.
4. Report unresolved unknown IDs, uncovered requirements, suspect links, broken references, and unsupported claim boundaries. A green renderer alone is not readiness.
