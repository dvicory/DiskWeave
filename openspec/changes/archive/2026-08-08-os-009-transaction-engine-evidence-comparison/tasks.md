## 1. Candidate crate and semantic adapter

- [x] 1.1 Add `dwv-transaction-proc` with pinned `procmachines` dependency and isolated dependency direction.
- [x] 1.2 Define the candidate IO exchanges, terminal outcomes, normalized events, and comparison metric types without exposing procedural internals.
- [x] 1.3 Implement one procedural semantic task that emits the existing action vocabulary and consumes one result per action.

## 2. Reference parity driver

- [x] 2.1 Build one representative dirty-region parity-write plan shared by both engines.
- [x] 2.2 Implement a deterministic driver that feeds identical results to the reference and procedural machines.
- [x] 2.3 Normalize action/result traces, stage transitions, terminal dispositions, and allowed batching differences.

## 3. Fault and ownership evidence

- [x] 3.1 Add success, EIO, short, uncertain, delayed, out-of-order, duplicate, and stale-generation schedules.
- [x] 3.2 Add abandonment schedules before and after durable intent/home mutation and verify operation ownership remains unresolved until terminal.
- [x] 3.3 Add daemon-crash and power-loss schedules after every modeled persistence transition.
- [x] 3.4 Compare dirty-clearing proof inputs and reject any candidate trace that reaches clean without equivalent evidence.

## 4. Measurements and decision record

- [x] 4.1 Measure deterministic trace counts, elapsed medians, structural sizes, synchronization operations, and dependency metadata for both engines.
- [x] 4.2 Record correctness results, fault coverage, cost evidence, limitations, fallback, and exit path in an ADR.
- [x] 4.3 Record executable commands, representative outputs, and portable claim boundaries in verification documentation.

## 5. Verification and completion

- [x] 5.1 Run candidate, comparison, fault-schedule, workspace, lint, dependency, and strict OpenSpec checks.
- [x] 5.2 Verify every OS-009 scenario against the implementation and reject or retain the candidate according to the mandatory safety gates.
- [x] 5.3 Archive the completed change only after artifacts, implementation, evidence, and decision record agree.
