## 1. Recovery observation and reconciliation

- [x] 1.1 Add bounded non-mutating recovery inspection and classification to the semantic and SQLite adapter boundaries
- [x] 1.2 Add exact prior/proposed/neither reopen reconciliation for uncertain commit observations

## 2. Persisted checksum baseline and admission

- [x] 2.1 Persist the active baseline descriptor and derive conservative required, partial, complete, and invalid states
- [x] 2.2 Rehydrate checksum authority on service open and block mandatory-baseline read/write admission until complete
- [x] 2.3 Add resumable per-extent baseline continuation using current identity, claims, hashes, fences, and recovery commits

## 3. Operator workflow

- [x] 3.1 Replace hand-written parsing with `clap`, add bounded declarative array-policy loading and the shared production/demo command-result-rendering boundary, and keep fixture manifests outside the production format
- [x] 3.2 Implement production status and members with one multidimensional semantic result and human/JSON rendering
- [x] 3.3 Implement production exhaustive scrub and exact deterministic damage reporting through `dwv-verify`
- [x] 3.4 Implement deterministic read-only recovery preview and stale-plan-safe apply with current exhaustive revalidation
- [x] 3.5 Implement production baseline and start commands with distinct outcomes and actual publication reporting

## 4. Acceptance and evidence

- [x] 4.1 Add focused semantic, SQLite reopen/failure, service admission, and production CLI regression tests
- [x] 4.2 Prove the integrated recovery-to-baseline path, refusal paths, zero payload writes, and unsupported publication behavior
- [x] 4.3 Update requirement links and verification evidence, validate OpenSpec, and run repository knowledge, workspace, and documentation gates
