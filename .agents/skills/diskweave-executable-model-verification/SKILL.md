---
name: diskweave-executable-model-verification
description: Evaluate verification and implementation conformance for DiskWeave executable models, especially delegated canonical models. Use the strongest practical evidence for the declared model scope and treat model/implementation mapping failures as semantic or architecture signals rather than automatically weakening verification.
---

# DiskWeave executable model verification

Use this skill when adding or changing an executable model, changing semantics delegated to one, or claiming that an implementation conforms to one.

## Verify the model

Use the strongest practical checker for the model's declared finite scope.

Prefer exhaustive reachable-state exploration when the model and tool support it. Sampling, witnesses, deterministic replay, and mutation tests are complementary evidence, not substitutes for exhaustive checking.

If exhaustive verification is not practical, record why and state the exact bound or coverage achieved.

Keep verification bounds and analysis configuration outside canonical semantics.

## Verify the implementation relationship

For a model that describes implemented behavior, account for every delegated action, consequential outcome, and transition class.

A model action does not need to map to one implementation call. A conformance bridge may drive several implementation operations and may compare only the implementation state represented by the model.

Aim to exercise every reachable delegated action and consequential outcome against the implementation for the declared finite abstraction. Use longer randomized traces as additional history-sensitive evidence.

When practical, also check the reverse direction: implementation traces within the delegated surface should be accepted by the model.

## Treat mapping problems as findings

Do not abandon conformance merely because model and implementation granularity differ.

If a delegated semantic concept cannot be mapped cleanly to the implementation, determine whether:

- the model is too abstract or owns too much;
- the implementation lacks the semantic seam the model requires;
- the state projection is insufficient;
- or the behavior properly belongs outside the model.

Reconcile the boundary instead of creating a fragile bridge or silently reducing the evidence bar.

## Claims

Distinguish:

- exhaustive verification of a declared finite model;
- bounded model verification;
- complete abstract-transition conformance coverage;
- sampled model-based testing;
- mutation or negative evidence;
- and formal implementation correctness.

Do not describe one as another.
