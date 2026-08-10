# clr-meta Compiler Stage7 design

Status: **closed (live gate PASS)** 2026-08-07. Parent floor is Stage6.

## Goal

Stage6 recompiles the frozen kernel → Stage7; structural description equal to
Stage6; source-hidden fresh-target replay; `promotion/allowed? = false`.

## Non-claims

Stage8+, self-reproduction, IL fixed-point, Trusting-Trust, ClojureCLR
replacement, promotion.

## Commands

```sh
./bin/clr-meta --build-compiler-selfhost-stage7 STAGE6_BUNDLE OUTPUT
./clr-meta/scripts/clr-meta-compiler-selfhost-stage7-gate
```

## Live receipt

`work/compiler-selfhost-stage7-gate.receipt.json` (gitignored) with
`claims.compiler_stage7 = true`, `stage7_fresh_target_replay = true`,
`promotion/allowed? = false`.
