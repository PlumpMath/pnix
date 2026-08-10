# Deep-research #3: how interp→compiler systems work (Q2.2), equivalence validation (Q2.3),
# and the recommended path for pnix (Q2.5) — DECISION

> 2026-07-03 third multi-agent deep research (104 agents, 5 angles, 22 primary sources, 104
> claims → 25 verified → **23 confirmed / 2 refuted**, ~2.0M tokens). Targets the three Q2
> sub-questions that returned ZERO verified claims in pass #2, to decide whether to build a
> SEPARATE stage-polymorphic (maybe-lift) evaluator beside the sacred one. **Outcome: do NOT.**

## Decision (Q2.5): DO NOT build a maybe-lift / separate stage-polymorphic evaluator
The stage-polymorphism goal ("one source is both interpreter and compiler") is **already realized
for pnix by the DERIVE route we shipped in 0029** — the compiler is *derived from* the pnix
interpreter by the specializer (`cogen.compiler_from_interpreter` / `tower.poly_mix_in_pnix`),
not hand-written. Every alternative the research surfaced is worse for our constraints:

- **(a) in-place rewrite of the sacred evaluator** — RULED OUT already (changes source text → breaks
  the mirror's `source_parity`/`compiler_source_parity` lanes).
- **(b) a SEPARATE hand-maintained maybe-lift evaluator** — the practitioner literature **cautions
  against hand-maintained parallel implementations**: RPython's documented architecture is
  **single-source auto-regeneration** — the compiler is derived "by construction" and "regenerated
  anew every time the interpreter is modified, so that they cannot get out of sync." The anti-drift
  guarantee comes from *mechanized generation*, not hand-maintenance. AND the validation
  methodology that would gate such an evaluator against the sacred mirror (**Q2.3**) returned
  **zero verified claims** — we cannot responsibly gate it. So (b) is not warranted.
- **What we do instead** = the RPython-recommended shape, adapted: **derive, don't hand-maintain.**
  The sacred interpreter stays the single source of truth; the compiler is derived from it via the
  cogen approach (0029) and cross-checked by existing differential gates (`cogen_report`,
  `compiled_differential`). This IS the "one interpreter, two roles" unification — via Futamura/
  cogen rather than maybe-lift — with the byte-identical 545×4 mirror untouched.

## Q2.2 — how real systems derive a compiler from an interpreter (ANSWERED, but host-specific)

**Q2.2(a) Truffle/Graal** — turns an AST interpreter into a compiler by **partial evaluation (1st
Futamura projection)**: split into host vs guest compilation; PE collapses AST dispatch
(`add(add(x,y),z)` → `x+y+z`). Authoring requirements: `@Child`(non-final)/`@Children`(final array),
`@CompilationFinal` (constant-fold interpreter-mutable fields), `@ExplodeLoop`, `@Specialization`
(Truffle DSL), and PE-boundary cuts `@TruffleBoundary`/`transferToInterpreter()` (else greedy
inlining explodes code size); `VirtualFrame` must never escape/recurse; `@Child` rewrite deopts.
**All JVM/Truffle-specific — HOST lane only, do NOT transfer to a Hy/CPython host.** (3-0)

**Q2.2(b) RPython meta-tracing** — derives a JIT by **tracing the interpreter "one level down"**
and unrolling the dispatch loop. Not free: an unmodified tracing JIT gave **0.29× (slower)**;
hints progress 0.29× → 0.60× (unroll) → 2.83× (full). Minimal author additions: a `JitDriver` with
**green** (loop-constant, compile-time: code+IP) vs **red** (runtime: frame+ctx) split,
`jit_merge_point` at dispatch, `can_enter_jit` at back-jumps. The **green/red split is a
binding-time separation = the LMS `Rep[_]` analogue** (the only Q2.2(c) point that survived).
**Decisive constraint: RPython REQUIRES the interpreter be written in RPython + run through PyPy's
translation toolchain → a plain Hy/CPython pnix evaluator is INELIGIBLE without a rewrite.** (3-0)

**Feasibility (both mechanisms, real not idealized)**: on the same SOM interpreter, Truffle/PE
reached ~2.3× and RPython/meta-tracing ~3× slower than Java — the interp→compiler transform is
practical. (3-0)

## Q2.3 — equivalence-preserving validation (STILL UNANSWERED)
Translation validation (Pnueli/Necula), refinement/observational-equivalence proofs, metamorphic
testing, differential/oracle testing + convergence gate, bisimulation — **zero verified claims this
pass.** This is exactly the methodology that would gate a separate evaluator; its absence is another
reason not to build one. (Note: pnix already USES differential/oracle+convergence gating in
practice — the 4-lane mirror and `compiled_differential` — we just lack a cited methodology to
claim it *proves* byte-identity; empirically it is our working gate.)

## Refuted (excluded)
- "meta-tracing needs fewer/less-critical annotations than PE" — **1-2** (do NOT assume meta-tracing
  is the lower-effort option).
- a specific Truffle failure-mode enumeration — **0-3** (over-specified).

## Net effect on the backlog
- **Q2 (stage-polymorphic) is CLOSED as a decision**: not pursued via maybe-lift/separate evaluator;
  the goal is met by the derive route (0029). Q2-1 (host staging layer) and Q2-2 (separate maybe-lift
  evaluator) → **won't-do** (superseded by derive route + literature caution + unvalidatable gate).
- Q2.3 and Q2.2(c)/LMS remain literature-open, but are now **moot for the decision** (they only
  mattered if building a separate evaluator).

## Primary sources
- GraalVM Truffle docs (HostOptimization; PE + Futamura); Würthinger et al. "Practical Partial
  Evaluation..." PLDI 2017; CMU 17-396 Truffle slides.
- PyPy/RPython docs (JIT / meta-tracing; JitDriver, green/red, jit_merge_point/can_enter_jit);
  Bolz et al. "Tracing the Meta-Level" (PyPy meta-tracing).
- Marr & Ducasse, "Meta-tracing vs Partial Evaluation" (SOM ~2.3×/~3× comparison), OOPSLA.
