# Milestone 8 semantic-reconciliation evaluation preservation

This directory preserves evidence from the DiskWeave Milestone 8 semantic-ownership/reconciliation work.

## Important evaluation rule

Do NOT evaluate an agent from this directory's tip revision.

The reports, oracle material, reviewer findings, and intervention notes are post-hoc evidence and may reveal expected answers.

A future evaluation must:

1. select the intended historical checkpoint revision from `meta/jj-commit-map.txt`;
2. check out that checkpoint, where these eval files do not yet exist;
3. give the candidate agent only the intended task prompt and repository state;
4. collect its output independently;
5. score the result afterward using the preserved reports/oracle.

The preservation branch exists to retain history and scoring evidence, not to provide context to the agent under test.

`reports/` contains agent-authored reports and review packages.
`meta/` contains chronology and known human interventions.
`oracle/` is reserved for post-hoc reviewer expectations and must never be visible in the evaluated baseline.
