# clr-meta Compiler Stage6 design

상태: **closed (live gate PASS)** 2026-08-07. Parent floor는 Stage5.

## 목표

Stage5가 frozen kernel → Stage6 recompile; Stage5와 structural description
equal; source-hidden fresh-target replay; `promotion/allowed? = false`.

## Non-claim

Stage7+, self-reproduction, IL fixed-point, Trusting-Trust, ClojureCLR
replacement, promotion.

## 명령

```sh
./bin/clr-meta --build-compiler-selfhost-stage6 STAGE5_BUNDLE OUTPUT
./clr-meta/scripts/clr-meta-compiler-selfhost-stage6-gate
```

## Live receipt

`work/compiler-selfhost-stage6-gate.receipt.json` (gitignored),
`claims.compiler_stage6 = true`, `stage6_fresh_target_replay = true`,
`promotion/allowed? = false`.
