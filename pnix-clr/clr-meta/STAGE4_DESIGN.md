# clr-meta Compiler Stage4 design

상태: **implemented + gated** (2026-08-07). Live gate
`scripts/clr-meta-compiler-selfhost-stage4-gate` PASS. Parent floor는 Stage3
(`STAGE3_DESIGN.md`).

## 목표

```text
Stage3 (closed) → Stage4 (Stage3 recompiles frozen kernel → semantic +
structural convergence under fresh load)
```

Stage3와 같은 honesty: PE byte-identity 아님, self-reproduction loop 아님,
Stage15/N 아님, host promotion 아님.

## Definition of done

`compiler_stage4=true`는 다음이 모두 성립할 때만:

1. Parent Stage3 lineage hash bind
2. Stage3 (Stage2/1/host 아님)가 frozen kernel → Stage4 PE compile
3. Fresh load: Stage4 + support only
4. target matrix에서 Stage3와 semantic agreement
5. Stage3와 structural description equal
6. Source-hidden fresh-target replay
7. `promotion/allowed?=false`, `self_reproduction=false`, `stage5=false`

## 명령

```sh
# From pnix-clr/
./bin/clr-meta --build-compiler-selfhost-stage4 STAGE3_BUNDLE OUTPUT
./clr-meta/scripts/clr-meta-compiler-selfhost-stage4-gate
```

## Non-claim

Stage5–7 convergence, IL fixed point, Trusting-Trust, ClojureCLR replacement.
