## 1. Reconcile the file-backed owner target

- [ ] 1.1 Review the added post-acquisition release requirement against the canonical single-writer/partial-acquisition rule and confirm that the existing definite partial cleanup semantics are unchanged.
- [ ] 1.2 Review exact claim/incarnation/resource/alias binding, per-claim `released`/`held`/`uncertain` meaning, aggregate `released-all`/`residual`/`uncertain` derivation, and process-loss behavior against affected store-operation, recovery, and startup owners.
- [ ] 1.3 Confirm that release owns no startup abandonment policy, writable-recovery disposition, session/epoch rollback, recovery `CLEAN`, checksum, transaction, shutdown, or frontend policy.

## 2. Reconcile dependent startup wording

- [ ] 2.1 Replace the active scan-independent startup target's unresolved full-set/later-gate file-backed prerequisite with consumption of this exact owner observation, while keeping startup as the owner of when abandonment requests release.
- [ ] 2.2 Preserve conservative outcomes: `released-all` permits only fresh later acquisition; `residual` or `uncertain` blocks same-process inferred retry/reacquisition; process loss uses the existing crash-release rule; durable session/epoch and other owner state remain unchanged by file-backed release.
- [ ] 2.3 Keep known partial acquisition under its existing definite release-all rule and keep writable-recovery authority under its separate owner.

## 3. Validate and review

- [ ] 3.1 Run strict OpenSpec validation for this change and the affected active startup target; repair only defects within this semantic scope.
- [ ] 3.2 Obtain affected-owner semantic review before downstream implementation or canonical synchronization.
- [ ] 3.3 Leave production implementation, evidence, canonical synchronization, and archival to their normal downstream work; this change is ready when the proposed semantics and dependent references are reviewed and valid.
