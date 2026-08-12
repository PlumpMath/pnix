# clr-meta Compiler Stage13 design

Status: **closed (live gate PASS)** 2026-08-12. Parent floor is Stage12.

## Goal

Long-horizon stale artifact, cache, and source-drift closure (the roadmap's
own one-line Stage13 definition): a freshness/boundary/degradation policy
table (`proofs/horizon-policy.tsv`) and a live replay of its two `DONE`
signals.

## Signals (6 rows)

- `stage-manifest` (`DONE`) — stage status is read from the machine-readable
  manifest, not ambient memory. Replayed by running `manifest-check` once.
- `session-replay` (`DONE`) — long-running sessions must replay through
  deterministic local receipts. Replayed by running Stage12's gate once.
- `stale-evidence`, `external-memory`, `organism-state`, `ambient-network`
  (`HELD`, all `degrade-to-held`) — none of these have a checked receipt
  mechanism yet; the policy's own default is to degrade to `HELD` rather
  than silently keep a `DONE` claim.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage13-gate
```

## Live receipt

`work/compiler-selfhost-stage13-gate.receipt.json` (gitignored) with
`claims.stage13 = true`, `claims["promotion/allowed?"] = false`.
