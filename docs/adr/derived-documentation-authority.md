# ADR: Derived documentation is a controlled projection

**Status:** Superseded by [Open traceable knowledge tooling](open-traceable-knowledge.md)
**Date:** 2026-08-08
**Scope:** Documentation and semantic-context tooling only
**Supersession:** The open-traceable-knowledge ADR preserves the authority boundary and formatting-stable semantic review, but replaces the custom curriculum/block/provenance CMS and model-response lifecycle with sparse OpenSpec identities, reviewed fingerprints, ordinary MyST, and maintained Sphinx integrations.


## Context

DiskWeave already has accepted product architecture, canonical OpenSpecs, executable scenarios, and evidence records. Explanatory documentation must remain useful without becoming a competing storage specification or being rewritten whenever a model changes.

## Decisions

1. Derived Guide, scenario, contributor, assurance, and self-documentation pages are non-normative projections. Product architecture and canonical OpenSpecs remain authoritative.
2. Source discovery is broader than human documentation. A version-controlled concept/page/block curriculum selects what is taught, with explicit audiences, goals, prerequisites, source selectors, claims, and omission reasons.
3. Stable source-unit and block IDs use normalized semantic digests for impact tracking. Formatting-only changes do not dirty prose; semantic deltas move affected blocks to assessment.
4. Maintenance is preservation-first: `KEEP` updates attestation without changing accepted bytes, while `PATCH`, `REPLACE`, and `BLOCKED` require structured decisions and bounded validation.
5. Provider, model, sampling, and style changes do not dirty accepted prose. They are audit metadata, not freshness inputs.
6. Deterministic checks and CI never require model credentials or network access and never regenerate prose. Generation/assessment may occur through a configured provider or external-agent task protocol.

## Consequences

- A clean block is not sent to a model.
- A new correctness-sensitive source must be classified or the coverage gate fails.
- Explanatory wording can be preserved byte-for-byte across model migrations.
- The system must retain source snapshots, intent, claims, goals, and accepted-text hashes.

## Reversal evidence

Revisit only if measured maintenance evidence shows semantic digests cannot distinguish relevant changes, preservation causes systematic teaching defects, or offline validation cannot provide a trustworthy gate. Any reversal requires a new ADR and tests demonstrating the replacement boundary.
