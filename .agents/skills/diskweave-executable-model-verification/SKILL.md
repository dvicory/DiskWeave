---
name: diskweave-executable-model-verification
description: Verify DiskWeave executable models and implementation conformance, especially for delegated canonical models. Match claims to evidence, use the strongest practical check for the declared scope, and treat mapping failures as semantic or architecture signals.
---

# DiskWeave executable model verification

Use this skill when adding or changing an executable model, changing semantics delegated to one, or claiming that an implementation conforms to one.

## Verify the model

Use the strongest practical checker for the model's declared finite scope.

Prefer exhaustive reachable-state exploration when the model and tool support it. Sampling, witnesses, deterministic replay, and mutation tests are complementary evidence, not substitutes for exhaustive checking.

If exhaustive verification is not practical, record why and state the bound or coverage achieved.

Keep verification bounds and analysis configuration outside canonical semantics.

Check that each important evidence claim is actually exercised by its predicate and bound. Safety invariants, reachability witnesses, scenario tests, sampled runs, bounded model checks, mutations, and conformance tests establish different facts.

For consequential paths, make sure the verification depth can reach the claimed behavior. Prefer positive path evidence over inferring execution from an intermediate state.

## Verify the implementation relationship

A model action does not need to map to one implementation call. A conformance bridge may drive several implementation operations and may compare only the implementation state represented by the model.

Implementation-conformance bridges are observational. Derive compared state and transition outcomes from implementation-observable state, results, effects, or evidence. Bridge-local bookkeeping may preserve model inputs, nondeterministic choices, correlation identity, and observations of implementation outcomes; it must not manufacture an implementation fact because the model requested the corresponding transition.

For each mapped transition, identify what observable implementation fact establishes it at the declared abstraction. Invoking a driver handler is not by itself evidence that the transition occurred.

Account for the complete declared conformance surface. Each delegated action, consequential outcome, and model-state distinction in the projection must be exercised against observable implementation behavior, explicitly excluded from the claim, or reported as an unresolved seam.

When practical, also check the reverse direction: implementation traces within the delegated surface should be accepted by the model.

## Treat mapping problems as findings

Do not abandon conformance merely because model and implementation granularity differ.

If a delegated semantic concept cannot be mapped cleanly, determine whether the problem is the model boundary, implementation seam, state projection, or ownership boundary.

Reconcile the boundary instead of creating a fragile bridge or silently reducing the evidence bar.

## Keep traces reconstructible

Prefer semantic actions, observations, correlation identity, and normalized state projections that can later be reconstructed from implementation-observable facts. This keeps model counterexamples and future production traces compatible with the same replay boundary.

Preserve useful bounded traces or reproduction metadata when they expose a correctness failure, but do not add production trace retention merely for model verification.

## Claims

Distinguish:

- exhaustive verification of a declared finite model;
- bounded model verification;
- complete abstract-transition conformance coverage;
- sampled model-based testing;
- mutation or negative evidence;
- and formal implementation correctness.

Do not describe one as another.
