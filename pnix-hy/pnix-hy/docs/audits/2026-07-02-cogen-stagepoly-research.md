# Deep-research: efficient cogen (A) & stage-polymorphic sacred rewrite (B)

> 2026-07-02 multi-agent deep research (109 agents, 6 angles, 26 primary sources, 119 claims →
> 25 adversarially verified → **24 confirmed / 1 refuted**, ~2.4M tokens). 0026/0028 이후
> 남은 두 hard frontier 답변: (A) EFFICIENT cogen 구축법 (3rd Futamura projection 올바르게),
> (B) sacred stage7 evaluator를 stage-polymorphic으로 만들지 여부.

## Verdict in one line
- **(A) 문헌이 SOLVED, pnix에 깨끗이 매핑, additive, ~zero mirror risk.**
  수정은 더 빠른 런타임이 아니라 — self-application으로 cogen을 만드는 것을 STOP하고
  대신 compiler generator를 hand-write(또는 bootstrap)하는 것. 분야가 1994–2011에 정착.
- **(B) 검증된 증거가 지지하지 않음.** Stage-polymorphic / maybe-lift rewrite of a
  byte-identical-mirror-pinned evaluator는 대부분 UNANSWERED; laziness constraint만
  검증 생존. 어떤 액션 전 전용 second research pass 필요. High risk.

## (A) Efficient / optimal cogen — confirmed findings

**A1 — WHY naive self-application bloats (exactly our >150s finding).** Double/triple
self-application (`cog = ⟦spec⟧(spec, spec)`)으로 만든 cogen은 본질적으로 bloated:
self-applied specializer가 **embedded interpreter + universal value datatype +
time-consuming environment/binding-time manipulations**를 모든 generating extension에
끌고 들어가고 residual **tag/untag** 코드를 남김. Bloat는 *artifact*에 있지 런타임에
있지 않음 — 네 실험(tree-walker / thunk / compiled-closure / scale-sweep 전부 실패)과
일치. *Birkedal & Welinder PLILP'94; Thiemann "Cogen in Six Lines" ICFP'96;
Jones-Gomard-Sestoft §4.8 "tricks under the carpet", §7.3.* (verified 3-0 / merged)

**A2 — The canonical fix: the "cogen approach" (hand-write the compiler generator).**
`mix`를 self-applicable로 만들지 말고, cogen을 **binding-time analysis의
syntax-directed extension**으로 직접 작성: two-level syntax trees만 조작, **인터프리터
없음**, 모든 호스트 feature 자유 사용. "The cogen turns out to be just a simple
extension of a binding-time analysis" (Leuschel). SML/Scheme/MetaScheme/Prolog에서
실용 입증. *Birkedal & Welinder; Thiemann; Leuschel et al. (logen); Glück & Jørgensen
PLILP'95/HOSC'97.* (verified 3-0, 7 merged claims)

**A3 — Minimal recipe = disciplined offline BTA + generating-extension emission.**
Binding-time-annotated program은 적합한 annotation 해석 하에서 **이미 generating
extension** — "this effectively removes every second step" (Thiemann);
JGS §5.8 (Romanenko's `gex`) builds the generating extension syntactically from
two-level annotations with **no self-application**. Anti-bloat discipline lives in
the BTA: **arity raising, let-insertion / bounded static variation, "the trick"** for
ambiguous annotations. *Thiemann; JGS §5.8, §7.3–7.4.* (verified 3-0)

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
**Hand-written pnix cogen** 작성 (`cogen.py` lane, additive) = pnix two-level
(BTA-annotated) AST 위 syntax-directed pass가 generating extension EMIT, 인터프리터
없음 — 기존 offline BTA (`tower.binding_time_analysis`) 재사용. 대안으로
`poly_specialize`에서 Glück 3-step bootstrapping. 어느 쪽이든 **additive,
host/pnix specializer lane, sacred 545×4 mirror에 essentially zero risk**
(새 artifact; stage7 미접촉). 0028 P2를 "research-blocked"에서 known blueprint
engineering task로 전환.

## (B) Stage-polymorphic rewrite of the sacred evaluator — NOT supported

Primary source (**Amin & Rompf, "Collapsing Towers of Interpreters", POPL 2018** — one
`maybe-lift`-parameterized evaluator is an *interpreter* when maybe-lift = identity and a
*compiler* when maybe-lift = lift; also LMS, Truffle first-Futamura, PyPy meta-tracing) was
FOUND, but **none of its claims survived into the confirmed set** — only the laziness constraint
(A7) applies. Verification explicitly warns: **do not read the absence of B findings as
"safe."** No verified guidance exists on:
- how to keep ONE maybe-lift evaluator as interp-vs-compile without two artifacts, and what the
  base language must provide (staging annotations / multi-level types / quasiquotation — pnix,
  being non-homoiconic, lacks these);
- whether an equivalence-preservation strategy (refinement proof, keep-old-as-oracle,
  differential/convergence gating) can preserve **byte-identical** outputs;
- whether maybe-lift staging **inherently changes output artifacts** (and thus necessarily breaks
  a byte-identical mirror).

### pnix action for (B)
**이 증거로 sacred stage7 evaluator를 refactor하지 말 것.** SACRED 유지. 추구한다면
Collapsing Towers / LMS / Truffle / PyPy + refactor-equivalence methods에 대한 전용
second deep-research pass 필요, 가장 안전하게 **sacred 옆에 둔 SEPARATE
stage-polymorphic evaluator**로 prototype (oracle + differential gate), in-place
rewrite 절대 금지.

## Open questions (carried)
1. Hand-written cogen / Glück bootstrapping이 **pure-lazy** 언어에서 작게 유지되는가 —
   thunk-as-dynamic (A7)이 evaluator 상당 부분을 residualize 강제 — laziness help or hurt?
2. pnix가 필요한 minimal BTA discipline (arity raising, let-insertion, the trick) — 그리고
   hand-written cogen vs 3-step bootstrapping 중 어느 쪽이 낮은 위험?
3. (B) 전부 — separate research pass.

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
