## 1. Artifact and dependency contract

- [x] 1.1 Cite handoff Phase 1 OS-013, D-001–D-003/D-007–D-010/D-014/D-017/D-018/D-021, Gate D, and portable-demo limits.
- [x] 1.2 Define all twenty design sections, ordinary data-member invariant, and OS-020 boundary.
- [x] 1.3 Define end-to-end acceptance, failure mapping, and resource obligations.

## 2. Workspace and orchestration modules

- [x] 2.1 Register and organize `dwv-store-file` and `dwv-transaction-ref` with explicit module boundaries in the workspace.
- [x] 2.2 Add request router/service lifecycle and typed operation-slot integration.
- [x] 2.3 Add range splitting, deterministic lock/resource admission, and bounded buffers.
- [x] 2.4 Compose topology, stores, OS-008 transaction actions, OS-010 intent, and OS-011 checksum evidence.

## 3. Healthy read/write path

- [x] 3.1 Implement healthy data reads with exact range/completion evidence.
- [x] 3.2 Implement single-XOR RMW/full-overwrite writes and parity updates.
- [x] 3.3 Implement flush/FUA/fence handling and clean checkpoint gating.
- [x] 3.4 Implement abandonment, restart/recovery, lock conflict, alias, and capability-failure behavior.

## 4. Verification and macOS evidence

- [x] 4.1 Add randomized reference-image tests, action traces, child-completion permutations, and resource-bound tests.
- [x] 4.2 Add macOS temporary-file integration tests, clean reopen, control-state rebuild, and independent data-file readability.
- [x] 4.3 Run workspace tests, format, dependency/license inspection, and OpenSpec validation; record portable-demo limitations.
- [x] 4.4 Mark complete after the workspace acceptance, restart/abandonment cut-point matrices, dependency inspection, and OpenSpec validation pass; leave OS-014–017 and OS-020 as separate changes.
