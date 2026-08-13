# hy-meta status (peer host-meta floor)

Last verified: 2026-08-07.

## Peer-floor statement

**hy-meta** is the Hy/Python host-meta bootstrap for `pnix-hy`. It owns the
deepest explicit stage ladder among host metas (stage1 → stage7 compiler chain,
self-host kernel fixed point, stages 8–15/N product/organism seeds).

| Peer | Peer floor | hy-meta counterpart |
|---|---|---|
| clj-meta | stage7 stock + bytecode selfhost | stage7-check + bootstrap-fixedpoint-check |
| rs-meta | TV + stage chain toward 15-N | stage ladder + parity ledger (kernel vs native) |
| cljs-meta | fixed-point compiler | bootstrap-fixedpoint-check (B==C kernel artifacts) |
| clr-meta | eval gen0–2 + C0–C3 | stage chain + evaluator/kernel path |

**Honest classification:** self-hosting **back-end** (direct kernel → Python AST),
not full meta-circular ownership of the reader. `hy.reader` and name mangling
remain delegated host substrate. Full upstream `hy.compiler` parity is a
post-stage7 track, not a closed claim.

Python proof targets: **3.11** and Homebrew **3.14** only (3.12/3.13 rejected).

## Closed claims

Live-verified this session (2026-08-07) via `./hy-meta/bin/hy-meta-gate primary`:

```text
self-check                         PASS (stage1=6, stage2=42, stage2_self_check=True)
stage7-check                       PASS
  stage_count=7, all_stage_self_checks=True
  compiler/kernel AST+Python+value stage7 mirrors=True
  isolation (modules/macros/globals) ok
  kernel_factorial=120, kernel_loop=120, kernel_features=449.0
```

Documented closed by bootstrap commands (not re-run this session):

```text
chain-check / kernel-check / prime-check / stage3-check / mirror-check
self-host-check / bootstrap-fixedpoint-check / no-fallback-check
parity-ledger-check / stage8..stage15 / stagen seeds
reader-boundary-check / kernel-import-check / native-subset-test
```

## Open claims (do not claim)

```text
full_reader_ownership = false
complete_upstream_hy_compiler_parity = false
full_REPL/hyc/hy2py/zipimport_product_surface = false
Python_3.12_or_3.13_support = false
trusting-trust_defense = false
pnix_language_semantics_ownership = false
```

Stage15/N checks are **local product/organism seeds**, not Hy/CPython replacement.

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

**hy-meta is the furthest along of the five hosts on this axis.** The
`trusting-trust_defense = false` line above is about the *full* bar, not zero
progress — a real DDC-style gate is already closed. From `todo.md`'s
2026-06-29 Deep-Research Audit:

```text
diverse-double-compile-check (CLOSED, wired into smoke)
    builds kernel.hy two independent ways:
      kernel_upstream = via upstream hy.compiler (stage1 seed)
      kernel_direct   = via the direct kernel (stage2 bridge, confirmed by a
                         nonzero direct-kernel hit count, i.e. it actually ran
                         through the new path, not a silent passthrough)
    both then compile kernel.hy and compiler.hy. Outputs agree at all four
    levels: normalized AST, canonical code, raw marshal, timestamped pyc.
    A backdoor present in the direct build path but absent from upstream would
    have produced a divergence here; none was found (green on 3.11 + 3.14).
```

**Why this is not yet "full" Trusting-Trust defense:** one of the two build
paths (`kernel_upstream`) still routes through the real, trusted upstream
`hy.compiler` as its seed — it is not an independently-authored third
implementation. This check catches a backdoor introduced uniquely in the
*direct-kernel* path; it would NOT catch one already present in upstream
`hy.compiler` itself, since that same upstream compiler is one of the two
things being compared. Wheeler's bar requires the second compiler to have no
shared authorship/lineage with the first.

**Independent mini backend added this session (2026-08-11):**
`independent_mini_backend.py` is a new, from-scratch Hy-subset-to-Python-AST
compiler — its own hand-written tokenizer/reader plus direct `ast` node
construction, sharing zero code with `hy.reader`, `hy.compiler`,
`stage1/compiler.py`, or `stage2/kernel.hy`. Python's `ast` module and
`compile()` builtin remain trusted host substrate (the same honest role the
JVM classfile format plays for clj-meta's analogous `frontend_selfhost.clj`).
Wired in as a *separate* check (`independent-mini-backend-check`, in
`smoke_test.py` right after `diverse-double-compile-check`) rather than a
literal third leg of it: the existing DDC check compares whole-file
`kernel.hy`/`compiler.hy` bytecode-artifact hashes, which a bounded tiny
backend cannot meaningfully participate in — same reason clj-meta's tiny
frontend gets its own `independent-mini-backend-subset` row instead of being
merged into its whole-file DDC comparison.

Covers 8 fixtures (arithmetic, comparisons, `if`, `defn`, recursion via
factorial, boolean/`None`-equality branching), each checked against real
upstream Hy (`stage1.compiler.eval_source`) as the host reference. Verified
live this session on both supported interpreters:
`independent-mini-backend-check` -> 8/8 accepted on Python 3.11.15 and 3.14,
with `stage7-check` re-run unaffected on both (no regression from the new
imports).

**Widened to a true 3-way per-fixture comparison (2026-08-11, later in this
session):** each of the 8 fixtures is now checked against *three*
independently-lineaged evaluators, not two — `host_result` (upstream Hy
1.3.0, `stage1.compiler.eval_source`, the same leg
`diverse-double-compile-check` calls `kernel_upstream`), `kernel_direct_result`
(kernel.hy compiled and run through the direct-kernel bridge,
`stage2.load_hy_file(KERNEL_PATH, ...)`, the same leg that check calls
`kernel_direct`), and `mini_backend_result` (the independent mini backend).
`diverse-double-compile-check` already compares upstream vs. direct-kernel at
the whole-file kernel.hy/compiler.hy bytecode-artifact level; this adds a
third, code-independent leg at small-fixture *behavior* granularity, closing
the "does not yet cross-check `kernel_direct` as a formal third leg" gap this
doc previously flagged. Verified live: `independent-mini-backend-check` ->
8/8 accepted with all three legs agreeing, `diverse-double-compile-check`
still `ddc_status: reproduced`, and the full `hy-meta-gate full` ladder
(self-check, chain-check, stage7-check, self-host-check,
bootstrap-fixedpoint-check) still green — no regression from adding the
`kernel_direct` leg.

**What this closes and what it still doesn't:** a backdoor present in *both*
upstream Hy and the direct kernel (e.g. inherited by the direct-kernel build
at some prior bootstrap stage) would still be caught, since the mini backend
shares no code, tooling, or bootstrap lineage with either. It is still only
14 fixtures, not the conformance corpus, and — same honest bar as clj-meta
and cljs-meta already settled on — behavior equivalence, not bit-identical
artifacts. **Next concrete step:** continue growing the fixture set (more
seq/dict ops, further macro coverage) toward parity with
`frontend_selfhost.clj`'s ~50-fixture scope on the clj-meta side.

**Widened further, same day (2026-08-12):** added string literals, list
literals as return values, and `setv`/`while` mutation (8→12 fixtures).
`independent_mini_backend.py`'s `_emit_defn` previously only ever emitted
the *last* body form (wrapped in `return`) and silently discarded every
form before it — fine for the existing pure-expression fixtures, but it
meant `setv`/`while` (which only make sense as statements with side
effects, not as a final expression) had no way to run at all. Added a new
`_emit_stmt` that turns `(setv name value)` into a real `ast.Assign` and
`(while test body...)` into a real `ast.While`, and `_emit_defn` now emits
every body form but the last through it. Verified against both real legs
before adding fixtures (not assumed): `bootstrap.py run -c` (upstream) and
`bootstrap.py kernel-run -c` (direct kernel) both agree with the mini
backend on a 0..9 summing while-loop (45) and a setv-then-arithmetic case
(41), plus a bare string and list literal. Verified live:
`independent-mini-backend-check` -> 12/12 accepted with all three legs
agreeing, `diverse-double-compile-check` still `reproduced`, full
`hy-meta-gate full` ladder still green — no regressions.

**Widened again, 2026-08-13:** added dict literals (string keys only —
Hy keyword literals read as `hy.models.Keyword` objects on the real host, a
reader-model identity this from-scratch backend deliberately does not
reproduce, so keyword-keyed dicts stay out of scope) and a
multi-`defn`-composition fixture (one top-level `defn` calling another,
already supported by the existing `compile_and_eval` loop with no code
change — each `defn` becomes a real module-level `FunctionDef`, so a later
one can call an earlier one by name through the shared `exec()` namespace;
verified this was true rather than assumed). New tokenizer/reader support:
`{`/`}` now tokenize, and `_parse_one` builds a `("__dict__", pairs)`
marker form, emitted via a new `_is_dict` check ahead of the general
call-form dispatch (`ast.Dict` with `ast.Constant` string keys). Verified
against both real legs (`bootstrap.py run -c` / `kernel-run -c`) before
adding fixtures (8→12→14 total). Verified live:
`independent-mini-backend-check` -> 14/14 accepted with all three legs
agreeing, `diverse-double-compile-check` still `reproduced`, full
`hy-meta-gate full` ladder still green — no regressions.

**Fixed this session: missing native-corpus dependency.** A fresh
checkout/venv used to fail `diverse-double-compile-check` and other
native-corpus-dependent checks with
`hy.errors.HyRequireError: No module named 'tests'` (confirmed on unmodified
`bootstrap.py` too — not a regression, a genuine gap). Root cause: `tests/`
(upstream Hy's own `tests/native_tests/*.hy` + `tests/resources/tlib.hy`,
used as a native-Hy oracle) was never materialized in this checkout — only
referenced by path. Fixed via `hy-meta/bin/fetch-native-tests`, which
resolves the already-pinned `hy-src` flake input (`github:hylang/hy` tag
`1.3.1`, hash-verified by `flake.lock`) through Nix and copies its `tests/`
subtree into `pnix-hy/tests/` (gitignored, ~528K, 95 files — not committed).
`nix develop`'s shellHook now does this automatically. A second, smaller gap
surfaced once `tests/` existed: `tests/resources/__init__.py` imports
`pytest` at module load time (upstream Hy's own test-resource file, not
something hy-meta invokes), so `pytest` was added to `flake.nix`'s
`proofPython` and is needed in manual venvs too. Verified live this session:
`diverse-double-compile-check` -> `ddc_status: reproduced` (was previously
unable to run at all), `native-subset-check` -> `ok`,
`parity-ledger-check` -> 100% direct (45/45 files, 1487/1487 forms, 0
fallbacks), `hy-meta-gate full` -> PASS.

## Primary gate

```sh
# From pnix-hy/
./hy-meta/bin/hy-meta-gate              # self-check + stage7-check
./hy-meta/bin/hy-meta-gate self-check
./hy-meta/bin/hy-meta-gate full         # + self-host + fixedpoint subset
```

Env used this session:

```sh
/usr/local/bin/python3.11 -m venv /tmp/pnix-hy-py311-venv
/tmp/pnix-hy-py311-venv/bin/python -m pip install 'funcparserlib ~= 1.0' 'hy == 1.3.1'
export HY_META_PYTHON=/tmp/pnix-hy-py311-venv/bin/python

# Only needed for diverse-double-compile-check / parity-ledger-check /
# native-subset-test (the native-Hy-corpus checks):
./hy-meta/bin/fetch-native-tests                          # materializes tests/
/tmp/pnix-hy-py311-venv/bin/python -m pip install pytest   # tests/resources/__init__.py needs it
```

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `hy-meta-gate primary` | **PASS** | Python 3.11.15 + hy 1.3.0 + funcparserlib |
| full ladder stage8–stagen | not default-run | available via bootstrap.py |
