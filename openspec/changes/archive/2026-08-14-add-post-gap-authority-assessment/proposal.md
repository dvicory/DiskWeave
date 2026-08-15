## Why

The current production assessment can treat recognized identity and topology as “current parity” or writable availability after a custody gap, although neither proves content continuity or a current protection basis. C0a makes that read-only assessment truthful before any restart, rollover, repair, or recovery work is allowed to rely on it.

## What Changes

- Extend the existing production assessment with separate lineage, custody-continuity, and range-local protection-basis dimensions.
- Define conservative `current`, `prior`, `unprotected`, `indeterminate`, and `not-yet-interpretable` basis results without inferring one dimension from another.
- Require bounded per-parity-role coverage and exceptions in both human and structured output.
- Preserve observation success when the array is blocked or uncertain, while explicitly stating that assessment authorizes no publication, mutation, repair, reconstruction, or historical-continuity claim.
- Replace current identity-derived parity and redundancy claims with evidence-derived classifications.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operator-recovery`: Evolve the existing observational assessment owner to classify post-gap authority and uncertainty without mutation or stronger authorization.

## Impact

- `src/operator.rs` and its renderers must add the new assessment dimensions and stop deriving current protection from member recognition alone.
- The versioned structured operator result gains required authority fields; strict consumers and CLI fixtures must be updated with the implementation.
- Focused operator tests must cover recognized complete arrays after unproved custody, missing or unsupported recovery evidence, range-local mixed basis, bounded aggregation, and human/structured parity.
- Recovery-state mutation, protection-epoch creation, first-write currentization, background rollover, publication, repair, and historical recovery remain out of scope.
