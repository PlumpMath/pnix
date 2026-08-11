# cljs-meta TODO

This file tracks remaining work toward cljs-meta's meta-circular self-hosting
claims. For current state and verified command output, see `STATUS.md`
(peer-floor statement, closed claims, primary gate) and `FIXED-POINT.md`
(stage sequence, trust root, cross-platform checklist). This file does not
duplicate that detail — it maps what is left, by axis, prioritized.

## Current Remaining Work (verified 2026-08-11)

Verified this pass: `./cljs-meta/bin/cljs-meta-gate` PASSes live (self_test +
fixed_point_test + independent_mini_backend_test, "independent mini backend
DDC: PASS (8 fixtures)"); `bin/build-cljs`, `bin/build-fixed-point.js`, and
`bin/cljs-meta-gate` match what STATUS.md/FIXED-POINT.md describe; the
top-level `bin/pnix-cljs-gate` runs the same three cljs-meta test files;
`flake.nix` lists `aarch64-darwin`/`x86_64-linux`/`aarch64-linux` alongside
`x86_64-darwin` but none of the other three have ever been *run* (only
evaluated as flake outputs), consistent with FIXED-POINT.md's explicit
"appears in flake.nix or evaluates successfully is not supported" caveat.
STATUS.md's "Open claims (do not claim)" block is accurate as of this pass —
nothing below was found already closed and mislabeled open, and the
`independent_mini_backend.js` work from earlier this session is correctly
described (8 fixtures, wired into both gates, not re-flagged as missing).

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

**State:** `independent_mini_backend.js` (added 2026-08-11) is a genuine
from-scratch tokenizer/reader + direct JS-text emitter sharing zero code with
`cljs.js`/`cljs.compiler`/`cljs.analyzer`, cross-validated against the real
self-hosted compiler's `evaluate()`. Covers 8 fixtures: `let`, `if`,
`+`/`-`/`*`, `<`/`>`/`<=`/`>=`/`=`, booleans, keyword literals. Wired into
`test/independent_mini_backend_test.js`, run from both `cljs-meta-gate` and
`pnix-cljs-gate`. This closed the "no DDC exists at all" gap — do not re-flag
it as missing.

**Done looks like (aspirational, not a hard bar):** fixture coverage
approaching clj-meta's ~50-fixture `frontend_selfhost.clj` scope, still on the
same honest behavior-equivalence bar (not bit-identical JS text — two
independently-authored emitters are not expected to produce identical
source).

**Size:** small increments, additive. Each fixture class is a self-contained
mini-backend extension plus a handful of cross-validated fixtures; no
architecture change needed.

- [ ] `do` (sequencing / multiple body forms).
- [ ] Data literals as *values*, not just return position: vectors, maps,
      keywords-as-values (currently keywords only appear as branch results).
- [ ] String literals and basic string handling (`str`, concatenation).
- [ ] `defn`/`fn` (named and anonymous function definition + call).
- [ ] Recursion (self-reference through a bound name).
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
