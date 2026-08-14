## 1. Standalone verifier implementation

- [ ] 1.1 Add the experimental `dwv.independent-parity.v1` descriptor model and validate profile, explicit protected geometry, selected range, region size/count, distinct payload identities, and all resource bounds before opening payload data; reject unknown versions/features, duplicate or aliased paths, unsupported profiles, and out-of-range input without writes.
- [ ] 1.2 Add the standalone equation-verifier executable beside the portable verification code with no dependency on `diskweave`, `dwv-service`, `dwv-frontend-ublk`, SQLite adapters, recovery-state inspection, or operator private state; read each selected data/parity region exactly, apply only declared logical zero extension beyond a data payload's protected length, and emit `matched`, `mismatched`, `incomplete`, or `unknown` region dispositions without identifying repair targets.
- [ ] 1.3 Render one bounded semantic result as equivalent human and structured output, including descriptor version, evidence tier, region findings, explicit non-claims, reason, and fixed process-status mapping for completed observation, invalid/unsupported input, and inability to produce a bounded report.

## 2. Focused evidence

- [ ] 2.1 Add disposable file-backed fixtures and executable evidence for matching equations, mismatches, short reads, failed reads, logical zero tails, invalid geometry, over-limit requests, duplicate/aliased payloads, and unchanged source/descriptor hashes after every run.
- [ ] 2.2 Prove human/structured equivalence, deterministic process statuses, bounded output/allocation behavior, experimental format scope, and the import/dependency boundary excluding production service, frontend, SQLite, recovery-state inspection, C8a/U26a, and U11 behavior.

## 3. Verification and canonicalization

- [ ] 3.1 Verify `add-bounded-parity-scrub` against its proposal, independent-parity-verification delta, design, and focused evidence; resolve every affected requirement review individually and do not broaden into C8a/U26a, U11, U26b export/migration/recovery-plan tooling, repair, or stable-format claims.
- [ ] 3.2 After verification passes, archive the change so `req.independent-parity-verification.standalone-equation-verification-is-bounded-read-only-and-non-authorizing` becomes current canonical authority while existing XOR, parity-verification, evidence, and recovery owners remain unchanged.
- [ ] 3.3 Link the implementation and focused evidence to the canonical requirement, update the temporary campaign's C8b/U26b split with exact current-versus-target posture and remaining open IDs, export the shared Beads viewer state if required, and close the canonicalization task only after all evidence and documentation gates pass.
