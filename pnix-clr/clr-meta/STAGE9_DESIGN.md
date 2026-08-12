# clr-meta Compiler Stage9 design

Status: **closed (live gate PASS)** 2026-08-12. Parent floor is Stage8.

## Goal

Clean-process compiler/runtime replay: prove `bin/clr-meta` itself — the
top-level product CLI dispatcher, distinct from the
`compiler-selfhost-runtime` support DLL that Stage1-8 already exercise
directly — behaves correctly *and reproducibly* when invoked as a genuinely
clean OS process, across its real entrypoint matrix.

## What's actually new here (not already covered by Stage1-8)

Every prior stage gate calls `dotnet Pnix.ClrMeta.CompilerSupport.dll <verb>`
directly, or runs `bootstrap-test` in-process via
`dotnet Clojure.Main.dll -m pnix.clr-meta.bootstrap-test` inheriting the
calling shell's environment. None of them ever invoke `bin/clr-meta` itself
— the actual thing a user runs — under a fully cleared environment
(`env -i`, nothing inherited: no `CLOJURE_LOAD_PATH`, no `DOTNET_*`, no
locale). Stage9 closes that gap, and adds a property nothing before it
checked: **replay** — running the identical clean-process command twice and
requiring byte-identical stdout, not just correctness once.

## Entrypoint matrix (4 cases, each run twice independently)

1. `bin/clr-meta --gate` — the evaluator gen0-2 self-interpretation report
   (`pnix.clr-meta.bootstrap-receipt.v1`), `:ready true`, all 9 corpus cases
   `:ok true`.
2. `bin/clr-meta -e "(+ 40 2)"` — evaluator-generation-2 eval mode, exact
   EDN output.
3. `bin/clr-meta FILE.clj` (single-file mode) — same exact output as case 2
   for equivalent source.
4. `bin/clr-meta -e '#?(:clj 1 :cljr 2)'` — negative case: reader
   conditionals stay outside the admitted tool surface, exit 1, stable
   structured error.

`bin/clr-meta`'s tool-level output is Clojure EDN (`pr`'d, not JSON), so the
gate checks it by exact/substring text matching rather than `jq`, matching
this codebase's existing convention for EDN/text assertions elsewhere.

## Non-claims

Stage10-15/N, compiler self-reproduction, IL fixed-point, ClojureCLR
replacement, promotion. This does not re-verify the compiler-selfhost
artifact family's own build reproducibility (that's Stage8) — it verifies
the *tool* that a user actually runs, under isolation Stage1-8 never tested.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage9-gate
```

## Live receipt

`work/compiler-selfhost-stage9-gate.receipt.json` (gitignored) with
`claims.stage9 = true`, `claims.replay_identical_across_two_runs = true`,
`claims["promotion/allowed?"] = false`.
