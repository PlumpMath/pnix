# Remaining-work decision — F7b / F8 / gate-hog / missed gaps

`/deep-research` verdict (2026-07-07, 105 agents, 3-vote adversarial verification;
10 findings, all but one 3-0). The machine checklist lives in
`resources/pnix_clj/roadmap.edn` (rendered into `docs/WIKI.md`); this document
carries the reasoning and references.

> ★ The 2026-07-07 build order (C → D → F8 → F7b-held) is DONE: C, the D-probes
> (D1–D17), and F8 all LANDED. What follows below is that historical reasoning.
> The CURRENT verdict on everything left is the 2026-07-08 section immediately
> below — read it first.

## UPDATE — verdict on the remaining backlog (2026-07-08, /deep-research, 104 agents, 20/25 claims confirmed 3-0)

The remaining backlog after F8 was four items: **D1c**, **conformance Phase D**,
the **`"`-in-`${}` splice leniency**, and **F7b**. Fresh external evidence (real
Nix C++, Tvix Rust, PL/PE literature) resolves all four — no owner menu needed.
This supersedes the "build order" table below for these items.

| item | 2026-07-08 verdict | why (evidence) |
|---|---|---|
| **splice `"`-in-`${}` leniency** | **REJECTED — do NOT remove; no corpus migration** | Premise was FALSE. Real Nix does NOT reject double-quoted strings inside `${…}`: the `${` opens a full expression context and nixpkgs relies on nested strings pervasively (`"${"foo"}"`→`foo`, `"a${"b${"c"}d"}e"`→`abcde`, all accepted on nix-instantiate 2.34.7). D7's balanced-scanner is CORRECT Nix, not a leniency; tightening would REDUCE conformance. (3-0; nix.dev string-literals manual.) The only true micro-quirk left is `\"`-*escaped* quotes inside splices — not isolated by the research, and not worth a migration either way. |
| **D1c** (explicit-stack non-tail eval) | **DEFER; if ever done, do it as PILLAR work, not conformance** | Real Nix gives NO deep non-tail stack-safety guarantee: native C++ stack + `max-call-depth` (default 10000, added 2.20) that counts **function calls only, not data nesting** — so Nix itself segfaults machine-dependently on our exact nested-list/left-spine shapes (open upstream issue #9627). Our graceful structured `:stack-overflow` bound **already IS the Nix-parity answer** → conformance value ≈ 0. The only reason to touch it is the **functional correspondence** (closure-conversion + CPS + defunctionalization → CEK/Krivine machine; Ager-Biernacki-Danvy-Midtgaard PPDP'03), which is itself a metacircular/projection artifact = M-series pillar work. Tvix confirms the direction (bytecode VM + generators for constant stack on deep data), but even Tvix's TCO only covers tail positions. ★Pitfall: **trampoline** and **"store-allocated continuations"** justifications were REFUTED 0-3 (clojure.core/trampoline; arXiv 1007.4446) — do not build D1c on those; call-by-need needs the Krivine + memoizing-store (CESK) refinement. |
| **conformance Phase D** (impurity/store) | **DEFER — conformance material; do only when a pillar needs it, and then only the pure subset** | Tvix shows the exact architecture to mirror: a swappable `EvalIO` trait (default `DummyIO`), `builder_pure`/`builder_impure`, a `pure_builtins` module vs. an `impure` module gated behind a cargo feature holding just **7** impure builtins (getEnv, hashFile, pathExists, readDir, readFile, readFileType, currentTime). Highest-value **pure/hermetic** subset = hashString, fromTOML, toXML, toFile→content-addressed path, deterministic path realization — no daemon/network. Fetchers / currentTime / currentSystem / flake-refs / findFile are genuinely impure → behind the same seam, only simulatable. Even a full Tvix keeps the store OUT of the evaluator (tvix-store/castore crates). (3-0; docs.tvix.dev, impure.rs.) |
| **F7b** (self-applicable PE for call-by-need) | **RE-CONFIRMED OPEN — stays HELD** | No call-by-need-native self-applicable PE exists. All (mix, Similix, …) target STRICT languages; Similix specializes non-strict *interpreters* but the specializer is strict and laziness is object-level suspension = **call-by-name (no sharing)**, not call-by-need. Matches the 07-07 conclusion; nothing refutes the open status. (3-0; Springer PE chapter.) |

**Net:** the "owner-decision menu" is dissolved — splice is rejected, F7b stays
held, D1c and Phase D are both deferred (D1c reframable as a pillar derivation,
Phase D as pure-subset-when-a-pillar-needs-it). No item here is an urgent
owner-gated call; forward motion is pillar-driven (M-series) or oracle-confirmed
divergence only, per the constitution.

Caveats: Tvix "~4000 LoC / TCO incomplete" figures are Sept-2022 snapshots
(architecture holds in 2024 sources). The functional-correspondence paper targets
the λ-calculus core; applying it to a lazy thunk-sharing evaluator needs the
call-by-need refinement — the D1c recommendation rests on standard PL theory, not
a paper that literally derives a stack-safe lazy Nix machine.

## DECISION — build order

| # | item | verdict | honest label |
|---|---|---|---|
| 1 | **C · gate report cache** | **DO NOW** — proven technique, zero research risk, ~123s/500s win | sound by trace theory given §8 pin + §9 determinism witness |
| 2 | **D · gap probe** | scoped, **oracle-confirmed bugs only** (★no D-angle claim survived verification — no checklist grinding) | each fix needs a nix-instantiate oracle verdict first |
| 3 | **B · F8 weval spike** | bounded spike AFTER C — ~2x ceiling, architectural proof not perf program | correctness = construction argument + differential tests; performance = heuristic |
| 4 | **A · F7b** | **stays HELD** — genuinely open research, owner sign-off required | would yield only a partial correctness TEST even if it works |

## A · F7b — self-applicable specializer in pnix: HELD (open research)

- **No call-by-need precedent exists.** Every classical self-applicable PE —
  flow-chart mix, Scheme0, lambda-mix, Logimix, Similix — targets a STRICT
  language ("Similix: a self-applicable partial evaluator for a higher order
  subset of the strict functional language Scheme"). Exhaustive search of the
  Jones/Gomard/Sestoft book found no self-applicable PE for a lazy language;
  nearest miss is Mogensen's normal-order λ-calculus PE (no call-by-need
  sharing). F7b for a Nix-like language would be NEW research, not engineering.
- **The proven lazy-language route is what we already have.** Bondorf 1990:
  `delay`'s memoization is a side effect pre-1990 self-applicable PEs could not
  handle; the road that worked (Jørgensen POPL'92, commercial-compiler-speed
  lazy-language compilers) is a STRICT-host specializer over a lazy-language
  interpreter — exactly pnix-clj's Clojure specializer.
- **Bounded payoff even on success** (Glück PEPM'09 §5.2): a spec(spec,spec)
  mismatch proves an error; agreement proves nothing — a partial correctness
  test, mechanically checkable, never a proof.
- **If the owner ever green-lights it**: follow the Jones/Gomard/Sestoft §7.4
  11-step recipe — cut pnix to a bare-bones core (no with/assert/contexts),
  clean self-interpreter, hand annotations, build the specializer in Clojure
  FIRST, re-program in pnix LAST; go/no-go gate = Jones-optimality of the
  self-interpreter specialization.

## B · F8 — IR-level PE spike: bounded, after C

- **Prior art is solid**: weval (PLDI 2025) — IR-level 1st Futamura on a
  mostly-unmodified interpreter body over an SSA basic-block CFG (~5 KLoC
  transform; SpiderMonkey needed only +1045/−2 lines, 133 in the interpreter
  fn; production StarlingMonkey). Truffle/GraalVM (PLDI 2017) is the JVM
  precedent.
- **The killing pitfall is precisely known**: constant propagation collapses at
  the interpreter loop backedge (pc merges non-constant → specialization
  returns a copy of the interpreter). Fix = pc-as-specialization-context
  intrinsics (split analysis per context, reconnect merges against exponential
  unrolling). Both weval intrinsics and Truffle `@TruffleBoundary` are
  HAND-PLACED — Truffle tried automatic heuristics and "removed all heuristics
  again" (still true 9 years on). Budget manual annotations.
- **Payoff ceiling is ~2x, honestly**: weval measured 2.17x avg (SpiderMonkey/
  Octane), 1.84x (Lua); a real JIT is still 3.86x beyond. F8 = proof that
  clj-meta can host IR-level PE, not a performance program.
- **JVM caveat**: JVM bytecode is stack-based; weval's transform assumes an
  SSA-CFG IR. A clj-meta spike should work on an SSA-ish view (or
  tools.analyzer AST), no deopt machinery, STATIC residuals only.

## C · gate report cache: DO NOW

- **Design licensed by Build-Systems-à-la-Carte** (ICFP 2018 §4.2.2-3):
  VERIFYING trace = record per report-kind the input hashes + result hash,
  skip re-render iff unchanged; CONSTRUCTIVE trace = also store the artifact,
  licensing copy-instead-of-render. Key = (report-renderer code version ⊕
  capability corpus CAS hash ⊕ §8 runtime-snapshot pin). Soundness conditions
  (determinism + complete input tracking) are exactly what §9 witnesses and §8
  pins already provide. Caveats honored: volatile tasks uncacheable (§6.3);
  Frankenbuild hazard needs the determinism precondition (§4.2.4) — we witness
  it.
- The 123s hog: `report-artifact-is-persisted-as-edn` re-renders 7 corpus
  reports (mirror-pair, determinism, coverage, forward-reference,
  clojure-form, clojure-projection, smoke) that their own deftests already
  rendered in the same gate JVM.
- Drift gates stay untouched: capabilities/wiki/lane-registry checks never go
  through the cache.

## D · missed gaps: oracle-gated probe only

★HONESTY FLAG: zero D-angle research claims (hnix/Tvix/Lix checklists,
JVM-lazy-hosting patterns) survived adversarial verification — the items below
are engineering plausibility, NOT verified-source-backed. Constitution applies:
each becomes work ONLY when a nix-instantiate oracle probe confirms a real
divergence (bug), never as checklist grinding.

Probe targets, by plausibility:
1. **Deep-recursion stack safety** — JVM tree-walk evaluator vs deeply nested
   pnix (e.g. 100k-deep `let`/list nesting; Nix has recursion limits/deepSeq);
   probable StackOverflowError where Nix errors gracefully or succeeds.
2. **builtins strictness matrix** — which builtin arguments are forced vs lazy
   vs Nix (Tvix documents per-builtin strictness).
3. **Catchable-vs-uncatchable error taxonomy** — `tryEval` catches `throw`/
   `assert` but NOT `abort`/type errors in Nix; probe our matrix.
4. **Float formatting/semantics parity** — toString/toJSON of floats.

## References

Jones/Gomard/Sestoft, *Partial Evaluation and Automatic Program Generation*
(§6.4 optimality, §7.4 recipe) · Bondorf 1990 · Jørgensen POPL'92 · Glück
PEPM'09 (Thm 1 p.54, §5.2, §7.2) · Fallin, *weval*, PLDI 2025 · Würthinger et
al., Truffle PE, PLDI 2017 · Mokhov/Mitchell/Peyton Jones, ICFP 2018.
