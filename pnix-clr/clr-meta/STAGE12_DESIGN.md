# clr-meta Compiler Stage12 design

Status: **closed (live gate PASS)** 2026-08-12. Parent floor is Stage11.

## Goal

Compiler changes remain quarantined until replay and gate admission (the
roadmap's own one-line Stage12 definition): a promotion-policy table
(`proofs/quarantine-policy.tsv`) and a live replay of its two `DONE` gates.

## Gates (6 rows)

- `local-verification` (`DONE`) — every accepted slice must pass Stage11's
  adapter closure. Replayed by running Stage11's gate once.
- `candidate-intake` (`DONE`) — new work enters as
  `proofs/stage-manifest.tsv` rows before any promotion. Replayed by running
  `manifest-check` once.
- `remote-ci` (`HELD`) — the monorepo workflow cannot promote changes.
- `manual-promotion`, `self-modification`, `external-evidence` (`HELD`) —
  all require an explicit receipt that doesn't exist yet.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage12-gate
```

## Live receipt

`work/compiler-selfhost-stage12-gate.receipt.json` (gitignored) with
`claims.stage12 = true`, `claims["promotion/allowed?"] = false`.
