## Why

Current requirements repeat detailed mutation, fence, abandonment, request-routing, and recovery policies across capability boundaries, and several still import authority from retired planning identifiers. This leaves implementations able to satisfy local wording while disagreeing on the actual owner and prevents deterministic re-review when an owner changes.

## What Changes

- Consolidate each affected detailed operational policy under one current requirement while preserving narrower recovery-store, transaction, checksum, service-composition, and adapter refinements.
- Remove normative dependencies on historical work IDs, handoff sections, roadmap properties, gates, and phases from current specifications.
- Clarify stable-slot request admission so collection order cannot select a logical member.
- Retire the completed Linux correction-program requirement while retaining and remapping its evidence to durable requirements.
- Separate the durable Linux acceptance contract from the exact retained evidence environment without broadening the accepted profile.
- Add colocated `requires` and `refines` markers, deterministic graph validation and ownership views, and dependency-closure semantic review invalidation.
- **BREAKING**: Retire one current requirement ID and bump knowledge/review schemas for the new relationship and fingerprint contract.

Non-goals: merge capability files, choose a production recovery adapter or transaction engine, broaden Linux claims, infer semantic equivalence automatically, create an ownership registry, or change metadata-certificate authorization or service request APIs in this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dirty-integrity-invalidation`: Retain complete durable-intent, dirty-clear, and restart-state ownership while exposing explicit refinements.
- `recovery-state-semantics`: Narrow recovery-store requirements to atomic persistence and typed recovery authority.
- `explicit-transaction-machine`: Narrow requirements to action emission and transaction-state transitions.
- `checksum-plane`: Narrow checksum invalidation to the local validity transition and remove historical authority.
- `healthy-portable-io`: Make service requirements composition-only and clarify stable-slot admission.
- `normalized-block-semantics`: Retain request and frontend-lifecycle ownership without historical gates.
- `volatile-media-simulator`: Remove historical work identifiers while preserving the media fault model.
- `macos-bridge-feasibility`: Remove roadmap-number authority from the accepted adapter boundary.
- `linux-ublk-frontend`: Retire the completed correction program and separate durable acceptance from its evidence environment.
- `documentation-knowledge-architecture`: Add canonical forward relationships, derived ownership views, and transitive effective-semantic review semantics.
- `checksum-scrub-verified-repair`: Expose independent-evidence repair authority relationships.
- `degraded-read-offline-rebuild`: Expose topology, recovery, and reconstruction prerequisites.
