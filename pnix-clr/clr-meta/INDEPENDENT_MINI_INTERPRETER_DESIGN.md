# clr-meta independent-interpreter DDC track

Status: **closed (live gate PASS)** 2026-08-12. Distinct from the
Compiler Stage1-7 compiler-backend DDC track (`independent_mini_backend.clj`).

## Goal

`STATUS.md` has long flagged this explicitly: the DDC gap that *is* closed
covers the Compiler Stage1-7 family — a second, from-scratch **compiler**
backend (`independent_mini_backend.clj`, a `DynamicMethod` IL emitter). A
second, from-scratch tree-walking **interpreter** that cross-checks the
gen0→1→2 evaluator-generation lane is a separate, necessary-but-not-
sufficient track — "an interpreter alone would not clear the full Wheeler
bar even if added," per this doc's own prior text. This closes that track.

## What it is

`src/pnix/clr_meta/independent_mini_interpreter.clj`: a from-scratch
tokenizer/reader + tree-walking interpreter for the small, environment-driven
Lisp subset `bootstrap.clj`'s own 9-case `conformance-cases` corpus proves
(`quote`, `if`, `let` with sequential bindings, `fn` — anonymous or named,
with an optional `&` variadic rest param — symbol/environment lookup and
application). Shares zero code with `pnix.clr-meta.main`'s reader or
`pnix.clr-meta.bootstrap/evaluate`.

Unlike `conformance-cases` itself (which starts from a truly empty
environment and injects placeholder names like `add`/`multiply` per case),
this witness is cross-validated against the **real, textual** `bin/clr-meta
-e` evaluator-generation-2 tool-eval path — confirmed live that ordinary
arithmetic/comparison/vector symbols (`+`/`-`/`*`/`<`/`vector`) already
resolve there with no injected environment. So `compile-and-eval` seeds a
small default environment with the same names, bound to real ClojureCLR host
functions as trusted substrate (the same honest role the CLR runtime and
JVM classfile format already play elsewhere in this repo's DDC witnesses) —
an independently-authored *reader*, not just an independently-authored
*evaluator*, since parsing textual source is as much a part of this witness
as tree-walking it.

## Gate

`scripts/clr-meta-independent-mini-interpreter-gate`: for each of 9
fixtures (matching `conformance-cases`'s shape, translated to literal source
text: `literal`, `quote`, `if-true`, `if-false`, `sequential-let`,
`closure`, `named-recursion`, `variadic-rest`, plus `let-bound-recursion` as
a bonus ninth case), spawns `bin/clr-meta -e` as a clean (`env -i`)
subprocess for the host leg and a separate clean `dotnet Clojure.Main.dll -e`
invocation of the mini interpreter for the independent leg, and requires
both to agree with the expected value.

## Non-claims

Full Wheeler DDC bar, full PNIX language coverage, production-evaluator
replacement. It is still only 9 fixtures, matching the existing gen0-2
conformance corpus's own scope, not the full admitted portable-form surface
`pnix.clr-meta.main` accepts.

## Commands

```sh
./clr-meta/scripts/clr-meta-independent-mini-interpreter-gate
```

## Live receipt

`work/independent-mini-interpreter-gate.receipt.json` (gitignored) with
`claims.independent_interpreter_ddc = true`,
`claims["promotion/allowed?"] = false`.
