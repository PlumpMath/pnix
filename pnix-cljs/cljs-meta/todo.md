# cljs-meta TODO

This file tracks remaining work toward cljs-meta's meta-circular self-hosting
claims. For current state and verified command output, see `STATUS.md`
(peer-floor statement, closed claims, primary gate) and `FIXED-POINT.md`
(stage sequence, trust root, cross-platform checklist). This file does not
duplicate that detail — it maps what is left, by axis, prioritized.

## Current Remaining Work (verified 2026-08-11)

Verified this pass: `./cljs-meta/bin/cljs-meta-gate` PASSes live (self_test +
fixed_point_test + independent_mini_backend_test, "independent mini backend
DDC: PASS (8 fixtures)" at the start of this pass, widened to 14 fixtures —
see §2 below); `bin/build-cljs`, `bin/build-fixed-point.js`, and
`bin/cljs-meta-gate` match what STATUS.md/FIXED-POINT.md describe; the
top-level `bin/pnix-cljs-gate` runs the same three cljs-meta test files;
`flake.nix` lists `aarch64-darwin`/`x86_64-linux`/`aarch64-linux` alongside
`x86_64-darwin` but none of the other three have ever been *run* (only
evaluated as flake outputs), consistent with FIXED-POINT.md's explicit
"appears in flake.nix or evaluates successfully is not supported" caveat.
STATUS.md's "Open claims (do not claim)" block is accurate as of this pass —
nothing below was found already closed and mislabeled open, and the
`independent_mini_backend.js` work from earlier this session was widened in
this pass from 8 to 14 fixtures (`do`, strings, vectors-as-values, named-`fn`
recursion), wired into both gates, verified against the real host, no
regressions.

The five open claims split into two genuinely actionable axes and three
structural scope boundaries that read as "false" by design, the same way
sibling hosts keep their own trust roots or runtime dependencies permanently
open rather than "complete."

---

### 1. Multi-platform byte determinism (non-x86_64-darwin) — actionable, medium size

**State:** only `x86_64-darwin` is checked closed. `flake.nix` declares
`aarch64-darwin`, `x86_64-linux`, `aarch64-linux` as build targets, but no
receipt, hash, or gate run exists for any of them.

**Done looks like:** every unchecked box in FIXED-POINT.md's "Cross-platform
closure checklist" for each of the three remaining platforms — clean
`./bin/build-cljs`, stage2==stage3 byte identity, source-closure identity,
stage-input-hash chain, no stage0 bootstrap-only namespace, `fixed_point_test.js`
+ `examples/fixed-point.js` + `pnix-cljs-gate` all green, `nix flake check`
green natively, and artifact hashes from each platform compared/explained
(not silently normalized).

**Size:** medium. No new code is required — the build/gate machinery already
exists and is platform-generic (Node.js + Clojure CLI + JDK). This is blocked
on **access to actual aarch64-darwin / x86_64-linux / aarch64-linux
machines/CI runners**, then executing the existing multi-minute cold
fixed-point build three times and reconciling hashes. Effort is
infra-and-execution, not design.

- [ ] `aarch64-darwin`: run full checklist, record receipt + hash comparison.
- [ ] `x86_64-linux`: run full checklist, record receipt + hash comparison.
- [ ] `aarch64-linux`: run full checklist, record receipt + hash comparison.
- [ ] Normalize/explain any path, tool-version, or timestamp differences
      between platform receipts before claiming determinism.
- [ ] Flip `multi_platform_byte_determinism` in STATUS.md only once all three
      platforms are closed (partial completion stays `platform-pending`).

---

### 2. Trusting-Trust / DDC depth — actionable, small-to-medium, incremental

**State:** `independent_mini_backend.js` (added 2026-08-11, widened
2026-08-11, again 2026-08-12, and again 2026-08-13) is a genuine from-scratch
tokenizer/reader + direct JS-text emitter sharing zero code with
`cljs.js`/`cljs.compiler`/`cljs.analyzer`, cross-validated against the real
self-hosted compiler's `evaluate()`. Covers 34 fixtures: `let` (including
recursive/nested vector destructuring), `if`, `do`, `when`, `cond`, `->`,
`+`/`-`/`*`, `<`/`>`/`<=`/`>=`/`=`, booleans, keyword literals, string
literals, vector/map/set literals as return values, the seq ops
`get`/`nth`/`count`/`conj`/`nil?`, `assoc`/`update` on maps, and named `fn`
literals including self-recursion (factorial, fibonacci). Wired into
`test/independent_mini_backend_test.js` (using `assert.deepEqual` so
vector/map-returning fixtures compare structurally, not by reference), run
from both `cljs-meta-gate` and `pnix-cljs-gate`. This closed the "no DDC
exists at all" gap — do not re-flag it as missing.

**Scope note found and resolved this pass:** `core/evaluate` runs `cljs.js`'s
`eval-str` with `:context :expr`, which only accepts a *single* top-level
expression — `(defn ...) (foo)`-style multi-form source fails on the real
host itself (confirmed live), not just the mini backend. So `defn` is not a
reachable DDC fixture shape at all under this evaluate path. Recursion is
instead expressed the way both backends can agree on it: a self-referencing
named `fn` literal invoked in place, e.g. `((fn fact [n] (if (<= n 1) 1 (* n
(fact (- n 1))))) 6)`. Do not re-flag `defn`/multi-form support as missing —
it is out of reach of this DDC harness by the real host's own design, not an
oversight.

**Done looks like (aspirational, not a hard bar):** fixture coverage
approaching clj-meta's ~50-fixture `frontend_selfhost.clj` scope, still on the
same honest behavior-equivalence bar (not bit-identical JS text — two
independently-authored emitters are not expected to produce identical
source).

**Size:** small increments, additive. Each fixture class is a self-contained
mini-backend extension plus a handful of cross-validated fixtures; no
architecture change needed.

- [x] `do` (sequencing / multiple body forms).
- [x] String literals and basic string handling (concatenation available via
      `str`, not yet exercised by a fixture).
- [x] `fn` (named and anonymous function definition + call).
- [x] Recursion (self-reference through a named `fn` literal).
- [x] Map literals as return values (keyword/string keys only).
- [x] `get`, `nth`, `count`, `conj`, `nil?` seq ops.
- [x] Vector destructuring in `let` bindings (out-of-bounds positions bind to
      `nil`/JS `null`, not `undefined`, matching this backend's own `nil`
      mapping — verified against `(nil? c)` on a too-short source vector).
- [x] Nested destructuring in `let` bindings (`[[a b] c]`,
      `[a [b c] d]`) — `bindPattern` now recurses; verified against the real
      host. Map destructuring in `let`/`fn` params is still unsupported
      (`emitFn`'s params only accept flat symbols).
- [x] `assoc` (variadic key/value pairs) and `update` (with a `fn`-literal
      updater; bare-symbol updater functions like `inc` are not yet
      supported — `update`'s third argument must itself emit as a callable
      expression, and this backend has no builtin-symbol-as-value table
      yet).
- [x] Set literals (`#{...}`) as return values, represented as a plain JS
      array — confirmed live that `clj->js` gives a small cljs set stable
      insertion order on the real host, so `assert.deepEqual` comparison
      holds; not de-duplicated at emit time (fixtures never contain
      duplicate elements), so this is a narrower model than a true set.
- [x] `when`, `cond`, `->` macros (thread-first rewritten to nested list
      forms at the AST level before a single `emitExpr` pass, not emitted as
      a JS-level threading helper).
- [ ] Keywords-as-values in non-branch position (currently only appear as
      branch results/map keys/vector elements — not yet a documented gap,
      just unexercised by a fixture).
- [ ] Map destructuring in `let`/`fn` params (`{:keys [a b]}` style) — still
      open, no fixture or backend support.
- [ ] Bare-symbol call values (e.g. `inc`/`dec` used as `update`'s updater or
      as a `->` step without parens) — still open; would need a small table
      mapping known builtin symbol names to inline JS arrow functions when
      they appear in value position rather than call-head position.
- [ ] `when-let`/`if-let`, `str`, more seq ops (`map`/`filter`/`reduce`) —
      still open, natural next widening targets.
- [ ] Re-run STATUS.md's "Trusting-Trust defense roadmap" honesty language
      (fixture count, scope caveat) after each widening pass so the doc never
      drifts ahead of actual coverage.

---

### 3. `pnix_language_semantics_ownership = false` — structural, not a gap

**State:** by design. README.md: "The evaluator is a host mechanism. It does
not own PNIX language semantics, service admission, or artifact approval."
`pnix-cljs/CLAUDE.md` draws the same boundary: cljs-meta is pnix-agnostic,
`pnix-cljs` owns pnix parse/evaluate, and "`cljs-meta` proof or repeat
compilation may verify the implementation, but cannot gate ordinary
`pnix-cljs` evaluation."

**Done looks like:** N/A under the current architecture. Closing this claim
would mean cljs-meta absorbing pnix-cljs's runtime responsibilities, which
would violate the stated repo boundary rather than complete it — the same
shape as hy-meta's explicit "hard non-goal: never pursue independence from
the Python runtime" entries.

**Size:** none as implementation work. Optional: reword this line in
STATUS.md's "Open claims" block to mark it as an intentional scope boundary
(mirroring hy-meta's non-goal framing) rather than leaving it looking like an
unclosed gap next to two genuinely actionable claims.

- [ ] (optional, docs-only) Add a one-line non-goal annotation next to this
      claim in STATUS.md so it reads as "by design" rather than "pending."

---

### 4. `independent_of_Node_Closure_cljs.core = false` — structural, large if ever pursued

**State:** explicit, permanent trust root per FIXED-POINT.md: Node.js, the
Google Closure runtime, `cljs.core` runtime + macro bootstrap kernel,
`cljs.reader`/`cljs.tools.reader`, the fixed-point stage harness, and the
embedded `cljs.core` analysis cache are all named as substrate that stays
outside the self-hosted artifact. This is the same honest shape as clj-meta's
JVM classfile format, hy-meta's CPython `ast`/`compile()`, and rs-meta's
`rustc`-as-toolchain — every sibling host keeps one non-negotiable trust
floor.

**Done looks like:** an independently-built JS execution substrate plus a
from-scratch reimplementation of `cljs.core`/Closure-equivalent semantics not
leaning on Node — effectively a second ClojureScript runtime.

**Size:** large-to-unbounded, and arguably against the project's own
trust-root model (a host language always needs *some* trusted substrate to
bottom out on). Not recommended as near-term work; treat as permanently open,
same as siblings' analogous trust roots.

- [ ] No action item. Keep STATUS.md's honest phrasing ("Node.js, the Google
      Closure runtime, and `cljs.core` itself remain shared trust-root
      substrate") rather than implying this is closable.

---

### 5. `full_ClojureScript_product_replacement = false` — structural, out of current scope

**State:** explicit non-goal. README.md calls this "the first executable
slice"; `pnix-cljs/CLAUDE.md` says the seed "does not claim full parity with
the three established hosts."

**Done looks like:** full ClojureScript language/tooling parity — complete
macro system beyond the bootstrap kernel, full core library surface, source
maps, REPL/tooling ecosystem, npm interop, etc. This is "reimplement
ClojureScript as a product," a different and much larger project than the
self-hosting proof this repo targets.

**Size:** very large; not prioritized; no current plan to pursue.

- [ ] No action item unless project scope is explicitly redefined.

---

## Priority order

1. **DDC fixture widening (#2)** — cheapest, incremental, directly strengthens
   the newest and least-mature closed claim.
2. **Multi-platform closure (#1)** — well-defined and mechanical, but blocked
   on non-x86_64-darwin machine/CI access rather than code.
3. **#3–#5** are not omissions; they are scope boundaries. The only
   recommended action is the optional STATUS.md wording tweak in #3 so the
   "Open claims" list doesn't read as three more TODOs of the same kind as
   #1/#2.

## Host toolchain (dot-nix, 2026-08-13)

dot-nix wraps `shadow-cljs` to inject `PNIX_CLJS` and installs `pnix-cljs` /
`clojurescript` as the **runtime** host. Full replacement of shadow as the
**build backend** (compile CLJS projects without shadow) is **not** claimed.

### Open

1. Optional pnix-native CLJS build pipeline that can replace shadow for
   non-Kimchi / simple modules (if ever desired).
2. Document which shadow hooks should call `pnix-cljs` vs cljs-meta for
   eval-at-build-time.


## Host-language import of pnix product library (user intent, 2026-08-13)

Context from home-manager (`dot-nix`) integration:

- `pnix-<host>-pnix` = pnix-language surface (REPL/eval of `.px`) on this host.
- `pnix-<host>-<lang>` = host-language interpreter/compiler used for day-to-day
  host development.
- Libraries produced by the **pnix product half** of this host are **host-
  language libraries**: they must load in *this* host language. They are **not**
  assumed to be portable common bytecode for other hosts.
- A future **common portable `.px` library** track (historical pnix-meta style)
  is deferred; do not block host-local import work on that.

dot-nix can only set PATH/env (classpath, PYTHONPATH, link paths, NODE_PATH,
DLL HintPath). Anything that requires a real packaging format is product work
below.


### cljs — remaining product work

1. Document Node `require` entry for `share/pnix-cljs` (module name, ESM/CJS).
2. Optional npm package publish so host CLJS/Node projects do not rely on
   NODE_PATH to a nix store share/.
3. Shadow integration hooks remain optional; runtime host is pnix-cljs, not a
   portable .px bytecode package.

