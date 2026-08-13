# clj-meta status (peer host-meta floor)

Last verified: 2026-08-07.

## Peer-floor statement

**clj-meta** is the JVM/Clojure host-meta substrate for `pnix-clj`. Practical
peer floor relative to other metas:

| Peer | Peer floor | clj-meta counterpart |
|---|---|---|
| hy-meta | stage ladder / fixed-point checks | `:gate` bytecode selfhost + stock stage7 rebuild |
| rs-meta | TV + multi-stage selfhost | compiler conformance + self-emit fixed point |
| cljs-meta | fixed-point compiler (stage2==stage3) | backend self-emit determinism / stage1→7 selfhost chain |
| clr-meta | eval gen0–2 + C0–C3 Stage1/2 | kernel + full-eval tower + compiler lane |

Two **separate** lanes (do not conflate):

1. **Bytecode meta compiler** (`src/pnix/clj_meta/{compiler,selfhost,gate}.clj`) —
   analyzer/ASM emit + deterministic self-host checks. Primary product floor.
2. **Stock stage7 rebuild** (`stage7-gate.sh`) — hosted deterministic rebuild of
   Clojure 1.12.5 via Maven/Ant. Reproducible-build evidence, not the
   meta-circular compiler proof.

Neither lane claims JVM-free Clojure self-hosting. Reader, `clojure.core`, and
the JVM remain permanent substrate.

## Closed claims

Live-verified this session (2026-08-07):

```text
./bin/clj-meta-gate selfhost              PASS  ready=true
./bin/clj-meta-gate stage7                PASS  stage7-check OK (Maven 3.9.12)
./bin/clj-meta-gate primary (:gate)       READY ✅
  stage11 multisurface                    OK
  stage12 quarantine                      OK
  stage13 long-horizon                    OK
  stage14 crosshost                       OK  (missing external transcripts held)
  stage15 openworld                       OK
  stageN recursive                        OK
  full-source stage1                      OK  (M12 fallback-free accepted)
  lowering admission                      OK
  diverse double compile                  OK
  reproducible DDC lane                   OK
  (plus prior OK witnesses: bytecode/TV/verifier/language-surface/…)
```

### Fixes that closed stage11–N (this wave)

1. **stage9/10 child classpath** — children invoked `clojure -M:audit-self-source`
   from pnix-clj root (no root `deps.edn`). Now use `-Sdeps` with absolute
   `clj-meta/src` (same shape as primary gate).
2. **stage10 sandbox cwd** — `source-path` now resolves under `CLJ_META_ROOT`
   (`clj-meta/src/...` preferred), so sandbox relocation finds compiler.clj.
3. **lowering-admission** — m6aj `checked-fallback` accepted rows
   (`promotion/allowed?=false`) map to held boundary, not raw-bytecode admission.
4. **stage14** — missing external transcripts and synthetic drift sentinel use
   `:held` (aligned with docstring + invariants), not `:unavailable`/`:rejected`.

Documented closed by design:

```text
boundary policy: direct emit uses host Compiler 0 times; fallback explicit
M12 fallback-free genuine stage1 boundary: ACCEPTED (host-Compiler-fallback-forms=0)
```

## Open / not claimed

```text
full-language-correctness                 false
trusting-trust / Wheeler independent DDC  false
JVM-free self-hosting                     false
external stage14 transcripts (hy/pnix-hy/pnix-clj files)  optional held evidence
```

Stage11–N are **local product/organism closure seeds** with honest held
boundaries (missing cross-host transcripts, checked-fallback lowering), not
Clojure language-runtime replacement.

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

**Current state is stronger than "false" alone suggests.** The "diverse double
compile OK" / "reproducible DDC lane OK" rows above are real, passing gates —
not aspirational. Three concrete pieces are already closed (see `todo.md`
U5/U6/U8/U10 for full detail and 2026-06-29 receipts):

```text
U5  independent-kernel-evaluator-supported-corpus
    kernel.clj (tree-walking value-semantics evaluator) cross-checked against
    compiler.clj on the full 112-case conformance corpus: host≡compiler≡kernel,
    0 unsupported. Honest scope: shares host clojure.core, not a second
    bytecode compiler, no independent deftype/defrecord typegen.
U6  frontend-selfhost
    a self-authored tiny reader + tiny analyzer + direct ASM emitter, sharing
    no recognizer/range-engine/emit-helper code with compiler.clj. Compiles
    61 fixtures (fn/if/do/let/loop-recur/arithmetic/compare/data-literals/
    quote/13 macros/vector-destructuring/fixed multi-arity fn/variadic `&`
    rest-args/count) with ZERO calls into tools.analyzer.jvm or the host
    reader.
U8  fuzz-conformance
    10,000 random-program comparisons (250 programs x 40 inputs), host≡compiler,
    0 divergences found.
```

**`independent-mini-backend-subset` DDC row (verified this session, 2026-08-11):**
this row in `diverse_double_compile.clj` already runs the real 3-way comparison
(host `eval` ≡ `compiler.clj` backend ≡ U6's independent mini backend) — it was
not merely documented, it is a live, passing gate. It was wired to only 14 of
U6's 51 fixtures, covering arithmetic/let/loop/threading-macros/cond/if-let/
comparisons/quot-rem/unary-ops/seq-ops/get/nested-destructure. Grown to **43
fixtures** this session by adding the previously-unwired categories that
already existed and passed standalone in `frontend_selfhost.clj`: `do`,
let-shadowing, boolean/nil/equality branching, all four data literals (vector/
string+keyword/map/set), all three quote forms, the remaining 13 macros
(`when`/`and`/`or`/`not`/`nil?`/`when-let`/`if-not`/`as->`/`cond->`/`cond->>`/
`some->`/`some->-nil`/`some->>`), plain and rest-position destructuring, and
`zero?`/`neg?`. Re-ran `-M:ddc`: `independent-mini-backend-subset -> accepted`
(43/43 agree), full `diverse-double-compile: OK`, no regressions in the other
15 rows. Receipt digest: `4688b206f7cd9c22beb0f3bbc4ae5a69d61fcdb01d806726ef24125f3827838c`.

**Widened again, 2026-08-13: fixed multi-arity `fn` support.** `analyze-fn`
now handles both `(fn [x] ...)` (single arity, unchanged) and `(fn ([x] ...)
([x y] ...))` (multiple *fixed* arities on one function) by generalizing its
AST to a list of arity clauses; `emit-class` emits one ASM `invoke` method per
clause on the same `AFunction` subclass — the same mechanism the real host
compiler uses (each `invoke0..invoke20` override is independently dispatched
by `IFn`'s normal argument-count-based call resolution, so no glue code is
needed on the call side). Variadic `&` rest-args were NOT attempted — that
needs `clojure.lang.RestFn`, a materially different base class with its own
arity-dispatch/rest-collection contract, a separate and larger feature.
Verified against real host `eval` before adding (2-arity and 3-arity cases,
plus that calling with an unmatched arity throws `ArityException` on both
sides, matching host behavior exactly) — not assumed from reading the ASM
code. U6: 51→55 fixtures (`frontend_selfhost.clj`'s own standalone check,
`-M:frontend-selfhost`: all 55 accepted). DDC row: 43→47 fixtures (added the
same 4 multi-arity cases to `mini-backend-ddc-fixtures` in
`diverse_double_compile.clj`, live 3-way host≡compiler≡mini-backend check —
`independent-mini-backend-subset -> accepted`, 47/47 agree). Full
`-M:conformance` (116/116, unaffected — this lane doesn't touch
`compiler.clj`/`kernel.clj`) and full `bin/clj-meta-gate` (`metacircular
gate: READY`) both still green, no regressions.

**Widened again, same day (2026-08-13): variadic `&` rest-args.** The
"separate, larger slice" flagged above as deliberately deferred is now done.
`clojure.lang.RestFn`'s exact contract was reverse-engineered from the real
host, not guessed: AOT-compiled `(fn [a & r] r)`, `(fn [& r] r)`, and
`(fn [a b & r] r)` with the trusted host compiler, then `javap -c`'d the
output `.class` files. Finding: `RestFn` already implements every public
`invoke(...)` overload (arities 0–20 plus a true-variadic 20+ overload)
*concretely* — argument-count matching and rest-sequence collection are
entirely the base class's job. A subclass supplies exactly two things:
`getRequiredArity()` (the fixed-arg count) and ONE `doInvoke` overload whose
parameter count is `fixed-arg-count + 1`, the last slot being the collected
rest sequence (an `ISeq`, or `nil` if no extra args were passed). `emit-class`
now branches on whether any arity clause has a `rest-param`: if so, the
class extends `RestFn` instead of `AFunction` and emits exactly that
`doInvoke`/`getRequiredArity` pair; otherwise the existing per-clause
`invoke` path (from the multi-arity work above) is unchanged. Scope
deliberately narrowed: a variadic clause may not be mixed with other fixed
arities in the same `fn` (real `RestFn` subclasses can do this, but it needs
additional lower-arity `invoke` overrides beyond the two pieces above — a
further slice, not attempted here; `analyze-fn` throws a clear error for
this shape rather than silently miscompiling it). Also added `count` as a
new unary op (`RT.count`, boxed via `Integer/valueOf` — confirmed via the
same `javap -c` reverse-engineering that real `count` boxes to `Integer`,
*not* `Long` like every other numeric op in this file, which would have been
wrong to assume). Verified against real host `eval` before adding: `(fn [a &
r] r)` with 3 args and with exactly 1 (rest = `nil`), `(fn [& r] (count r))`,
`(fn [a b & r] [a b r])`, an unmatched 0-arg call throwing `ArityException`
on both, and that mixing a variadic clause with a fixed clause is correctly
rejected. U6: 55→61 fixtures, all accepted. DDC row: 47→50 fixtures (3 new
variadic cases wired into `independent-mini-backend-subset`, still
accepted). Full `-M:conformance` (116/116, unaffected) and full
`bin/clj-meta-gate` (`metacircular gate: READY`) both still green.

**What's still genuinely open:** full Wheeler DDC needs the independent
backend's coverage to match the *production* corpus, not a 43-fixture subset,
and (harder) bit-identical rather than behavior-identical output — two
different compiler backends targeting the same bytecode format by coincidence
is not the honest bar; behavior equivalence is. U5's kernel is also still an
interpreter, not a second compiler, so it doesn't independently count toward
this claim either.

**Next concrete step:** widen `mini-backend-ddc-fixtures` further using
`frontend_selfhost.clj`'s remaining ~8 fixtures not yet wired in (the ones that
overlap categories already covered, e.g. plain `>`/`>=`/`<=` in isolation,
`inc`/`dec`/`pos?` in isolation) for completeness, then grow U6's own fixture
set itself past 51 toward the full 112-case conformance corpus U5 already
reaches — at that point the claim upgrades from "43-fixture subset" to "full
conformance-corpus independent 2nd compiler cross-validation," the actual
Wheeler bar for this corpus. Full compiler-binary DDC (bit-identical bytecode,
not just behavior) remains a further, harder bar on top of that and stays
explicitly held; see `todo.md` U10 for the CakeML/CompCert/Octagon research
trail this was checked against.

## Primary gate

```sh
# From pnix-clj/clj-meta/
./bin/clj-meta-gate              # full :gate integrated receipt
./bin/clj-meta-gate selfhost     # practical peer floor (bytecode selfhost)
./bin/clj-meta-gate stage7       # stock rebuild (needs mvn on PATH)
```

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `./bin/clj-meta-gate selfhost` | **PASS** | ready=true |
| `./bin/clj-meta-gate stage7` | **PASS** | Maven 3.9.12 |
| `./bin/clj-meta-gate primary` | **READY ✅** | stage11–N + DDC + full-source closed |
| env | JDK 21, Clojure 1.12.5 CLI, Maven 3.9.12 | OK |
