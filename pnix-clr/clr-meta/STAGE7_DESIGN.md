# clr-meta Compiler Stage7 design

상태: **closed (live gate PASS)** 2026-08-07. Parent floor는 Stage6.

## 목표

Stage6가 frozen kernel → Stage7 recompile; Stage6와 structural description
equal; source-hidden fresh-target replay; `promotion/allowed? = false`.

## Non-claim

Stage8+, self-reproduction, IL fixed-point, Trusting-Trust, ClojureCLR
replacement, promotion.

## 명령

```sh
./bin/clr-meta --build-compiler-selfhost-stage7 STAGE6_BUNDLE OUTPUT
./clr-meta/scripts/clr-meta-compiler-selfhost-stage7-gate
```

## Live receipt

`work/compiler-selfhost-stage7-gate.receipt.json` (gitignored),
`claims.compiler_stage7 = true`, `stage7_fresh_target_replay = true`,
`promotion/allowed? = false`.
