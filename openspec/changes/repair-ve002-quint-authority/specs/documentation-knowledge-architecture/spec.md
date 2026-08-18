## MODIFIED Requirements

### Requirement: Change-boundary impact is deterministic and bounded
<!-- dwv:req req.documentation-knowledge-architecture.change-boundary-impact-is-deterministic-and-bounded -->
<!-- dwv:requires req.documentation-knowledge-architecture.canonical-semantic-relationships-are-colocated-and-derived -->

At a change boundary, deterministic readiness SHALL compare current local and effective semantic/review fingerprints and current typed relationships with an available repository revision baseline through a provider-neutral interface. Local semantic changes, dependency-closure semantic-prerequisite changes, removed or reassigned relationships, and retired IDs SHALL identify the exact requirements requiring individual review. A changed current delegated canonical source identified by its source-local requirement marker SHALL also be treated as a semantic source change even when its marker set and extracted requirement IDs are unchanged; the marked owner and its transitive requirement-dependent closure SHALL require review. Such a path SHALL be classified as a delegated canonical source change rather than as an implementation-only or evidence-only edit. Formatting-only changes and implementation edits with unchanged relationships remain context rather than automatically creating documentation work. The command SHALL report when no baseline is available instead of claiming that relationships are unchanged, and diagnostics SHALL provide exact bounded inspection commands.

#### Scenario: A semantic prerequisite changes

- **WHEN** a target's effective semantic/review fingerprint changes or a source relationship is removed or reassigned
- **THEN** `docs check` reports every exact direct and transitive review-required dependent ID with `semantic-prerequisite-changed` diagnostics while unrelated requirements remain unchanged

#### Scenario: A delegated canonical source changes

- **WHEN** a current model or other executable semantic source carrying a current requirement marker changes while its marker set remains present
- **THEN** affected-path and change-impact classify the path as `delegated_canonical_source_changed`, require review of every marked current owner and its transitive dependent closure, and do not classify the edit as implementation-only evidence

#### Scenario: A delegated source has no current owner marker

- **WHEN** a model path changes without a current requirement marker or its prior marker is removed
- **THEN** the change remains bounded context or an explicit removed-reference diagnostic and does not invent a semantic owner from filename, implementation calls, or historical artifacts

#### Scenario: The revision baseline is unavailable

- **WHEN** no supported repository revision baseline can be read
- **THEN** semantic readiness still runs and relationship-delta status is reported as unavailable rather than unchanged
