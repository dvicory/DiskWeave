## Why

DiskWeave has a detailed architecture handoff but no repository-local OpenSpec contract that turns its decisions, invariants, dependency order, and evidence rules into resumable implementation work. OS-000 must establish that contract before any Rust runtime or storage implementation is started.

## What Changes

- Establish the v0.6 architecture decisions, terminology, invariants, release gates, and decision authority in the OS-000 OpenSpec artifacts.
- Make the first dependency-ready OpenSpecs and their prerequisites explicit in the design and task artifacts.
- Define the portable-versus-platform-specific boundaries and fail-closed safety rules that later implementation OpenSpecs must preserve.
- Define the agent workflow for acceptance evidence, conservative stop/continue behavior, escalation, and handoff using the OpenSpec artifact sequence and CLI.
- Keep the source handoff read-only and make no runtime, data-member, or persistent-format changes in OS-000.

## Capabilities

### New Capabilities

- `architecture-contract`: A repository-local OpenSpec contract covering DiskWeave invariants, decision authority, dependency order, component seams, acceptance gates, and agent handoffs.

### Modified Capabilities

None. No existing OpenSpec capabilities are present.

## Impact

- Updates only the OS-000 proposal, capability spec, design, and task artifacts.
- Establishes the contract for later work such as OS-001 normalized block semantics, OS-002 store contracts, and OS-003 parity math.
- Uses the OpenSpec CLI for status, instructions, validation, and task readiness.
- Does not modify `docs/handoffs/diskweave-refined-architecture-v0.6.md`, Rust source, user data, data-member bytes, or any stable storage format.
