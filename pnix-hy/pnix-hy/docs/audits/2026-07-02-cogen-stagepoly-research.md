# Deep-research: efficient cogen (A) & stage-polymorphic sacred rewrite (B)

> 2026-07-02 multi-agent deep research (109 agents, 6 angles, 26 primary sources, 119 claims →
> 25 adversarially verified → **24 confirmed / 1 refuted**, ~2.4M tokens). Answers the two
> hard frontiers left after 0026/0028: (A) how to build an EFFICIENT cogen (3rd Futamura
> projection done right), (B) whether to make the sacred stage7 evaluator stage-polymorphic.

## Verdict in one line
- **(A) SOLVED by the literature, and it maps cleanly onto pnix, additive, ~zero mirror risk.**
  The fix is NOT a faster runtime — it is to STOP producing cogen by self-application and instead
  hand-write the compiler generator (or bootstrap it), which the field settled 1994–2011.
- **(B) NOT supported by verified evidence.** The stage-polymorphic / maybe-lift rewrite of a
  byte-identical-mirror-pinned evaluator is largely UNanswered; only the laziness constraint
  survived verification. Needs a dedicated second research pass before any action. High risk.

## (A) Efficient / optimal cogen — confirmed findings

**A1 — WHY naive self-application bloats (this is exactly our >150s finding).** A cogen produced
by double/triple self-application (`cog = ⟦spec⟧(spec, spec)`) is intrinsically bloated because
the self-applied specializer drags an **embedded interpreter + a universal value datatype +
time-consuming environment/binding-time manipulations** into every generating extension, and
leaves residual **tag/untag** code. The bloat lives in the *artifact*, not the runtime — which
matches our four experiments (tree-walker / thunk / compiled-closure / scale-sweep all failed).
*Birkedal & Welinder PLILP'94; Thiemann "Cogen in Six Lines" ICFP'96; Jones-Gomard-Sestoft §4.8
"tricks under the carpet", §7.3.* (verified 3-0 / merged)

**A2 — The canonical fix: the "cogen approach" (hand-write the compiler generator).** Instead of
making `mix` self-applicable, write the cogen directly as a **syntax-directed extension of a
binding-time analysis**: it manipulates only two-level syntax trees, contains **no interpreter**,
and may use all host features freely. "The cogen turns out to be just a simple extension of a
binding-time analysis" (Leuschel). Proven practical in SML/Scheme/MetaScheme/Prolog.
*Birkedal & Welinder; Thiemann; Leuschel et al. (logen); Glück & Jørgensen PLILP'95/HOSC'97.*
(verified 3-0, 7 merged claims)

**A3 — Minimal recipe = disciplined offline BTA + generating-extension emission.** A
binding-time-annotated program **is already a generating extension** under a suitable
interpretation of the annotations — "this effectively removes every second step" (Thiemann);
JGS §5.8 (Romanenko's `gex`) builds the generating extension syntactically from two-level
annotations with **no self-application**. Anti-bloat discipline lives in the BTA: **arity
raising, let-insertion / bounded static variation, "the trick"** for ambiguous annotations.
*Thiemann; JGS §5.8, §7.3–7.4.* (verified 3-0)

**A4 — Empirically far smaller/faster.** Hand-written cogen gives "remarkable reduction of
generation time and generator size" (Glück & Jørgensen). Self-applicable SAGE: **>10h** to
generate a compiler, generating extensions still >100000ms; hand-written **logen: milliseconds**
(~3 orders of magnitude). (Cross-system comparison — evidences direction, not a controlled
ablation.) (verified 3-0)

**A5 — Middle path (reuse our EXISTING specializer): 3-step bootstrapping.** Glück PSI'2011:
"bootstrapping of compiler generators from program specializers is a viable alternative to the
third Futamura projection… three-step bootstrapping was found to be **faster** and to produce the
**same** compiler generator" as double self-application. Lets pnix reuse its current specializer
to derive the cogen without paying the blowup. **Caveat: demonstrated only in an idealized
recursive-flowchart setting, NOT pure-lazy → promising-but-unproven for pnix.** (verified 3-0;
the stronger "single step, no self-application" variant was **REFUTED 1-2**.)

**A6 — Jones-optimality vs cogen size (answers A4-question).** Jones-optimality measures
*specializer* quality (removing a whole interpretation layer); it governs **residual-program**
quality, **not cogen size** directly. Correlated (both flow from good BTA/tag-elimination) but
distinct axes. A single technique CAN be Jones-optimal + self-applicable + type-checked (Brown &
Palsberg POPL'18, "specialization-safe normalization"). Their optimality is proved for
call-by-value, experimental for normal-order/memoized — relevant to pnix's laziness. (verified 3-0)

**A7 — Laziness (pure-lazy pnix specifics).** Laziness is compiled by treating the
**thunk/`delay` as a DYNAMIC binding-time operation that is residualized**, never performed at
specialization time; correctness needs guarding against duplication/discarding and preserving
evaluation order (Bondorf, Glasgow'90 / POPL'92). **pnix mapping**: BTA must classify thunk
alloc/force as dynamic; a maybe-lift "lift" switch must **NOT cross a thunk boundary**. Effect of
pervasive laziness on cogen *size* remains open. (verified 3-0, single-author → medium confidence)

### pnix action for (A)
Write a **hand-written pnix cogen** (`cogen.py` lane, additive) = a syntax-directed pass over
pnix two-level (BTA-annotated) AST that EMITS a generating extension, containing no interpreter —
reusing our existing offline BTA (`tower.binding_time_analysis`). Alternatively, try Glück
3-step bootstrapping from `poly_specialize`. Either is **additive, host/pnix specializer lane,
essentially zero risk to the sacred 545×4 mirror** (new artifact; stage7 untouched). Would turn
0028 P2 from "research-blocked" into an engineering task with a known blueprint.

## (B) Stage-polymorphic rewrite of the sacred evaluator — NOT supported

The primary source (**Amin & Rompf, "Collapsing Towers of Interpreters", POPL 2018** — one
`maybe-lift`-parameterized evaluator is an *interpreter* when maybe-lift = identity and a
*compiler* when maybe-lift = lift; also LMS, Truffle first-Futamura, PyPy meta-tracing) was
FOUND, but **none of its claims survived into the confirmed set** — only the laziness constraint
(A7) applies. The verification explicitly warns: **do not read the absence of B findings as
"safe."** No verified guidance exists on:
- how to keep ONE maybe-lift evaluator as interp-vs-compile without two artifacts, and what the
  base language must provide (staging annotations / multi-level types / quasiquotation — pnix,
  being non-homoiconic, lacks these);
- whether an equivalence-preservation strategy (refinement proof, keep-old-as-oracle,
  differential/convergence gating) can preserve **byte-identical** outputs;
- whether maybe-lift staging **inherently changes output artifacts** (and thus necessarily breaks
  a byte-identical mirror).

### pnix action for (B)
**Do NOT refactor the sacred stage7 evaluator on this evidence.** It stays SACRED. If pursued at
all, it requires a dedicated second deep-research pass on Collapsing Towers / LMS / Truffle /
PyPy + refactor-equivalence methods, and would most safely be prototyped as a **separate
stage-polymorphic evaluator kept beside** the sacred one (oracle + differential gate), never an
in-place rewrite.

## Open questions (carried)
1. Does a hand-written cogen / Glück bootstrapping stay small for a **pure-lazy** language given
   thunk-as-dynamic (A7) forces much of the evaluator to residualize — does laziness help or hurt?
2. Minimal BTA discipline pnix needs (arity raising, let-insertion, the trick) — and is
   hand-written cogen or 3-step bootstrapping the lower-risk path for pnix?
3. All of (B) — a separate research pass.

## Primary sources
- Birkedal & Welinder, *Hand-Writing Program Generator Generators*, PLILP 1994.
- Thiemann, *Cogen in Six Lines*, ICFP 1996 — dl.acm.org/doi/10.1145/232629.232647.
- Glück & Jørgensen, *Efficient multi-level generating extensions*, PLILP'95 / HOSC 1997.
- Leuschel et al. (logen) — arxiv.org/pdf/cs/0208009.
- Glück, *Bootstrapping Compiler Generators from Partial Evaluators*, PSI 2011 (LNCS 7162).
- Brown & Palsberg, *Jones-Optimal Partial Evaluation by Specialization-Safe Normalization*, POPL 2018 — dl.acm.org/doi/10.1145/3158102.
- Jones, Gomard & Sestoft, *Partial Evaluation and Automatic Program Generation* — itu.dk/people/sestoft/pebook.
- Bondorf, *Compiling Laziness by Partial Evaluation*, Glasgow 1990 / POPL 1992.
- Amin & Rompf, *Collapsing Towers of Interpreters*, POPL 2018 — cs.purdue.edu/homes/rompf/papers/amin-popl18.pdf (Problem B; unverified here).
