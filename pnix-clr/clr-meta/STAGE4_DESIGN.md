# clr-meta Compiler Stage4 design

Status: **implemented + gated** (2026-08-07). Live gate
`scripts/clr-meta-compiler-selfhost-stage4-gate` PASS. Parent floor is Stage3
(`STAGE3_DESIGN.md`).

## Goal

```text
Stage3 (closed) → Stage4 (Stage3 recompiles frozen kernel → semantic +
structural convergence under fresh load)
```

Same honesty as Stage3: not PE byte-identity, not self-reproduction loop,
not Stage15/N, not host promotion.

## Definition of done

`compiler_stage4=true` only if:

1. Parent Stage3 lineage hashes bind
2. Stage3 (not Stage2/1/host) compiles frozen kernel → Stage4 PE
3. Fresh load: Stage4 + support only
4. Semantic agreement with Stage3 on target matrix
5. Structural description equal to Stage3
6. Source-hidden fresh-target replay
7. `promotion/allowed?=false`, `self_reproduction=false`, `stage5=false`

## Commands

```sh
# From pnix-clr/
./bin/clr-meta --build-compiler-selfhost-stage4 STAGE3_BUNDLE OUTPUT
./clr-meta/scripts/clr-meta-compiler-selfhost-stage4-gate
```

## Non-claims

Stage5–7 convergence, IL fixed point, Trusting-Trust, ClojureCLR replacement.
