# ADR: Bootstrap inputs are retired, not promoted

**Status:** Accepted
**Date:** 2026-08-08
**Scope:** Documentation-system lifecycle and repository maintenance

## Context

A temporary bootstrap document can accelerate architecture capture, but keeping it in the active source graph would create a second hidden authority and make recovery depend on historical conversation context.

## Decisions

1. Bootstrap documents are readable only by an explicitly bounded audit/retirement operation. Ordinary extraction, context, prompts, provenance, generated pages, tests, and CI exclude them.
2. Every enduring directive is distributed to the correct durable home: architecture, ADR, canonical OpenSpec, schema/configuration, CLI/help, agent rules, work index, or executable test.
3. A machine-readable disposition inventory and a fresh-context review are mandatory before deletion. Every non-illustrative unit must map to durable artifact IDs and, where testable, verification commands or IDs.
4. Retirement requires a clean-room drill with the bootstrap input absent, generated output initially absent, no hidden environment state, provenance-loss failure-closed behavior, formatting-only stability, semantic stale detection, coverage failure for new critical sources, and full offline gates.
5. After the gate, retain only a compact non-operational retirement record containing the retired path/digest, replacement artifact IDs, drill evidence, disposition counts, and confirmation that the retired input is not operational.

## Consequences

- A self-documentation page cannot be the only durable home for an important rule.
- Deleting the bootstrap file is an acceptance test, not cleanup.
- Future agents must find operation, recovery, and next work from the repository itself.

## Reversal evidence

Do not restore a retired bootstrap input to current-source status. If a missing durable rule is discovered, amend the accepted architecture, ADR, OpenSpec, schema, agent rule, or work index directly and add a regression test. Historical version-control recovery is sufficient for the retired source.
