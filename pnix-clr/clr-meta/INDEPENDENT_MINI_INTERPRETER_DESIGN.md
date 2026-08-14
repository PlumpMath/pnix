# clr-meta independent-interpreter DDC track

상태: **closed (live gate PASS)** 2026-08-12. Compiler Stage1-7
compiler-backend DDC track (`independent_mini_backend.clj`)과 구별됨.

## 목표

`STATUS.md`가 오랫동안 명시적으로 플래그한 것: *닫힌* DDC gap은 Compiler
Stage1-7 family — 두 번째 from-scratch **compiler** backend
(`independent_mini_backend.clj`, `DynamicMethod` IL emitter) — 를 다룬다.
gen0→1→2 evaluator-generation lane을 cross-check하는 두 번째 from-scratch
tree-walking **interpreter**는 별도이며 necessary-but-not-sufficient track이다
— 이 문서 이전 텍스트대로 "interpreter alone would not clear the full Wheeler
bar even if added." 이 track을 닫는다.

## 무엇인가

`src/pnix/clr_meta/independent_mini_interpreter.clj`: `bootstrap.clj` 자체
9-case `conformance-cases` corpus가 증명하는 small, environment-driven Lisp
subset용 from-scratch tokenizer/reader + tree-walking interpreter
(`quote`, `if`, sequential binding `let`, `fn` — anonymous 또는 named,
optional `&` variadic rest param — symbol/environment lookup 및
application). `pnix.clr-meta.main`의 reader나 `pnix.clr-meta.bootstrap/evaluate`와
코드를 공유하지 않는다.

`conformance-cases` 자체(truly empty environment에서 시작해 case마다
`add`/`multiply` 같은 placeholder name을 inject)와 달리, 이 witness는
**real, textual** `bin/clr-meta -e` evaluator-generation-2 tool-eval path에
대해 cross-validate된다 — ordinary arithmetic/comparison/vector symbol
(`+`/`-`/`*`/`<`/`vector`)이 injected environment 없이 이미 resolve됨이 live
확인됨. 따라서 `compile-and-eval`은 같은 이름을 가진 small default
environment를 seed하고, trusted substrate로서 real ClojureCLR host function에
바인딩한다 (이 repo의 다른 DDC witness에서 CLR runtime과 JVM classfile format이
이미 하는 것과 같은 honest role) — tree-walking뿐 아니라 textual source
parsing이 witness 일부이므로 independently-authored *reader*이기도 하다.

## 게이트

`scripts/clr-meta-independent-mini-interpreter-gate`: 9 fixture 각각
(`conformance-cases` shape에 맞추고 literal source text로 번역: `literal`,
`quote`, `if-true`, `if-false`, `sequential-let`, `closure`,
`named-recursion`, `variadic-rest`, 보너스 아홉 번째 `let-bound-recursion`)에
대해 host leg로 clean (`env -i`) subprocess의 `bin/clr-meta -e`를, independent
leg로 별도 clean `dotnet Clojure.Main.dll -e` mini interpreter 호출을 띄우고,
둘 다 expected value에 합의해야 한다.

## Non-claim

Full Wheeler DDC bar, full PNIX language coverage, production-evaluator
replacement. 여전히 9 fixture뿐이며, existing gen0-2 conformance corpus 자체
범위에 맞고, `pnix.clr-meta.main`이 받는 full admitted portable-form surface가
아니다.

## 명령

```sh
./clr-meta/scripts/clr-meta-independent-mini-interpreter-gate
```

## Live receipt

`work/independent-mini-interpreter-gate.receipt.json` (gitignored),
`claims.independent_interpreter_ddc = true`,
`claims["promotion/allowed?"] = false`.
