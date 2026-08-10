# clr-meta Compiler Stage6 design

Status: **closed (live gate PASS)** 2026-08-07. Parent floor is Stage5.

## Goal

Stage5 recompiles the frozen kernel → Stage6; structural description equal to
Stage5; source-hidden fresh-target replay; `promotion/allowed? = false`.

## Non-claims

Stage7+, self-reproduction, IL fixed-point, Trusting-Trust, ClojureCLR
replacement, promotion.

## Commands

```sh
./bin/clr-meta --build-compiler-selfhost-stage6 STAGE5_BUNDLE OUTPUT
./clr-meta/scripts/clr-meta-compiler-selfhost-stage6-gate
```

## Live receipt

`work/compiler-selfhost-stage6-gate.receipt.json` (gitignored) with
`claims.compiler_stage6 = true`, `stage6_fresh_target_replay = true`,
`promotion/allowed? = false`.
