# clr-meta Compiler Stage15 design

Status: **closed (live gate PASS)** 2026-08-12. Parent floor is Stage14.

## Goal

External evidence stays evidence-only until replay and explicit admission
(the roadmap's own one-line Stage15 definition): an evidence-federation
policy table (`proofs/evidence-federation.tsv`) and a live replay of its two
`DONE` sources.

## Sources (6 rows)

- `local-proof` (`DONE`) — local `bootstrap-test` and the compiler-selfhost
  stage receipts are the only current promotion evidence. Replayed by
  running Stage14's gate once.
- `stage-manifest` (`DONE`) — replayed by running `manifest-check` once.
- `remote-ci`, `external-web`, `external-tool`, `human-note` (`HELD`) — none
  can promote a claim without a checked receipt that doesn't exist yet.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage15-gate
```

## Live receipt

`work/compiler-selfhost-stage15-gate.receipt.json` (gitignored) with
`claims.stage15 = true`, `claims["promotion/allowed?"] = false`.
