# Deep-research #2: laziness × PE size (Q1/A7) & stage-polymorphic sacred rewrite (Q2/B)

> 2026-07-03 follow-up multi-agent deep research (106 agents, 6 angles, 23 primary sources, 103
> claims → 25 verified → **23 confirmed / 2 refuted**, ~2.5M tokens). 첫 패스(2026-07-02)가
> 명시적으로 UNANSWERED로 남긴 두 질문 대상. Q1은 잘 답변됨; Q2는 **부분**
> 답변 (mechanism yes, refactoring-safety no).

## Q1 (A7) — does laziness help or hurt PE / generating-extension size? ANSWERED

**Q1a — the real hazard is SHARING, not laziness per se.** Naive PE by beta-normalization/
unfolding that ignores sharing **re-duplicates work → residuals run SLOWER than the original**.
Call-by-need shares this hazard with call-by-value; only non-sharing normal-order/CBN is
Jones-optimal under unrestricted normalization. Measured: up to **33× slowdown** for CBV
(Brown & Palsberg POPL'18, Table 3: Min speedup(CBV)=0.03×). So "laziness hurts?" resolves to
"**sharing hurts NAIVE normalization-based PE, call-by-need inherits it, and it is recoverable**."
*Fischer/Silva/Tamarit/Vidal LOPSTR'07; Brown & Palsberg POPL'18.* (3-0)

**Q1b — thunk-as-dynamic bloat mechanism + recovery.** Classifying an enclosing expr dynamic
**propagates downward**, forcing genuinely-static inner subexprs to be *reconstructed* rather than
computed — the exact bloat mechanism. Recovery = **selective eta-expansion**: a uniform
binding-time coercion that recovers lost static computation, auto-enables "The Trick" at sum type,
"pads" so one dynamic occurrence doesn't dynamize static ones, and **can be inserted automatically
by an extended BTA** (no hand-massaging source). *Danvy/Malmkjær/Palsberg TOPLAS'96, LASC'95.* (3-0)

**Q1c — CPS as a non-bloating BTI.** The BTIs from CPS-converting the SOURCE can instead be had by
**writing the SPECIALIZER in CPS** (Bondorf/Similix), leaving source unchanged — and crucially
**without bloating outputs**: residuals are not in CPS, and generating extensions carry no
closure-manipulation overhead. CBN-CPS is a BTI strong enough to achieve **all of deforestation**
via PE (CBV-CPS cannot). *Bondorf LFP'92; Nielsen & Sørensen SAS'95.* (3-0)

**Q1d — feasibility proven for a pure-lazy language.** PE generated a **realistic compiler for a
strongly-typed pure-LAZY language (BAWL, call-by-need)** with code "comparable to hand-written"
(Jørgensen POPL'92); Similix compiles laziness by treating `delay`/thunks as DYNAMIC (confirming
our premise) with no duplication/discarding and preserved evaluation order. BUT **naive
specialization did NOT yield good code — BTIs were required**. *Jørgensen POPL'92; Bondorf'90/91.* (3-0)

**Not answered**: a direct *measured* size comparison of cogen/generating-extensions across
CBV/CBN/CBNeed. Evidence shows sharing-loss makes residuals SLOWER and that lazy compiler-gen
NEEDS BTIs, but not a bigger-vs-smaller size figure.

### pnix action for Q1 (additive, specializer/BTA lane, ~zero mirror risk)
pnix는 **call-by-need (sharing)** 이므로 specializer는 (1) **sharing이 손실될 수 없는 곳에서만
unfold** (affine/right-linear / specialization-safe reducer — modern fix, NOT more static
eval), (2) **static computation 회복용 BTI** 적용: `tower.binding_time_analysis`가 삽입하는
auto eta-expansion, 및/또는 CPS-written specializer. 0029 residuals를 더 작게/강하게.
Refuted 1-2: "BTIs can't compensate for a non-Jones-optimal specializer" — BTIs help but
don't replace a sound reducer.

## Q2 (B) — stage-polymorphic maybe-lift rewrite. PARTIALLY answered

**Q2.1 — the mechanism (ANSWERED).** Amin & Rompf "Collapsing Towers" (POPL'18, Pink/λ↑↓): ONE
evaluator is stage-polymorphic by abstracting over a **`maybe-lift`** parameter — instantiate
`(lambda _ e e)` (identity) → **interpreter**; `(lambda _ e (lift e))` → **compiler**; same source.
Recipe is **mechanical and representation-agnostic**: wrap `maybe-lift` around every user-level
**value constructor** (literal numbers, closures/lambda, cons) and leave dispatch/eliminators
(if, application, car/cdr, var lookup, primitives) unchanged. **pnix being NON-homoiconic does NOT
block it.** *Amin & Rompf POPL'18; namin/pink reference impl.* (3-0)

**Q2.4 — does stage-poly break a byte-identical mirror? (PARTIALLY).** Compile-mode output differs
in KIND (residual ANF code) from interpret-mode (runtime value); with `maybe-lift = identity`
interpret-mode is a **no-op wrapper**, so interpret-mode *outputs* can in principle stay
byte-identical while a separate compile-mode is added. **BUT** (inference, not literature-proven):
making the evaluator stage-polymorphic **changes its SOURCE TEXT**, which breaks any **source-hash-
pinned** lane — and our 545×4 mirror includes `source_parity` + `compiler_source_parity` (source
hashing). So an **in-place rewrite of the sacred evaluator would break the mirror's source lanes.**
(Over-read "compile mode is provably output-optimal" was **REFUTED 0-3**.)

**Q2.2 / Q2.3 / Q2.5 — UNANSWERED (zero surviving claims).** Truffle/PyPy/LMS interpreter→compiler
requirements; proven equivalence-preserving refactoring methods (refinement proofs, translation
validation, metamorphic testing, oracle+differential gate); and the recommended path — the
adversarial verification surfaced **no durable evidence**. Needs a third pass.

**Prerequisite gap (host lane).** The maybe-lift/λ↑↓ recipe requires the **metalanguage** to
provide multi-level staging (`lift`/`reflect`, overloaded elimination, quasiquotation-style
codegen). No verified source confirms the Hy/Python host lane supplies these — a real prerequisite.

### pnix recommendation for Q2
- **In-place rewrite of the sacred stage7 evaluator = RULED OUT**: it changes source text → breaks
  the mirror's source-hash lanes (`source_parity`/`compiler_source_parity`). Consistent with SCOPE_LOCK.
- **The evidence points to option (b)**: build a **SEPARATE maybe-lift evaluator BESIDE** the sacred
  one (identity-instantiation cross-checked against the sacred evaluator as an oracle + differential
  gate), never touching the sacred lane. But note: (i) it needs a host staging layer (prerequisite),
  and (ii) the safe-refactoring methods (Q2.3) and the Truffle/PyPy/LMS requirements (Q2.2) are still
  **unverified** — a third research pass is warranted before building.
- No source makes the recommendation for us; (b) is an engineering judgment the literature supports
  in mechanism but not in refactoring-safety.

## Refuted (excluded)
- "compile mode is provably output-optimal / residual identical to source" — **0-3** (over-read of
  Prop 4.4; compilation removes interpretive overhead but is not a byte-identity guarantee).
- "BTIs cannot compensate for a non-Jones-optimal specializer" — **1-2**.

## Primary sources
- Brown & Palsberg, *Jones-Optimal PE by Specialization-Safe Normalization*, POPL 2018.
- Fischer, Silva, Tamarit & Vidal, *Preserving Sharing in PE*, LOPSTR 2007.
- Danvy, Malmkjær & Palsberg, *Eta-Expansion Does The Trick*, TOPLAS 1996 / LASC 1995.
- Bondorf, *Improving Binding Times Without Explicit CPS-Conversion*, LFP 1992; *Compiling Laziness by PE*, Glasgow 1990/91.
- Nielsen & Sørensen, *CPS-translation and deforestation*, SAS 1995.
- Jørgensen, *Generating a Compiler for a Lazy Language by PE*, POPL 1992.
- Amin & Rompf, *Collapsing Towers of Interpreters*, POPL 2018 — cs.purdue.edu/homes/rompf/papers/amin-popl18.pdf; ref impl github.com/namin/pink.
