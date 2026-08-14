# clr-meta Compiler Stage9 design

상태: **closed (live gate PASS)** 2026-08-12. Parent floor는 Stage8.

## 목표

Clean-process compiler/runtime replay: Stage1-8이 이미 직접 exercise하는
`compiler-selfhost-runtime` support DLL과 구별되는 top-level product CLI
dispatcher인 `bin/clr-meta` 자체가, genuinely clean OS process로 호출될 때
real entrypoint matrix 전역에서 올바르고 *재현 가능하게* 동작함을 증명.

## 실제로 새로운 것 (Stage1-8이 이미 다루지 않는 것)

모든 prior stage 게이트는 `dotnet Pnix.ClrMeta.CompilerSupport.dll <verb>`를
직접 호출하거나, calling shell environment를 상속한 채
`dotnet Clojure.Main.dll -m pnix.clr-meta.bootstrap-test`로 in-process
`bootstrap-test`를 실행한다. 어느 것도 fully cleared environment
(`env -i`, 상속 없음: `CLOJURE_LOAD_PATH` 없음, `DOTNET_*` 없음, locale 없음)
아래에서 사용자가 실제로 실행하는 `bin/clr-meta` 자체를 호출하지 않는다.
Stage9가 그 gap을 닫고, 이전에는 검사하지 않은 속성을 추가한다: **replay** —
동일한 clean-process 명령을 두 번 실행하고, 한 번 맞음만이 아니라
byte-identical stdout을 요구.

## Entrypoint matrix (4 case, 각각 독립적으로 두 번 실행)

1. `bin/clr-meta --gate` — evaluator gen0-2 self-interpretation report
   (`pnix.clr-meta.bootstrap-receipt.v1`), `:ready true`, 9 corpus case 모두
   `:ok true`.
2. `bin/clr-meta -e "(+ 40 2)"` — evaluator-generation-2 eval mode, exact
   EDN output.
3. `bin/clr-meta FILE.clj` (single-file mode) — 동등 source에 대해 case 2와
   같은 exact output.
4. `bin/clr-meta -e '#?(:clj 1 :cljr 2)'` — negative case: reader
   conditional은 admitted tool surface 밖, exit 1, stable structured error.

`bin/clr-meta`의 tool-level output은 Clojure EDN (`pr`'d, JSON 아님)이므로
게이트는 `jq` 대신 exact/substring text matching으로 검사한다. 이 codebase의
다른 EDN/text assertion 관례와 같다.

## Non-claim

Stage10-15/N, compiler self-reproduction, IL fixed-point, ClojureCLR
replacement, promotion. 이것은 compiler-selfhost artifact family 자체의
build reproducibility를 재검증하지 않는다 (그건 Stage8) — Stage1-8이 테스트하지
않은 isolation 아래에서 사용자가 실제로 실행하는 *tool*을 검증한다.

## 명령

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage9-gate
```

## Live receipt

`work/compiler-selfhost-stage9-gate.receipt.json` (gitignored),
`claims.stage9 = true`, `claims.replay_identical_across_two_runs = true`,
`claims["promotion/allowed?"] = false`.
