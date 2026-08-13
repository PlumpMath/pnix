# pnix-clj todo - clj-meta backed pnix runtime

Updated: 2026-07-08 KST

> ★ BACKLOG STATUS (2026-07-08): the post-F8 "owner-decision menu" is DISSOLVED
> by a /deep-research verdict (104 agents, 20/25 claims 3-0). See
> `docs/REMAINING_DECISION.md` "UPDATE 2026-07-08" and `resources/pnix_clj/roadmap.edn`.
> — splice `"`-in-`${}` leniency: **REJECTED** (premise false; D7 behavior is correct Nix — keep).
> — D1c non-tail eval: **DEFERRED** (graceful bound already IS Nix parity; only worth it as a metacircular abstract-machine derivation, not conformance).
> — conformance Phase D: **DEFERRED** (conformance material; pure subset via a Tvix-style EvalIO seam only when a pillar needs it).
> — F7b: **RE-CONFIRMED OPEN / HELD**.
> Net: nothing here is an urgent owner-gated call. Forward motion is pillar-driven
> (M-series) or oracle-confirmed divergence only. Do NOT re-present these as a menu.
>
> ★ LANDED SAME DAY (2026-07-08, acting on the verdict):
> — **D18** · && || -> boolean-operand typing (oracle-confirmed divergence found
>   while deriving the machine from eval-binary: all three lanes leaked truthiness
>   on the logical operators — the R2 Phase D class, unaudited). Host
>   strict-violation reasons :non-bool-{and,or,implies}-operand, lowering
>   require-bool on both operand forms, px evalIf-pattern in evalBinary
>   (resources + checkout && ||). Short-circuits stay lazy; type errors
>   tryEval-uncatchable. Strict gate 278/269/0. Pin: logical-operand-bool-nix-parity.
> — **M7** · the abstract machine (pnix-clj.machine), derived from eval-ast* by
>   the functional correspondence (PPDP'03), call-by-need Krivine+memoizing-store,
>   value algebra SHARED with the evaluator (public seam extracted — control
>   transformed, values one definition). Closed lexical fragment, static free-var
>   refusal. 35-source differential exact agreement; D1c shapes: plus-100k ok /
>   list-100k ok (iterative realize) where tree-walk holds; 256KB-thread witness
>   machine-ok vs tree-walk-SOE. Pins: machine-derivation-agrees-with-evaluator,
>   machine-constant-stack-beyond-treewalk. Follow-ups: fragment growth,
>   :machine report capability, fuel/coverage hooks.
> — **M7b** · machine fragment growth: attrset(rec/inherit/static-keys) ·
>   select(or-default = :unwind delimited catch, tryEval의 모양) · has-attr ·
>   assert · with(scope chain) · string-template(공유 interpolation algebra,
>   __toString 경유 포함). 새 모드 :force/:unwind; free-var 규칙 정련(=
>   default-env 이름만 정적 거부, 그 외 :unbound-var 런타임 패리티); 반복적
>   MAP realize. Differential 35→78행 divergence 0 첫 실행; 256KB 증인에
>   attrs-3k 추가. ★파일: 파서가 중첩 attrset 리터럴에서 초선형(20k≈52s;
>   기계 run+realize 0.2s) — 오라클 프로브 후에만 작업(D-후보). 다음 성장:
>   builtins env · param patterns · dynamic keys.
> — **M7c** · 기계가 default env 아래서 실행: builtins var/select 해소,
>   비단순-클로저 적용은 apply-callable에 통째 위임(부분적용·D2 lazy 위치·
>   :call-target-not-callable까지 단일 정의; builtin 내부 = 손-배치 값-대수
>   경계, weval 규율). tryEval=[:try-eval] 프레임(D3 catch set 정확);
>   [:select-finish]=nullary-builtin-result seam(eval-select 재배선);
>   __curPos 미러(source-position 승격, 섀도 무시 bug-for-bug). 정적 walk
>   단순화(free-var 분석 삭제). Differential +44행 divergence 0 첫 실행.
>   잔여 성장: param patterns(위임 아닌 네이티브 프레임 — 본문이 기계
>   제어에 남아야) · dynamic keys · path/import · 계측+:machine report.
> — **D19** · pattern 람다 오라클 정합(3-lane): default는 lazy+knot 재귀
>   스코프(뒤 formal 참조·미사용 미평가·cycle=무한재귀), 필수 formal은
>   pattern 순서로 여분-키보다 먼저 검사, `...` 없으면 여분 키 error(파서는
>   :ellipsis? 기록해왔음 — evaluator들이 무시), 인자는 attrset 필수, @as는
>   실인자만. host=env-ref knot / lowering=pattern-guard+PROMISE-KNOT 방출 /
>   px=재귀 let이 공짜 knot. 12행 매트릭스 전 레인 정합, tryEval 불포착.
>   핀: pattern-lambda-nix-parity.
> — **M7d** · pattern 람다 기계 네이티브: [:pattern-bind] 프레임 —
>   guard+knot은 value-level, default는 기계 thunk(in-loop force, 공유
>   blackhole), 본문 기계 제어 유지. 정적 walk에서 param-pattern 거부 삭제.
>   Differential +22행 divergence 0. 잔여 거부: dynamic key · :path ·
>   :import · 비단순 let — fragment가 사실상 정적-키 전체 언어면 도달.
> — **D20** · dynamic attr key 오라클 정합(3-lane): 충돌=silent overwrite였음
>   (wrong VALUE) → :duplicate-attr held; 비문자열 키 str-강제변환 →
>   :dynamic-attr-key-not-string(or 불포착); 구성-시점 laziness 보존.
>   host=attr-key-value-result seam+contains? 검사 / lowering=attr-key-string
>   +attrset-pairs / px=isString+hasAttr 검사. 핀: dynamic-attr-key-nix-parity.
> — **M7e** · dynamic key 기계 네이티브: attrs-transition(정적 프리픽스
>   인라인+중복검사) + [:attrs-key]/[:select-key]/[:hasattr-key] 프레임,
>   select 정적/동적 arm이 select-transition 하나 공유(드리프트 불가), or가
>   키-오류 불포착(unwind 통과). Differential +20행 divergence 0. 잔여 거부:
>   동적-세그 path 바인딩 · :path · :import · 비단순 let.
> — **M7f** · :path 리터럴 + :import(공유 *import-resolver* seam — 한
>   바인딩으로 양 레인이 같은 42 해소, unwired는 동일 held). +11행 0.
> — **M7g** · 기계 = 1급 capability: :machine report kind(공유
>   differential-corpus 155행 = 핀과 같은 목록 + 256KB 증인, honest labels
>   내장) + deps alias + 게이트 스텝 + fuel 패리티(같은 volatile·같은 tagged
>   throw — safe-eval류 예산이 양 레인 동일 구속; coverage는 의도적 미러
>   안 함 = evaluator 레인의 계측). M7 headline capability :smoke→:machine.
>   **M7a–M7g 완주: fragment = 동적-세그 path 바인딩·비단순 let 빼고 전체
>   언어면.** 남은 것: 조립 슬라이스(property-fuzzer/tower 추가 레인).
> — **M7h** · 조립: 퍼저 5번째 속성 machine-property — 랜덤 typed 소스에서
>   기계⇄evaluator **정확 일치**(ok/held 모두, collapse보다 강함)+shrinking.
>   첫 스윕 120 소스 divergence 0. **M-에픽 모양 완성: 유도(a)→언어 성장
>   (b–f)→capability 승격(g)→상비 검증 함대 편입(h).** 다음: 게이트-속도
>   슬라이스(리포트 JVM ~15개 통합) · fragment 꼬리(저순위).
> — **게이트-속도** · report batch(13 kinds 1-JVM, 콜드스타트 12회 제거+웜
>   JIT; 캐시는 kind별 — 과대주장 교정함). 실패 의미론 보존.
> — **M7i** · dynamic-seg path 바인딩 기계 네이티브(attrs-path-transition +
>   [:attrs-path] 프레임 + merge-attr-path 공개 승격 단일정의). +11행 0.
>   기계는 이제 파서가 생산 가능한 모든 형태 실행(비단순 let은 방어적 잔존).
> — **★D21 필드(오라클-확정, evaluator 후속)**: Nix는 a.${k}=v를 중첩-lazy
>   데수가로 취급 — 동적 서브키는 부모 강제 시 평가({ a.${1} = 1; } unforced
>   통과, attrNames=[a]) + literal↔path 병합 양방향({a.${"b"}=1; a.c=2;}.a.c
>   =2). 우리 3-lane은 구성-eager+충돌-strict = too-strict held (wrong value
>   아님). 착지 시 기계는 공유 seam으로 자동 추종.
> — **D21 LANDED(당일)** · 파서 데수가로 해결: path->nested 일반화(D10의
>   every?-string? 가드 삭제) — a.${k}=v ≡ a = { ${k} = v; }. 중첩-lazy 키
>   (attrNames=["a"], 미강제 통과)·literal↔path 병합 양방향·중복 leaf는
>   D20이 eval에서 포착. 13행 오라클 매트릭스 그린. 연쇄: evaluator path
>   분기·기계 [:attrs-path] 프레임 = 방어적 사문화(유지), 기계 무수정 추종,
>   **lowering frontier 공짜 폐쇄**(clj-meta 3 일치) → px lane이 새 정직
>   blocker(:px-runtime-run-error) = px 백로그 필드(D8–D17 패턴). rec 직접
>   동적 키의 lowering frontier는 별개로 잔존.
> — **D22 LANDED(당일)** · let = attrset과 같은 binds 프로덕션: ① bare
>   ${"리터럴"} 세그 파스-폴드(충돌=파스 오류 양순서·let 이름 허용·quoted
>   템플릿 키는 D20 eval-dup 유지) ② parse-let이 path->nested+merge-attr-
>   bindings 공유(dotted 병합·중복 이름=파스 오류(구현은 조용한 섀도잉이었음
>   — 오라클-wrong)·동적 서브세그=D21 lazy·진짜-동적 TOP='not allowed in
>   let'). 26행 매트릭스 그린, 기계 0수정 추종. ★D20의 오라클-미검증 핀 1개
>   (`let s={a=1; ${"a"}=2;}; in 1`=ok로 핀돼 있었음, Nix는 파스 오류)
>   적발·교정 — D5/D19 클래스 3번째.
> — **★D22 필드(파서 frontier)**: dotted let(let a.b = 1; in a.b → Nix 1,
>   우리 :unsupported-syntax). 기계/evaluator가 파서 공유라 differential은
>   이미 일치 — 파서 슬라이스.

Updated: 2026-07-02 KST

## ⛳ THE ROAD (owner, 2026-07-02 — read this before picking ANY work)

```text
걷는 길      = ~/pnix-hy 의 길 (메타서큘러 투영 툴킷), 단 Clojure/JVM 방식으로.
걷지 않는 길 = ~/clj-msv (과거 pnix-clj, MSV/gate-graph 실험)
             = ~/pnix-old (과거 pnix, 표면 기능/문장처리 축적)

⚠ 길이 같다고 Python/Hy 를 다루는 것이 절대 아니다: pnix-hy 는 코드·의존성·
  투영 대상 어느 것도 아니고 "수준 참고"일 뿐. pnix-clj 의 투영 상대는 오직
  Clojure/JVM (clj-meta). Python/Hy interop·브리지·이식 일절 금지.
```

pnix-hy가 보여주는 "수준" = 메타서큘러 능력 자체가 제품의 기둥:
Futamura 특화(specialize, 건전성 우선·gap 기록) / 타워 단일 진입점
(read→compile→run→project→collapse, 층별 의미보존 증거) / 양방향 투영 왕복
(meaning_preserved + drift 분류) / 순수·자원제한 샌드박스 평가(safe_eval) /
내용주소 캐시 평가(cached_eval) / action 판정(accepted·held·rejected+witness) /
기계생성 능력 인덱스(--capabilities, 중복개발 방지) / 모든 능력의 *_report()
회귀 고정. pnix-hy를 복사하지 말고(내부는 완전 다름) 이 수준을 pnix-clj의
강점(4-lane 교차검증·게이트·receipt·clj-meta bytecode self-host) 위에 세운다.
작업 목록 = 아래 "🗼 METACIRCULAR LEVEL ROADMAP" (M1..). builtin/문자열
컨포먼스는 기둥이 요구할 때만 채우는 재료다.

This file is the live development map for `~/pnix-clj/pnix-clj`.
It replaces the copied `~/pnix-hy/pnix-hy/todo.md` plan. The old Hy/Python
stage plan is not the architecture here — the ROAD is role-translated, the
internals are not.

Core decision:

```text
pnix-clj = Clojure/Java/JVM bootstrap/front-end for the pnix runtime path
../clj-meta = Clojure meta-circular stage15..N compiler/evaluator backend
pnix runtime (.px) = repo-owned runtime artifacts under resources/pnix_clj/pnix_runtime
pnix mirror = pnix-side self-observation/evidence layer
~/pnix-old = original corpus/fixture source for human gap reading and vendoring
~/pnix = separate new pnix / outer-envelope ABI standard owner, not a pnix-clj dependency
```

`pnix-clj` must not become the old MSV/gate-graph experiment. That work can be
archived separately, for example under `~/clj-msv`. This repository is now the
pnix runtime path that uses `clj-meta` as the production compiler/evaluator
substrate.

Project purpose:

```text
pnix-clj is a human-operated meta-circular language projection lab.
It is not an AI agent project.
It is not a coding-agent runtime.
It is not an autonomous planning/execution system.
```

The work here is for a developer to manually discover and implement how far
Clojure/Java/JVM meta-circular machinery and pnix meta-circular machinery can
be pushed. The central research object is language expressiveness projection:

```text
Clojure/Java/JVM ecosystem expression power
<-> clj-meta stage15..N compiler/evaluator evidence
<-> JVM ecosystem surfaces reachable from Clojure
<-> pnix runtime representation written in .px
<-> pnix-side mirror observation
```

Every feature should be judged by whether it improves the Clojure/Java/JVM
ecosystem <-> pnix projection and mirror evidence. It should not be judged by
whether it can drive an agent, route tasks, synthesize plans, run a coding
workflow, or become the old `pnix` / `pnix-clj` / MSV experiment under a new
name.

External checkout roles:

```text
~/pnix-old
  original corpus/fixture source. Humans may read it to understand gaps or
  refresh vendored fixtures, but normal pnix-clj runtime/test/report/gate paths
  must not read it.

~/pnix
  new pnix repository for ABI standard management at the outer receipt/mirror/
  evidence envelope level. It may define envelope vocabulary later, but pnix-clj
  must still pass its own gates without reading or depending on it.

pnix-clj
  independent Clojure/Java/JVM ecosystem <-> pnix host-faithful projection
  project. It validates with repo-owned resources and the explicit ../clj-meta
  backend dependency.
```

Root invariant:

```text
clojure mirror
-> pnix runtime (.px)
-> pnix mirror
```

The Clojure side is the bootstrap/compiler mirror. It shows how host Clojure and
`clj-meta` see the form, bytecode, evaluation, determinism, and fallback surface.
It does not replace the `.px` runtime. The pnix runtime must stay as a `.px`
runtime artifact, and the pnix mirror must read that runtime back in pnix terms.
The main proof obligation is the bridge between those two mirrors.

Runtime location invariant:

```text
normal pnix-clj runtime/report path
-> resources/pnix_clj/pnix_runtime/pnix-mirror-runtime
-> resources/pnix_clj/pnix_runtime/pnixc-pnix
-> selected resources/pnix_clj/pnix_runtime/stdlib entries
```

Parent checkout paths such as `../pnix-mirror-runtime`, `../pnixc-pnix`, and
`../stdlib` are import sources only. They are not runtime roots.

Structural inheritance:

```text
pnix-hy + hy-meta
  and
pnix-clj + clj-meta
```

walk the same road. The sameness is structural, not linguistic. `pnix-clj` must
not use Hy or Python as implementation machinery, but it should preserve the
same role sequence that `pnix-hy/hy-meta` was trying to close:

```text
host-language mirror/bootstrap
-> meta compiler/evaluator substrate
-> pnix runtime artifact expressed as .px
-> pnix-side mirror
-> evidence/admission/corpus growth
```

So every copied `pnix-hy` idea must be translated by role, not by name. If an old
step meant "host mirror", it becomes a Clojure/clj-meta mirror step. If it meant
"runtime becomes pnix", it remains `.px` runtime. If it meant "mirror closes the
loop", it becomes pnix mirror evidence, not a Clojure-only success.

Host-language expression projection law:

```text
pnix-hy runtime  = Hy/Python-faithful projection into its own pnix runtime
pnix-clj runtime = Clojure/Java/JVM-faithful projection into its own pnix runtime
```

`pnix-clj` does not try to understand Hy/Python, and `pnix-hy` does not try to
understand Clojure/Java/JVM ecosystem semantics. The sameness between the
projects is only the mirror discipline and runtime-in-pnix direction. Their
language semantics are separate and host-local.

Projection means more than transpilation. The pnix runtime must be able to
represent Clojure plus the Java/JVM ecosystem surface without losing host
meaning:

```text
Clojure reader data / forms / Java interop / JVM objects
-> pnix-clj projection terms
-> Clojure eval, macroexpand, namespace, var, dynamic binding, interop,
   exception/control-flow, Java object/class/member access, bytecode, and
   runtime effect semantics
-> clj-meta stage15..N compiler/evaluator mirror evidence
-> internal pnix runtime (.px)
-> pnix mirror receipt
```

The JVM ecosystem surface is a first-class research surface when it is reachable
from Clojure and backed by receipts: Java classes, constructors, methods,
fields, reflection, classloader behavior, JVM exceptions, Vars/namespaces,
dynamic binding, bytecode evidence, and host runtime effects.

The near-term goal is host fidelity before host parity:

```text
pnix-clj compares against Clojure/Java/JVM and clj-meta meaning.
pnix-hy compares against Hy/Python meaning.
pnix-clj does not compare its language semantics against pnix-hy.
```

That is sequencing, not a permanent non-goal. Later, after each pnix-language
runtime has a strong host-local projection and mirror pair, the project family
may do sync work between runtimes: semantic parity checks, common brain
experiments, and semantic/common ABI design. `pnix-clj` must earn that stage by
first making Clojure/Java/JVM <-> pnix expression projection strong enough to
compare without flattening host semantics.

During the current phase, commonality is allowed only at the outer evidence
envelope level:

```text
allowed common surface:
  receipt shape, source hash, trace id, mirror event envelope,
  accepted/held/rejected lifecycle, evidence/admission discipline

forbidden common surface:
  macro semantics, eval semantics, import/namespace semantics,
  host object model, exception/control-flow semantics, type/value model
```

There is no current-phase plan inside pnix-clj to build a common pnix brain,
semantic ABI, or cross-host parity layer. A separate `~/pnix` may own outer
envelope ABI standards, and future sync work may define deeper common semantics,
but pnix-clj must not depend on that repository now and must not commonize host
semantics before Clojure/Java/JVM <-> pnix projection is strong.

Runtime-before-projection law:

```text
1. Complete the internal pnix runtime first.
2. Complete the two local mirrors for this repo:
   - clojure mirror: host/clj-meta/JVM view of the computation
   - pnix mirror: pnix-side view of the internal .px runtime behavior
3. Only after those mirrors can generate stable evidence should broader
   Clojure/Java/JVM host-language projection expand.
```

This is why the current work has been focused on `.px` runtime execution,
pnix mirror rows, cross-mirror verdicts, Rust-grounded invariance pressure,
and stage7 lock-ins. Host-language projection without a completed pnix runtime
and mirror pair would create another host-only interpreter path and would not
give the later stability this repository is supposed to provide.

---

## 🗼 METACIRCULAR LEVEL ROADMAP (owner course-correction, 2026-07-02 — PRIORITY OVER conformance filler)

Owner: "왜 문장처리쪽으로 가나? clj-msv(과거 pnix-clj)/과거 pnix 길 금지. ~/pnix-hy가
어떻게 하는지 읽어라 — 너도 너 나름대로 수준이 그래 높아야 된다."

Reading ~/pnix-hy (README + todo): its PILLARS are metacircular capabilities, not
surface conformance — `specialize_pnix` (Futamura partial evaluation with gap
recording and soundness-first rules: never emit a partial fold that changes
meaning), `meta_circular_tower` (read→compile→run→pnix→collapse, one entrypoint,
per-layer meaning-preservation evidence), bidirectional projection with
`projection_value_roundtrip` (meaning_preserved) + `classify_drift`,
`check_action` (accepted/held/rejected + witness), `--capabilities` generated
capability index (anti-duplication), SCOPE_LOCK, every capability regression-
pinned by its own `*_report()`. pnix-hy is NOT to be copied (independent project,
totally different internals) — role-translate the LEVEL onto pnix-clj's own
strengths (4-lane cross-checking, gates, receipts, clj-meta bytecode self-host).

What pnix-clj already has at this level: 4-lane run-source, value_roundtrip /
emit_form_roundtrip (verification of fixed cases), translation_validation
catalog, witness/receipts, strict-audit, live-oracle, fuzzer, coverage, and the
clj-meta bytecode self-host (a pillar pnix-hy does NOT have). What is missing:

### 🔬 DEEP-RESEARCH-DRIVEN ROADMAP (2026-07-03) — Clojure/JVM-unique metacircular gaps

A `/deep-research` sweep (112 agents, adversarially verified) ranked the
implementable-but-missing capabilities Clojure/JVM uniquely affords. Status
tracked here so nothing is re-developed:

- [x] **F1 · Futamura 2nd projection (generating extension), cogen-free**
  (2026-07-03): `pnix-clj.futamura`. gen = program-AGNOSTIC pnix→JVM-bytecode
  compiler built the cogen-free way (Latifi DLS'19 — no self-applicable PE);
  in our system gen(p) = `specialize-to-host(p, {}, full-env)` (fold nothing =
  pure compile). Verified on the corpus: interp == 1st projection == 2nd
  projection (gen), gen's compiler-id CONSTANT across all programs (= it's a
  compiler) while the 1st-projection residual VARIES per program (=
  per-program specialization) — the crisp 1st-vs-2nd distinction. 3rd
  projection (cogen) STATED with its genuine proof anchor (Glück PEPM'09 Thm 1:
  self-generating cogens = 3rd-projection cogens), honestly flagged
  stated-not-built. Jones-optimality reported as a structural property (lowering
  never emits an interpreter dispatch loop → gen(p) carries no residual
  interpreter), flagged as structural argument not mechanized proof. Verified by
  `futamura-second-projection`; report `:futamura`, alias `-M:futamura`.
- [ ] **F2 · Jones-optimality as a measured quality target** — beyond the
  structural note: specialize the `.px` SELF-interpreter to a program and show
  the residual removes the interpretation layer (Brown-Palsberg POPL'18
  affine-variable specialization-safe normalization as the buildable blueprint).
  Genuine metric, higher effort (needs specializing evaluator.px).
- [x] **F5 · translation validation** — ALREADY EXISTS
  (`pnix-clj.translation-validation`); per-compilation source-form↔bytecode
  semantic-preservation. Extend per Necula PLDI'00 if needed.
- [x] **F3 · grammar fuzzing** — ALREADY EXISTS (`pnix-clj.grammar-fuzzer`,
  positive/negative). MISSING extension: clojure.spec/Malli generative
  function-schema contracts to auto-drive DIFFERENTIAL fuzzing across the 4
  substrates (test.check counterexample search) + Var-indirection runtime
  contracts. [ ] to build.
- [ ] **F4 · tools.analyzer.jvm AST-pass substrate** — a manipulable
  Clojure-on-Clojure AST + custom pass pipeline (Python/Hy has no equivalent);
  clj-meta already USES tools.analyzer.jvm — expose it as a reusable pass lane.
- [ ] **F7 · self-generating cogen proof anchor** — mechanize Glück Thm 1's
  structural-equivalence check as a proof-carrying property of the tower (only
  meaningful once F1's cogen is actually constructed).
- [ ] **F8 · weval-style IR-level PE** — stronger PE on a mostly-unmodified
  interpreter body at the IR level (no Truffle-style rewrite); lower-friction
  path to deeper specialization. Research/spike.
- (F6 GraalVM/Truffle grounding = context, not a build item.)

### M1. pnix partial evaluator — `specialize` (Futamura stage 1) [FIRST]
- [x] **First slice LANDED (2026-07-02)**: `pnix-clj.specialize/specialize`
  (source, static-env) → residual pnix AST + `pnix-clj.unparse` emitter
  (full grammar, fully parenthesized; `${`-bearing strings emitted as safe
  concatenations; plain-inherit bindings re-emitted as `inherit n;` to avoid
  rec self-cycles) + `strip-positions` structural roundtrip. Soundness as
  planned: recursive let folds at the WHOLE-let node via the production
  evaluator (sibling names never resolve to the static env; partially-dynamic
  lets stay structurally intact — sound because the residual IS pnix);
  if prunes only on real static bools (non-bool static cond = gap
  `:if-non-bool-condition`, full residual, and the whole-node fold is
  bypassed so the lenient evaluator cannot erase the gap); lambdas/calls/
  imports are `:heavy` (never folded, no fuel yet) and lambda params shadow
  statics (capture-free literal substitution). Fold = delegation to
  `evaluator/eval-ast` on closed subtrees, so folds cannot disagree with
  evaluation; held folds keep residual + gap. `specialize/report` runs
  differential verification per case (eval(residual, dynamics) ==
  eval(source, statics∪dynamics)) over the pnix-hy-A4/A5/A15-translated
  cases + substitution/pruning/shadow/partial-fold — 10/10 accepted.
  Tests: `unparse-roundtrip-full-grammar` (24 sources) +
  `specialize-futamura-first-slice`.
- [x] **Second slice LANDED (2026-07-02) — fuel + call folding + partial
  select**: `evaluator/*fuel*` step-budget seam (one-line pre-check in
  eval-ast*, tagged throw) + `eval-ast-with-fuel` (fixed 8MB-stack thread —
  deep recursion burns FUEL before blowing the stack; any escaping Throwable
  incl. StackOverflowError becomes held `:fuel-eval-threw`, never a crash;
  fuel signals are re-thrown through builtin catch-alls so callbacks can't
  swallow them). This seam is shared with M5 safe-eval. specialize: lambda-
  parameter refs are closed at their lambda via a `:lams` set (mirror of let's
  `:sibs`) so fully-applied closed calls fold; heavy (call/lambda) subtrees
  fold under fold-fuel=4096 on the fuel evaluator — `(x: x+1) 5` with x=100
  static folds to 6 (capture-free proof), `builtins.length [1 2 3]` → 3,
  `foldl' (+) 0 [1..4]` → 10, divergent `let f = x: f x; in f 1` → gap
  `:fold-fuel-exhausted`; default-scope globals count as fixed (lexical beats
  with-scope) so builtin-bearing subtrees stay foldable; partial select picks
  a static entry out of a mixed plain attrset (lazy values make discarding
  dynamic siblings sound). Report now 14 differential cases, 14/14 accepted.
- [x] **Third slice LANDED (2026-07-02) — Futamura projection to the host +
  gate wiring**: `specialize-to-host` closes the residual over its sorted
  dynamic parameter names as a pnix lambda, APPLIES it to the dynamic
  argument literals in-form (clj-meta's eval==compiled check compares
  values, and two function instances never compare equal — so the host lane
  receives the closed application, whose value is data), lowers it, and
  compiles/evaluates via `clj-meta/eval-lowered` — every case carries the
  bytecode compile receipt with determinism `:ok`. Report v1 = 14
  differential + 6 futamura rows, 20/20 accepted; wired as the
  `:specialize` report-artifact kind + `:report-specialize` alias + a gate
  step. This is the pnix-clj-native completion of pnix-hy's specialization
  pillar, one level deeper: residual pnix -> Clojure form -> clj-meta JVM
  bytecode with a determinism receipt.
- [x] Specialization cache (2026-07-02, M1×M6 composition):
  `specialize-cached` — content-addressed key (position-stripped AST hash
  + statics hash + epoch), cached-eval idiom (schemad key/clear!/stats),
  bypasses computed fresh (parse-failed / non-data statics /
  specialize-held) so the cache can never change an answer; results are
  pure EDN data (verified by read-string round-trip). The tower's
  specialize-residual layer now runs through it, so repeated corpus
  climbs pay the fold cost once per epoch.
- [ ] M1 follow-ups (as needed): per-call fold-fuel option; residual
  lambdas as REUSABLE compiled artifacts (needs a function-value equality
  story in the host lane).
- New ns `pnix-clj.specialize`: `(specialize source static-env)` →
  `{:residual-ast .. :residual-source .. :fully-static? .. :value .. :gaps [..]}`.
- Walk the parsed AST with a static env: fold literals/arithmetic/if-with-static-
  bool-condition/select-on-static-attrset/static let bindings (RECURSIVE let
  scope — learn from pnix-hy A4: two-pass name-set + fixpoint, NEVER resolve a
  sibling name against the outer env, and if a binding can't fold, gap the whole
  let rather than emitting a sequential approximation).
- Soundness-first rules (pnix-hy A-group lessons, translated): if-condition must
  be a real bool to prune (A15); never drop bindings on multi-path attrsets (A5);
  any uncertainty → record gap + keep dynamic residual, do NOT partially rewrite.
- Residual = pnix AST + an UNPARSER back to pnix source (new, small: we own the
  grammar; round-trip parse(unparse(ast)) == ast as its own report check).
- Differential verification report (`specialize-report`): for each case, eval
  (specialize src statics) residual with the dynamic remainder == eval src with
  full env; report artifact like mirror-error; gate-wired.
- Later: residual → lowering → clj-meta compile = actual Futamura projection to
  JVM bytecode (pnix program specialized into a compiled artifact — this is the
  pnix-clj-native form of pnix-hy's specialization, one level deeper).

### M2. Tower entrypoint — `pnix-clj.tower/run-tower` — LANDED 2026-07-02
- [x] `pnix-clj.tower/run-tower`: ONE call climbs read → emit-roundtrip →
  direct-eval → lowering → clj-meta-host (bytecode determinism surfaced) →
  px-runtime → pnix-mirror, then a COLLAPSE verdict (:collapsed value +
  agreeing layers + witness hashes, or :held/:rejected with the blocking
  layer/pair). Pure repackaging of run-source/run-mirror/cross-mirror —
  the only NEW evidence is the emit-roundtrip layer (parse(unparse(ast))
  structural equality, reusing M1's unparser). `tower/report` collapses
  the whole mirror-pair corpus (155/155) and carries a deliberate
  held-probe (appendContext, a frontier source) proving the verdict
  degrades honestly instead of pretending. Wired as the `:tower`
  report-artifact kind + `:tower`/`:report-tower` aliases + a gate step.
  Verified by `tower-single-entrypoint-collapse`.
- [x] M1×M2 composition follow-up (2026-07-02): a `:specialize-residual`
  layer joined the tower — fold-only specialization whose residual must
  re-evaluate to the direct value, so the M1 specializer's meaning
  preservation is re-proven across the WHOLE tower corpus (155/155 on
  first run) on every gate, far beyond its own 14 differential cases.
- One API: source → parse → (specialize?) → eval(direct) → lower → clj-meta
  compile/eval → project value back → collapse; per-layer receipts + a
  meaning-preservation verdict per adjacent pair; single tower report.
- This is a REPACKAGING of existing lanes (run-source already computes most of
  it) into an explicit tower with collapse semantics — low risk, high legibility.

### M3. Reverse projection — LANDED 2026-07-02
- [x] `pnix-clj.synthesize/form->pnix`: a whitelisted CORE of ordinary
  Clojure expression forms → pnix AST/source (literals, vectors,
  string-keyed maps, arithmetic/comparison ops, str, not, if, let, 1-arg
  fn, application, get/contains? → select/has-attr, count → builtins.length),
  deny-by-default: anything else held `:non-projectable-form` with the
  offending form; qualified (`ns/sym`) and Java-interop (`.method`)
  symbols statically denied. KEY: this is NOT the inverse of lowering's
  emitted forms (those are runtime-helper-specialized) — like pnix-hy's
  synthesize it projects the human-shaped subset. Semantic trap handled:
  Clojure let is SEQUENTIAL vs pnix's RECURSIVE let — a let projects only
  when no binding references its own/later names (then the semantics
  coincide), else held `:sequential-let-not-projectable` instead of
  silently changing meaning. Verification composes the pillars: clj-meta
  evaluates the ORIGINAL Clojure form (bytecode receipt, determinism :ok)
  and the synthesized pnix must collapse through the whole M2 TOWER to
  the same value — 11 projected + 5 held-honesty cases, 16/16. Wired as
  `:synthesize` artifact + aliases + gate step; capabilities regenerated.

### (original M3 sketch)
- Inverse of lowering for the value/expression core (literals, arithmetic, let,
  if, vectors, maps, select). Round-trip report: pnix → lower → synthesize →
  pnix, meaning_preserved via evaluator; and Clojure-first direction:
  form → pnix → eval == clj-meta eval of form. Gaps recorded for
  non-projectable forms (host interop, mutation) — deny-by-default like interop.

### M4. Capability index — LANDED 2026-07-02
- [x] `pnix-clj.capabilities`: index derived ONLY from code (lanes from
  `receipt/lane-order`, artifact kinds from the promoted
  `report-artifact/supported-kinds` def, deps aliases from deps.edn,
  builtin table + unprefixed scope from `evaluator/default-env`, public
  API via ns-publics over a fixed namespace list) → deterministic
  markdown `docs/CAPABILITIES.md` (no timestamps). `clojure
  -M:capabilities` regenerates; `clojure -M:capabilities-check` (gate
  step) fails on drift, so gaining/losing a capability without
  regenerating the doc fails the gate. NOTE: editing deps.edn or adding a
  builtin/report kind now requires regenerating the doc (that is the
  point). Verified by `capabilities-index-generated-and-drift-checked`.

### (original M4 sketch)
- Machine-generated from the report-artifact registry + builtin table + lane
  list; anti-duplication lookup like pnix-hy's `--capabilities`; docs-drift
  check in the gate (regenerate + diff).

### M5. Resource-limited pure eval — LANDED 2026-07-02
- [x] `pnix-clj.safe-eval`: sandbox-BY-DESIGN (runtime purity gates already
  hold impure builtins) + explicit limits as structured verdicts, reusing
  the M1 fuel seam untouched (`eval-ast-with-fuel`, fixed 8MB stack).
  `safe-eval source {:fuel n :pure-only? bool}` → fuel exhaustion =
  `{:limit-exceeded :fuel}`, runtime purity gate tagged
  `{:limit-exceeded :impure}`, `:pure-only?` REFUSES statically-impure
  sources before evaluating (`:static-impure-use`). `static-purity-check`
  = conservative AST walk (no evaluation): `builtins.<impure>` selects,
  bare impure names (with-scope reachable), and dynamic `builtins.${..}`
  access flagged undecidable-impure; single source =
  `evaluator/impure-builtins`. Report 8/8; wired as `:safe-eval` artifact
  + aliases + gate step (before capabilities-check). The capabilities
  drift gate fired on this slice's own changes and was cleared by
  regeneration — the M4 mechanism working as designed.

### (original M5 sketch)
- pnix-hy's `safe_eval`: evaluation is a sandbox BY DESIGN (purity) plus
  explicit resource limits. pnix-clj already has purity (impure builtins held)
  and a fixed-stack evaluator lane; missing are FUEL (step budget) and
  depth/size limits surfaced as a structured verdict
  (`{:ok? .. :limit-exceeded :fuel|:depth|:impure ..}`) instead of a crash.
  Wire a step counter through eval-ast* (cover! already threads a hook point),
  default unlimited, opt-in via dynamic var; `safe-eval` public API + report.
- Also: `static-purity-check` (AST walk listing impure builtin uses without
  evaluating) — cheap, pairs with the strict-audit corpus report.

### M6. Content-addressed cached eval — LANDED 2026-07-02 → 🗼 ROADMAP M1–M6 COMPLETE
- [x] `pnix-clj.cached-eval`: content-addressed memoization — the key is
  the POSITION-STRIPPED AST hash (M1's strip-positions) + a cache epoch,
  so whitespace/paren/span variants share one entry (true content
  addressing, not source-string equality). Follows the lowering-cache
  idiom (schemad key, clear!, stats). Guards all reused: M5
  static-purity-check (impure sources bypass), :ok-only, plain-data-only
  (callables never replayed) — a bypass is always evaluated fresh, so the
  cache can skip recomputation but never change an answer. Report:
  25-source mirror-pair corpus cross-check (miss→hit == fresh, value for
  value) + content-addressing/impure/held/closure checks, 29/29. Wired as
  `:cached-eval` artifact + aliases + gate step; capabilities regenerated.
  origin/main cas.clj deliberately not read (different design line; PORT
  reference only — branch reality).

With M6 the metacircular level roadmap M1–M6 is COMPLETE: specialize (→
JVM bytecode Futamura), tower collapse, reverse projection, capability
index + drift gate, safe-eval sandbox tier, cached-eval — pnix-hy's
pillars, pnix-clj's way (4-lane, gates, receipts, clj-meta). Follow-ups
live under each M-section; the ROAD block at the top stays the compass.

### (original M6 sketch)
- pnix-hy's `cached_eval`: memoize by CANONICAL CONTENT (purity + determinism
  make this sound). pnix-clj has hash/data-hash + parse cache keys already;
  add an eval-result cache keyed by (ast-hash, evaluator-version) for
  pure-verdict sources only; hit/miss receipts; report with determinism
  cross-check (cached value == fresh value on sampled corpus).
- NOTE branch reality: origin/main has cas.clj/store.clj (different design
  line). PORT ideas deliberately if useful; do not cherry-pick blind.

(check_action's role — accepted/held/rejected + witness — is already filled by
receipt/verdict + witness in pnix-clj; keep it, don't duplicate.)

Order: M1 (multi-slice) → M2 (1-2 slices) → M4 (1 slice) → M5 → M3 → M6.
Conformance/string work ONLY when one of these pillars needs it.

---

## 🔭 REMAINING WORK (after 2026-07-01 Nix-conformance hardening)

Written out per owner request. Context: the overnight/next-day hardening loop
(probe → fix → REPL → full gate → commit, no red commits) closed 13 real Nix
builtin/semantics bugs + the `rec` forward-reference fix, and landed operator
strict-audit Phase A. Current gate after the follow-up report/parity slices is
**96 tests / 2426 assertions**. What is LEFT is no longer easy single-lane
builtin bugs — it is evaluator-semantics / multi-lane-agreement /
fixture-taxonomy work.
Everything below keeps the same discipline: **never a silent negative→success
flip; leave a receipt/evidence trail; never commit red.**

### R1. Frontier-LIFT — recursive bindings across clj-meta / px-runtime lanes (DONE 2026-07-01)
- **Landed.** Valid `rec`/`let` forward references now reach all-lane `:ok`:
  `rec { x = y; y = 1; }.x`, `let a = b + 1; b = 10; in a`, and
  `rec { a = b; b = c; c = 5; }.a` all end at `:accepted
  :all-lanes-agree`.
- **clj-meta lane.** `pnix.clj-meta.compiler/lazy-letrec` is the pnix-agnostic
  recursive value-slot primitive. `src/pnix_clj/lowering.clj` now lowers pnix
  `let` and recursive attrsets to that form instead of sequential `let`.
- **.px runtime lane.** `pnixc-pnix/eval/evaluator.px` now registers simple
  `let` bindings and `rec` attr keys as thunk env slots before forcing RHS
  values; `Var`/`Select`/attr index force on read.
- **Receipt.** `resources/pnix_clj/forward_reference/cases.edn` is promoted from
  `:forward-reference-frontier-fixture-set` to
  `:forward-reference-lift-fixture-set`; `forward-reference-frontier-corpus`
  now asserts `:forward-ok` rows are all-lane accepted and cycle/unbound rows
  stay held.
- **Verification.** `clojure -M:test` → 71 tests / 1349 assertions; clj-meta
  `clojure -M:compiler-smoke` → 159/159; `clojure -M:conformance` → 116/116
  plus negatives 22/22.
- **Focused report.** Follow-up `pnix-clj.forward-reference` report/aliases
  (`clojure -M:forward-reference`, `clojure -M:report-forward-reference`) now
  record the R1 lift contract independently: 6 fixtures, 3 all-lane
  forward-ok rows, 3 semantic held rows, receipt hash recorded in the report.
- **Scope note.** R1 only lifted recursive binding visibility; the later R3
  core slice closed attrset/list element laziness separately.

### R2. Operator strictness Phases B–D (MEDIUM, phased, supervised)
- **State.** Phase A landed: `core/eval-source-strict-audit` records non-bool
  `if`/`assert`/`!` and `+` string↔non-string coercion with **zero behavior
  change** (`evaluator/*strict-audit*` defaults nil). Verified by
  `evaluator-strict-audit-records-without-changing-behavior`.
- **Phase B — DONE 2026-07-01.** `pnix-clj.strict-audit/report` classifies the
  source corpus as `:strict-ok`, `:strict-violation`, or `:held` with recorded
  event evidence. It covers fixture/oracle rows plus repo-owned runtime `.px`
  files by default. Artifact command:
  `clojure -M:report-strict-audit` → `target/pnix-clj/reports/strict-audit.edn`
  (68 sources, strict-ok 60, violations 0, held 8, events 0; artifact hash
  `bb69d7fcc1156da98d6a61ea5f7a730d2eb1bab107d32209f83bc5090b044344`).
  Gate: `clojure -M:test` → 72 tests / 1359 assertions.
- **Phase C — DONE 2026-07-01.** Added opt-in strict mode:
  `evaluator/*strict*`, `core/eval-source-strict`, and
  `clojure -M:strict-gate`. The Phase-A audit constructs now become held errors
  only under strict mode (`:non-bool-if-condition`,
  `:non-bool-assert-condition`, `:non-bool-not-operand`, `:string-coercion`).
  Default mode remains lenient. Strict gate over the default corpus:
  classified 68, checked 60, ok 60, failed 0. Gate:
  `clojure -M:test` → 74 tests / 1381 assertions.
- **Phase D — DONE 2026-07-07 (owner doctrine: "two languages, no blending").**
  The lenient default was reclassified as a CLOJURE HOST LEAK into the guest
  language, not a dialect choice — removed outright. Strict Nix typing is
  pnix's only semantics on every lane (evaluator default, lowering
  require-bool/plus guards, px already held). Measured before landing: 278
  corpus+runtime sources, strict violations 0. No lenient escape hatch
  remains; `eval-source-strict` survives as an explicit alias. Gate pin:
  `strict-semantics-default-across-lanes`.
- **Audit extension — DONE 2026-07-01.** Extended strict audit/strict mode to
  string builtins on non-strings (`stringLength`, `concatStringsSep`,
  `concatStrings`, prefix/suffix/infix helpers, case conversion, splitting,
  `replaceStrings`, regex `match`/`split`, etc.), `substring` negative start,
  and arithmetic non-number operands. Default lenient behavior is preserved;
  strict mode holds with `:string-builtin-non-string`,
  `:string-list-builtin-non-string-element`, `:substring-negative-start`, and
  `:arithmetic-non-number`. Current corpus remains clean:
  `clojure -M:strict-audit` → 226 sources, strict-ok 216, violations 0,
  held 10, events 0;
  `clojure -M:strict-gate` → classified 226, checked 216, ok 216, failed 0;
  `clojure -M:test` → 96 tests / 2426 assertions.

### R3. Lazy attrset / list *values* (HIGH, structural, supervised)
- **All-lane core slice — DONE 2026-07-01.** `let` bindings, function args,
  `rec` sibling scope, attrset VALUES, and list ELEMENTS are now call-by-need
  thunks in the semantic evaluator, the clj-meta lowering lane, and the internal
  `.px` runtime evaluator. Consumers force WHNF before inspection and public
  boundaries deep-realize final values for existing callers. Covered by
  `evaluator-lazy-collection-values-are-forced-on-demand` plus four all-lane
  `mirror-pair` receipts:
  `let s = { a = 1/0; b = 5; }; in s.b`/nested select succeeds, `builtins.head`
  avoids the tail, `builtins.length` avoids elements, and `builtins.elemAt`
  avoids unrelated elements.
- **Boundary fix included.** Report/runtime callers that inspect .px-produced
  values now normalize at their public boundary (`clojure_projection` validator
  result, `px_runtime` artifact/source execution summaries), and runtime source
  normalization runs on the same large-stack worker as source execution. Lazy
  slot thunks do not leak into report status checks or large rust-grounded
  receipts.
- **Still left.** Broaden beyond the current shape/ignored-value/equality
  slices: audit every remaining list/attrset builtin for exact Nix laziness
  (notably folds/maps that must actually inspect values, equality
  error-boundary receipts, and strict-vs-lazy edge cases). Current strict
  builtins still intentionally force-normalize inputs where Nix must inspect
  values.
- **Builtin broadening receipts 2026-07-01.** `take 0`, `drop`, `append`,
  `intersectAttrs`, and `zip` shape-only laziness are now promoted from probe
  evidence to all-lane `mirror-pair/lazy-*` receipts:
  `builtins.length (builtins.take 0 [ (1 / 0) ])`,
  `builtins.length (builtins.drop 1 [ (1 / 0) 2 ])`,
  `builtins.length (builtins.append [ (1 / 0) ] [ 2 ])`,
  `builtins.attrNames (builtins.intersectAttrs { a = 1; } { a = 1 / 0; b = 2; })`,
  and `builtins.length (builtins.zip [ (1 / 0) ] [ 2 ])`.
- **Ignored-value builtin lift 2026-07-01.** The clj-meta lowering lane and
  internal `.px` runtime lane now use WHNF collection forcing (not deep force)
  for `sort`, `mapAttrs`, `mapAttrsToList`, `filterAttrs`, `mapAttrs'`, and
  `zipListsWith` where the consumer/function may ignore element values. New
  all-lane `mirror-pair/lazy-*` receipts cover constant-comparator `sort`,
  value-ignoring `mapAttrs`/`mapAttrsToList`/`filterAttrs`, and constant
  `zipListsWith`.
- **Equality deep-comparison lift 2026-07-01.** clj-meta lowering now uses
  `pnix-clj.lowering/nix-equal` for `==`/`!=`, and the internal `.px` runtime
  uses `nix_equal` instead of raw host equality. Numeric int/float equality now
  matches recursively through lists/attrsets across all lanes; function equality
  stays false; equality forces inspected thunks. All-lane receipts:
  `mirror-pair/numeric-equality-*`.
- **`seq`/`deepSeq`/`toJSON` force receipts 2026-07-01.** clj-meta lowering now
  implements `seq` as WHNF forcing of the first arg and `deepSeq` as deep force
  before returning the second arg. The internal `.px` runtime mirrors that, and
  `deep_force_value` now eagerly visits attr values while preserving WHNF-only
  behavior for shape-only `filterAttrs`/`intersectAttrs`. New all-lane receipts:
  `mirror-pair/seq-list-whnf-lazy`, `seq-attrset-whnf-lazy`,
  `deep-seq-attrset-ok`, `to-json-attrset`, and `to-json-list`; negative probes
  confirm `builtins.seq (1 / 0) 2` and `builtins.deepSeq { a = 1 / 0; } 2`
  hold in evaluator, clj-meta, and `.px`.
- **List higher-order ignored-input lift 2026-07-01.** clj-meta lowering and
  the internal `.px` runtime now WHNF-force input lists for higher-order
  builtins whose callback may ignore an element: `map`, `filter`, `concatMap`,
  `all`, `any`, `count`, `foldl'`/`foldl`, `foldr`, `findFirst`, `groupBy`,
  `imap0`, `imap1`, plus the existing string-map helpers. New all-lane
  `mirror-pair/lazy-*-ignored-input` receipts cover each promoted case. Nix
  probe kept `partition` out of this success set because real Nix forces enough
  to fail on `[ (1 / 0) ]` even when the predicate body ignores the value.
- **Attr/list shape-producer lift 2026-07-01.** clj-meta lowering and `.px`
  runtime now preserve lazy attr/list values for `listToAttrs`, `zipAttrsWith`,
  `catAttrs`, and `foldlAttrs`; row/name/key positions are forced only as needed
  while value slots remain lazy. Added all-lane receipts:
  `mirror-pair/lazy-list-to-attrs-attrnames`,
  `lazy-zip-attrs-with-attrnames`, `lazy-cat-attrs-length`,
  `lazy-foldl-attrs-ignored-value`, and `lazy-map-attrs-prime-attrnames`.
- **Strictness-vs-laziness correction 2026-07-01.** `builtins.partition` now
  matches Nix by forcing list elements before predicate classification; even a
  value-ignoring predicate (`x: true` / `x: false`) holds on `[ (1 / 0) ]`.
  clj-meta/`.px` already held there; the evaluator was the permissive lane.
- **`genAttrs` lazy-value lift 2026-07-01.** pnix `builtins.genAttrs` now stores
  value results as lazy slots in evaluator and clj-meta lowering, matching the
  `.px` runtime behavior. `builtins.attrNames (builtins.genAttrs ["a"] (name:
  1 / 0))` is all-lane accepted, while selecting `.a` still holds.
- **`elem` equality/laziness lift 2026-07-01.** clj-meta lowering now uses
  `nix-equal` for `builtins.elem`, and `.px` searches sequentially instead of
  deep-forcing the whole list. Receipts: `builtins.elem 1 [ 1.0 ]` and
  `builtins.elem 1 [ 1 (1 / 0) ]` are all-lane accepted. Direct evaluator and
  clj-meta also return `false` for Nix's empty-list lazy-needle edge
  (`builtins.elem (1 / 0) []`); `.px` now handles the direct
  `builtins.elem <needle>` application lazily too. All-lane receipt:
  `mirror-pair/lazy-elem-empty-list-needle`.
- **`attrValues`/`values` lazy-value lift 2026-07-02.** `.px` runtime no longer
  deep-forces attr values when producing `builtins.attrValues` / `builtins.values`
  lists. Length and selected later elements avoid unrelated attr thunks across
  all lanes. Receipts: `mirror-pair/lazy-attr-values-length`,
  `lazy-attr-values-elem-at`, and `lazy-values-alias-length`.
- **`concatLists` inner-element laziness lift 2026-07-02.** clj-meta lowering
  and `.px` runtime now force only the outer list and each inner list shape,
  preserving lazy inner elements. Receipts:
  `mirror-pair/lazy-concat-lists-length` and
  `mirror-pair/lazy-concat-lists-elem-at`; selecting the failing concatenated
  element still holds.
- **`reverseList` shape-only lift 2026-07-02.** clj-meta lowering now WHNF-forces
  only the input list shape before reversing, preserving element thunks. The
  evaluator and `.px` runtime already had this behavior. Receipts:
  `mirror-pair/lazy-reverse-list-length` and
  `mirror-pair/lazy-reverse-list-elem-at`; selecting the reversed failing
  element still holds. `reverseList` is a pnix extension here (Nix 2.34 has no
  such builtin), so this receipt records pnix lane agreement rather than upstream
  Nix conformance.
- **List accessor guard lift 2026-07-02.** clj-meta lowering now rejects
  non-list and empty-list inputs for `head`, `tail`, `last`, `init`, and
  `length` instead of silently producing host `nil`/chars/vectors. The evaluator
  now also rejects `builtins.last "abc"` with `:last-not-list`, and `.px`
  `init []` now holds instead of returning `[]`. Regression test:
  `evaluator-list-builtin-guards` checks evaluator/clj-meta/.px all hold for
  the empty/non-list guard matrix.
- **`unique` singleton laziness lift 2026-07-02.** clj-meta lowering and `.px`
  runtime now force only the input list shape and compare new elements with
  `nix-equal` on demand. A singleton error element can contribute list shape
  without being forced: `mirror-pair/lazy-unique-singleton-length`. Selecting
  that element, or comparing a later error element against previous output, still
  holds.
- **`find` equality/laziness lift 2026-07-02.** clj-meta lowering now uses a
  sequential `find-value` helper with `nix-equal`, and `.px` no longer
  deep-forces the whole search list. Receipts:
  `mirror-pair/find-numeric-equality` and
  `mirror-pair/lazy-find-stops-before-tail-error`. A miss that reaches an error
  element still holds, and `builtins.find (1 / 0) []` remains evaluator-strict.
- **`optionals` true-list shape lift 2026-07-02.** `.px` runtime now returns
  the selected list at WHNF instead of deep-forcing its elements when the
  condition is true. Receipt: `mirror-pair/lazy-optionals-true-length`.
  Selecting the error element still holds; `optional` remains evaluator-strict
  for its value argument.
- **Already-implemented aggregator receipts 2026-07-02.** Promoted existing
  all-lane behavior to explicit receipts: `zipAttrsWith` can pass a values list
  whose element is an error to a callback that only checks `builtins.length`
  (`mirror-pair/lazy-zip-attrs-with-values-length`), and nested
  `recursiveUpdate` preserves an unrelated lazy sibling while selecting the
  merged key (`mirror-pair/lazy-recursive-update-nested-select`).
- **`optional`/`optionalString` false-branch laziness lift 2026-07-02.** The
  evaluator now treats the second arg of `optional`/`optionalString` as lazy
  after the condition is known, clj-meta wraps `optional true` values in lazy
  slots, and `.px` has direct Apply handling so the skipped branch is never
  evaluated. Receipts: `mirror-pair/lazy-optional-false-value`,
  `lazy-optional-true-length`, and `lazy-optional-string-false`; forcing the
  selected error value still holds.
- **Generated-list value laziness lift 2026-07-02.** `builtins.genList` now
  returns lazy element thunks in the evaluator and `.px` runtime, and clj-meta
  lowering wraps each generated value in a lazy slot. `builtins.replicate` now
  treats the replicated value as a lazy position in evaluator/lowering and `.px`
  direct Apply, so list shape can be observed without forcing the value.
  Receipts: `mirror-pair/lazy-gen-list-length` and
  `mirror-pair/lazy-replicate-length`; `elemAt` of the generated/repeated error
  element still holds.
- **Higher-order result-value laziness lift 2026-07-02.** `map`, `imap0`,
  `imap1`, and `zipListsWith` now preserve lazy result elements across the
  evaluator, clj-meta lowering, and `.px` runtime. Function targets are still
  checked immediately, but the callback result is stored as a thunk/lazy slot
  until an element is selected. Receipts: `mirror-pair/lazy-map-result-length`,
  `lazy-imap0-result-length`, `lazy-imap1-result-length`, and
  `lazy-zip-lists-with-result-length`; the corresponding `elemAt` probes still
  hold when they select the error result.
- **Attr higher-order result-value laziness lift 2026-07-02.** `mapAttrs` and
  `mapAttrsToList` now store callback results lazily in evaluator/clj-meta/`.px`
  while still checking the callback target immediately. `mapAttrs'` already kept
  the returned pair's `value` lazy after forcing only the result pair and `name`;
  that behavior is now receipt-backed. Receipts:
  `mirror-pair/lazy-map-attrs-result-attrnames`,
  `lazy-map-attrs-to-list-result-length`, and
  `lazy-map-attrs-prime-result-attrnames`; selecting the error value still
  holds.
- **`zipAttrsWith` result-value laziness lift 2026-07-02.** `zipAttrsWith`
  already preserved lazy input values in the callback values-list; now the
  callback result itself is stored as a lazy attr value across evaluator,
  clj-meta lowering, and `.px`. Receipt:
  `mirror-pair/lazy-zip-attrs-with-result-attrnames`; selecting `.a` still
  holds when the callback returns `1 / 0`.
- **Higher-order lazy traversal receipt slice 2026-07-02.** Added all-lane
  receipts for already-supported traversal boundaries where a builtin may
  inspect list/attr shapes, predicate truth, or result length without forcing
  skipped tail/input values: `lazy-map-identity-head`,
  `lazy-filter-true-head`, `lazy-concat-map-identity-head`,
  `lazy-sort-constant-comparator-length-two`,
  `lazy-filter-attrs-true-attrnames`,
  `lazy-map-attrs-identity-values-length`,
  `lazy-zip-attrs-with-head-values-length`, and
  `lazy-concat-lists-map-result-length`.
- **Dynamic attr key `.px` parser parity lift 2026-07-02.** The internal `.px`
  source parser now accepts bare dynamic attr keys (`{ ${expr} = value; }`) and
  lowers them to the evaluator's existing dynamic-key AST shape. Receipts:
  `mirror-pair/dynamic-attr-key-select`,
  `dynamic-attr-key-attrnames`, and
  `dynamic-attr-key-merge-attrnames`. This was a parser parity hole surfaced
  while probing remaining attr/laziness surfaces.
- **Dynamic select/hasAttr `.px` parser parity lift 2026-07-02.** The internal
  `.px` source parser now accepts bare dynamic select and has-attr forms
  (`set.${expr}`, `set ? ${expr}`), handles dynamic select defaults
  (`set.${expr} or default`), and treats string-template has-attr keys as
  dynamic instead of literal. Receipts: `mirror-pair/dynamic-select`,
  `dynamic-has-attr`, `dynamic-select-default`, and
  `dynamic-string-template-has-attr`.
- **Why hard.** True laziness means storing thunks IN data structures, so EVERY
  consumer (all list/attrset builtins, `//`, select, ==, toJSON, …) must
  force-on-read. Large surface. Also entangled with the mirror-error/laziness
  corpora and the frontier lanes (which are eager).
- **Gate receipt.** `clojure -M:test` → 96 tests / 2426 assertions;
  `clojure -M:strict-audit` → 226 sources, strict-ok 216, violations 0,
  held 10, events 0;
  `clojure -M:strict-gate` → classified 226, checked 216, ok 216, failed 0;
  `clojure -M:mirror-pair` → 147 accepted after the later path/interpolation/
  absolute-path/lazy-shape/ignored-value/select-or/equality parity fixtures;
  `clojure -M:rust-batch` → 10 accepted.

### R4. CAS / store PORT from origin/main (MEDIUM, MAIN-ONLY asset)
- `origin/main` is a **different design line** (cas.clj / store.clj / stage.clj /
  purity.clj / gate-graph / 67-lang). It is a MAIN-ONLY reference asset, NOT a
  missing pillar of this branch. If content-addressed term storage is wanted on
  `feat/clj-meta-metacircular`, PORT the pieces deliberately (read main's design,
  reimplement to this branch's value model), don't cherry-pick blind. Confirm on
  the branch before claiming absence. See memory `pnix-clj-branch-reality`.

### R5. Singleton `run-mirror` (DONE 2026-07-01, architecture)
- Implemented `pnix-clj.mirror/run-mirror` as the single runtime mirror
  entrypoint with explicit facets:
  `:host/parse`, `:host/eval`, `:host/lower`, `:host/clj-meta`,
  `:inner/stage15-control`, `:inner/px-runtime`, `:inner/pnix-mirror`,
  `:cross/value-agreement`.
- `core/run-source` now calls `mirror/run-mirror` once and takes
  `:clojure-mirror`, `:px-runtime`, `:pnix-mirror`, and
  `:cross-mirror-verdict` from that receipt. The legacy top-level receipt fields
  are preserved for compatibility, with `:mirror-run` added as the owner receipt.
- Row constructors (`clojure-mirror-row`, `px-runtime-row`, `pnix-mirror-row`,
  `cross-mirror-verdict-row`) remain as implementation helpers under the one
  entrypoint; interop remains separate.
- Gate receipt: `clojure -M:test` → 75 tests / 1416 assertions;
  `clojure -M:mirror-pair` → 9 accepted / 0 held.

### R6. Interop wiring into real host crossings (DONE 2026-07-01, supervised)
- Live host crossings now go through the interop gate:
  - `interop/host-eval-form` is deny-by-default; `clojure-form` must pass the
    explicit `interop/host-eval-capabilities` grant. Every success/denial emits
    a `:pnix-clj.interop.witness.v0` witness.
  - `clj-meta/eval-lowered` checks the explicit `:host-compile` capability grant
    and attaches a witness to every clj-meta compile/eval result.
  - `clojure_projection.clj` host-term fixture helpers now run via
    `interop/run-crossing`, with explicit effect classes for host-eval,
    macroexpand, dynamic-binding, host-call, reflection, classloader,
    resolve-var, global-mutation, and thread. The projection term schema stays
    unchanged; rows carry `:host-crossing` evidence.
- `:host-compile` is now part of the closed effect-class taxonomy. Default
  capabilities remain `#{:pure}`; projection-only effects are granted only inside
  the projection proof fixture path.
- Gate receipt: `clojure -M:test` → 76 tests / 1431 assertions;
  `clojure -M:clojure-form` → 53 accepted / 0 held;
  `clojure -M:mirror-pair` → 9 accepted / 0 held.
  Follow-up projection slice: `clojure -M:test` → 76 tests / 1434 assertions;
  `clojure -M:clojure-projection` → 43 accepted / 0 held.

### Discipline reminder for all of the above
Small slice → check for existing impl first → REPL-verify → **full gate green** →
commit + push. Cross-lane changes: keep `mirror-error` (error agreement) and
`mirror-pair` (success agreement) corpora green; reclassify fixtures only with a
written receipt (see `rec-forward-reference-taxonomy.md` as the template). The
evaluator lane may run ahead of the clj-meta/px frontier lanes — that divergence
is already tolerated (e.g. `let`-forward) as long as it is recorded as a declared
frontier, not flagged as an error-agreement.

---

## Current Phase

Phase: Clojure/Java/JVM ecosystem <-> pnix expression projection through
internal runtime/mirrors

Target loop:

```text
pnix source / runtime fixture / Clojure-JVM projection fixture
-> pnix-clj parser / AST
-> pnix-clj semantic evaluator
-> clojure mirror of host/clj-meta meaning
-> Clojure form lowering
-> ../clj-meta compile/eval
-> JVM execution / Java interop / bytecode receipt
-> pnix runtime (.px) artifact / behavior
-> pnix mirror self-observation
-> mirror/evidence/admission
-> pnix result / error / held reason
```

This is not "generate Clojure text and hope it runs". The runtime must keep
an evidence chain from source to AST to semantic result to clj-meta compiled
result to `.px` runtime behavior to pnix mirror receipt. A single lane result is
evidence, not truth.

Do not turn this phase into cross-host parity work. The host language to project
here is the Clojure/Java/JVM ecosystem surface only. Hy/Python projection
belongs to `pnix-hy`.

Do not turn this phase into AI-agent work. The implementation target is not
"make an agent that writes code" or "revive autonomous pnix"; it is to expose,
test, and document Clojure/Java/JVM ecosystem <-> pnix language projection
behavior so a human developer can study the meta-circular surfaces directly.

Do not start broad Clojure projection work until the internal pnix runtime and
the clojure-mirror/pnix-mirror pair are stable enough to generate receipts for
the runtime itself. The next engineering priority remains runtime and mirror
completion.

That precondition is now met (the runtime/mirror spine generates stable
receipts: smoke/mirror-pair/mirror-error/projection/rust/stage7 all green). The
forward plan for raising completeness from here lives in
`## Completeness Roadmap (research-grounded, 2026-06-30)` below — read it after
this section. Its priority verdict: Axis 1 (language semantic depth, laziness
first) and Axis 4 (conformance/differential corpus) are the real gaps; host
projection and the runtime/mirror are already role-complete.

---

## Ownership Map

```text
pnix-clj parser/AST
  owns pnix syntax shape and source spans

pnix-clj evaluator
  owns pnix language semantics and value model

pnix-clj lowering
  owns pnix AST -> Clojure form projection

clojure mirror
  owns host/clj-meta observation of forms, bytecode, evaluation, determinism,
  fallback, and compiler receipts

Clojure/Java/JVM ecosystem <-> pnix projection
  owns the language-expressiveness research surface: reader data, forms,
  functions, macros, namespaces, vars, dynamic binding, Java interop, Java
  classes/constructors/methods/fields, reflection, classloader behavior,
  bytecode evidence, JVM objects, exceptions/control flow, runtime effects,
  and their pnix representation.
  It does not own autonomous agent behavior.

../clj-meta compiler/evaluator
  owns Clojure form analysis, direct emit, eval, bytecode, compiler receipts

pnix runtime (.px)
  owns the repo-internal runtime artifacts under resources/pnix_clj/pnix_runtime
  that must remain expressible in pnix, not only in host Clojure scaffolding

pnix mirror
  owns pnix-side self-observation of the .px runtime and its behavior

mirror/evidence/admission
  owns equivalence claims between clojure mirror, evaluator, lowered form,
  compiled execution, .px runtime behavior, pnix mirror receipts,
  bytecode/determinism receipts, and repo-owned copied oracle rows

~/pnix-old original implementation
  is archival provenance for already-copied fixtures. It is not a runtime,
  test, report, or gate dependency; normal pnix-clj commands must succeed when
  that directory is absent.
```

`clj-meta` is a dependency and a proof/evidence substrate. It is not copied into
this directory and it is not reimplemented here.

---

## What Changed From pnix-hy

The copied `pnix-hy` checklist used the wrong implementation model for this
repository. These names are invalid in the new plan unless they appear only in
historical notes:

```text
Python host interpreter
Hy evaluator/compiler source lanes
HY_AST_EVALUATOR_SOURCE / CLJ_AST_EVALUATOR_SOURCE string injection
run_px / compile_px_source as Python-style entry points
Thunk / NativeFunc / PnixString Python classes
py_compile / venv / CLOJUREPATH verification recipes
pnix_mirror.self_test_report copied from pnix-hy
```

Replacement model:

```text
Clojure data AST
Clojure value model
pnix AST -> Clojure form lowering
clojure mirror -> pnix runtime (.px) -> pnix mirror
clj-meta compile-form / eval-form / compile-fn-strict / compile-ns
clj-meta bytecode/determinism/conformance/gate receipts
pnix-clj mirror receipt built for this repo
```

Stage invariant:

```text
clj-meta stage15..N compiler/evaluator evidence
-> clojure mirror receipt
-> pnix runtime (.px) behavior
-> pnix mirror receipt inside that runtime
```

`clj-meta` success is necessary backend evidence, not final pnix truth. A case
is still held until it can cross the internal `.px` runtime and pnix mirror
boundary.

Do not keep Hy/Python implementation steps around as unchecked TODOs. They create
false continuity and make later agents implement the wrong project.

What is preserved from `pnix-hy/hy-meta`:

- same staged closure direction
- same mirror discipline
- same runtime-in-pnix destination
- same corpus/oracle pressure, but through repo-owned copied `.px` artifacts
- same rule that a host-language success is not the final pnix truth
- same need to keep implementation receipts instead of vague claims

What is not preserved:

- Python objects/classes
- Hy source staging
- Hy/Python verification commands
- copied function names that imply the wrong runtime
- string-injected evaluator/compiler blobs as the main architecture

---

## clj-meta Backend Facts

Source root:

```text
~/pnix-clj/clj-meta/src/pnix/clj_meta
```

Primary compiler API lives in:

```text
../clj-meta/src/pnix/clj_meta/compiler.clj
```

Expected public/product surface:

```text
compile-form
eval-form
compile-fn-strict / compile-form-strict when exposed
compile-ns
load-compiled-ns
compile-classes
verified compile / bytecode witness lanes
```

Important clj-meta gates from `../clj-meta/deps.edn`:

```text
clojure -M:compiler-smoke
clojure -M:conformance
clojure -M:fuzz-conformance
clojure -M:gate
clojure -M:bytecode-witness
clojure -M:translation-validation
clojure -M:lowering-admission
clojure -M:bytecode-verifier
clojure -M:verified-compile
clojure -M:determinism-policy
clojure -M:audit-self-source
clojure -M:full-source-stage1
```

Honest boundary:

- `clj-meta` gives compiler/evaluator evidence for a Clojure subset and JVM
  bytecode path.
- It does not prove all of Clojure, all host behavior, or all pnix semantics.
- `tools.analyzer.jvm`, host Clojure reader/analyzer behavior, JVM class loading,
  and selected runtime helpers remain trusted/held frontiers unless a specific
  receipt says otherwise.
- `pnix-clj` must not claim full Wheeler DDC or language correctness from a
  single clj-meta gate.

---

## Architecture

### Lane 1 - pnix source to AST

Goal: parse pnix source into Clojure data with stable spans and hashes.

Required records:

```text
source-path or source-id
source-hash
AST
AST hash
span map
unsupported syntax rows
```

Tasks:

- [x] Create `deps.edn` for `pnix-clj` with a local/root dependency on `../clj-meta`.
      `pnix-clj/deps.edn` exists and the full gate runs through the local
      clj-meta lane.
- [x] Establish namespace layout under `src/pnix_clj/...`.
      Current layout includes parser/evaluator/lowering/core/clj_meta/mirror/
      px_runtime/strict_audit/report modules plus projection subnamespaces.
- [x] Define pnix AST nodes as Clojure data, not source-string fragments.
      `parser/parse-source` returns map AST nodes; lowering keeps
      `:source-string-codegen? false`.
- [x] Keep source span and normalized hash on every parsed top-level form.
      Parser AST nodes carry `:span`; `run-source`/`compile-source` attach
      `:source-hash` and `:ast-hash`.
- [x] Build a small parser smoke corpus before importing/copying large `.px`
      corpus slices. `parser-literal-smoke`, smoke reports, stage7/mirror-pair,
      and rust-grounded slices are all in the gate.
- [x] Mark unknown grammar as `:held` / `:unsupported-syntax`, not silent fallback.

### Lane 2 - pnix semantic evaluator

Goal: implement pnix semantics directly in Clojure so compiled output has an
independent semantic lane to compare against.

Required records:

```text
AST hash
environment hash
result value or structured error
forced/lazy boundary if any
trace rows for imports, builtins, and strictness decisions
```

Tasks:

- [x] Define the pnix value model in Clojure.
      `evaluator.clj` covers scalar/list/attrset/callable/thunk values; domain
      extension stubs are explicitly non-Nix coverage.
- [x] Define structured error values before adding many builtins.
      `pnix-clj.error` now fixes `:pnix-clj.error.v0`; evaluator held/error
      paths keep their legacy top-level reason fields while attaching a
      machine-readable phase/reason/details envelope.
- [x] Implement the minimal expression core first: literals, vars, let, if,
      attr/list basics, function/call, selected builtins. Current gate covers
      parser/evaluator/lowering/runtime lanes plus a broad builtin corpus.
- [x] Preserve laziness/strictness as explicit evaluator behavior, not accidental
      host Clojure behavior. Let/function args/collection slots are thunked;
      strict audit/strict mode and overflow guards pin host-behavior gaps.
- [x] Add cycle/error receipts where pnix semantics require them.
      Internal `.px` runtime import evaluation now exposes cache/cycle receipts;
      recursive import cycles are held as `:px-runtime-import-cycle`.
- [x] Keep evaluator result comparison deterministic and printable.
      Receipts compare evaluator/clj-meta/px/pnix-mirror values with stable
      hashes, lane summaries, and structured held reasons.

### Lane 3 - pnix AST to Clojure form lowering

Goal: lower pnix AST into Clojure forms that `clj-meta` can analyze and compile.

Required records:

```text
AST hash
lowered Clojure form
lowered form hash
lowering policy version
unsupported-op or held reason
```

Tasks:

- [x] Start with a tiny supported lowering subset and reject the rest honestly.
      Unsupported lowering returns structured `:held` reasons; supported corpus
      has grown case-by-case with receipts.
- [x] Lower to Clojure forms/data, not generated source strings, unless a source
      string is explicitly required by a clj-meta API. Lowering receipts keep
      `:source-string-codegen? false`.
- [x] Use stable generated names derived from source/AST content, not global
      counters that break deterministic receipts. Lowering cache keys are
      `{:ast-hash ... :policy ...}` and clj-meta receipts are repeat-compiled.
- [x] Add type/strictness annotations only when the evaluator lane already has the
      matching semantics. Strict audit/strict mode are opt-in and backed by
      evaluator tests.
- [x] Do not let host Clojure truthiness, numeric overflow, or collection behavior
      silently replace pnix semantics. Strict mode rejects audited truthiness/type
      gaps; integer overflow now returns structured `:integer-overflow`.

### Lane 4 - clj-meta compile/eval lane

Goal: execute lowered forms through `../clj-meta` and compare them with the
pnix-clj evaluator. This lane feeds the clojure mirror; it is not the final pnix
runtime by itself.

Required records:

```text
lowered form hash
clj-meta API used
direct emit vs host fallback status
bytecode hash when available
runtime result or structured compile error
clj-meta gate receipts relevant to this feature
```

Tasks:

- [x] Wire `pnix-clj` to call `pnix.clj-meta.compiler/eval-form` for simple
      forms.
      `pnix-clj.clj-meta/eval-lowered` now uses public
      `pnix.clj-meta.compiler/eval-form` for the execution value, then keeps the
      existing `compile-form*`/`form-proof/compile-receipt` path as evidence and
      asserts both values agree. Result maps record `:execution-api`,
      `:evidence-apis`, and `:api-values-agree?`. Verified by
      `clojure -M:test` → 83 tests / 2035 assertions.
- [x] Wire strict compilation for forms that must not use host fallback.
      clj-meta compile receipts include strict direct evidence.
- [x] Capture compile diagnostics and fallback status when the API exposes them.
      `clj_meta/eval-lowered` records mode, diagnostics, fallback status, and
      bytecode evidence.
- [x] Compare evaluator result vs clj-meta result before marking a case accepted.
      `receipt/verdict` requires lane agreement for `:accepted`.
- [x] Add bytecode/determinism evidence for lowered forms that become production
      runtime paths.
      Compile receipts now include primary/repeat determinism, strict direct
      evidence, and a `compile-to-dir` bytecode artifact hash when clj-meta can
      emit class files.
- [x] Treat compile failure as `:held` or structured error, not as a parser or
      evaluator success. clj-meta compile/eval exceptions become
      `:clj-meta-eval-failed` / held receipts.

### Lane 5 - clojure mirror -> pnix runtime (.px) -> pnix mirror

Goal: preserve the original mirror spine. Clojure/clj-meta observation must flow
into a `.px` runtime artifact, and pnix must mirror that artifact back in pnix
terms.

Required records:

```text
clojure mirror row
clj-meta compile/eval receipt
.px runtime source/artifact hash
pnix runtime behavior trace
pnix mirror row
cross-mirror equivalence verdict
```

Tasks:

- [x] Define what a clojure mirror row means in this repo: form, analyzer/mirror
      data, compile mode, bytecode hash, eval result, and held frontier.
      Current row records lowered form/hash, clj-meta mode/diagnostics/value, and
      stage15 control metadata. The lane is ok when evaluator and clj-meta agree;
      unexecuted stage15 gates remain attached control frontier, not the first
      runtime blocker.
- [x] Define the `.px` runtime artifact boundary: which runtime pieces must be
      representable as pnix source/data rather than only Clojure helper code.
      `px-runtime/runtime-boundary` pins allowed repo-owned roots, entry artifact,
      and the no-parent-checkout runtime dependency invariant.
- [x] Keep the normal runtime root inside `resources/pnix_clj/pnix_runtime`,
      not in parent checkout directories.
- [x] Define the pnix mirror row: how the `.px` runtime reports its own behavior
      back as pnix evidence.
      Current row is held until runtime execution, but records source/AST hashes
      and px-runtime status/reason as the pnix-side mirror boundary.
- [x] Add a cross-mirror verdict: clojure mirror and pnix mirror agree, disagree,
      or held with reason.
- [x] If a feature cannot pass through `.px` runtime expression, keep it held even
      when the Clojure evaluator and clj-meta lane agree.
      Internal `vm.px` + P1-P12 bootstrap now parses/evaluates from repo-owned
      `.px` artifacts. Small source expressions now route through
      `pnixc-pnix/exec/runtime.px` + `pnixc-pnix/eval/evaluator.px`; unsupported
      runtime evaluator cases stay held as `:px-runtime-run-held`.

### Lane 6 - mirror, evidence, admission

Goal: one source case produces one durable receipt that tells which lanes agreed.

Receipt shape:

```clojure
{:source-id ...
 :source-hash ...
 :ast-hash ...
 :eval-result ...
 :lowered-form-hash ...
 :clojure-mirror ...
 :clj-meta-result ...
 :px-runtime-hash ...
 :pnix-mirror ...
 :cross-mirror-verdict ...
 :oracle-result ...
 :bytecode-hash ...
 :status :accepted | :rejected | :held
 :reason ...
 :receipts [...]}
```

Tasks:

- [x] Create the receipt schema before growing the language surface.
- [x] Implement `accepted` only when required lanes agree.
- [x] Preserve `rejected` for real mismatches.
- [x] Preserve `held` for unsupported syntax, unsupported lowering, clj-meta
      frontier, missing oracle, or non-deterministic evidence.
- [x] Add a report command that prints accepted/rejected/held counts and the first
      concrete mismatch.

---

## Verification Policy

For doc-only edits in this repository, no runtime gate is required.

Before claiming a pnix-clj code feature works:

```text
1. Run the pnix-clj test/report command once it exists.
2. Run relevant clj-meta gates from ../clj-meta.
3. Compare evaluator lane vs clj-meta lane.
4. Compare clojure mirror vs `.px` runtime vs pnix mirror.
5. Compare with repo-owned copied oracle rows when the feature has imported
   corpus coverage. Refresh copied rows manually outside normal runtime/report
   commands; never make a report read parent/original checkouts.
6. Record held reasons instead of widening claims.
```

Minimum clj-meta backend health check before depending on a new compiler feature:

```sh
cd ../clj-meta
clojure -M:compiler-smoke
clojure -M:conformance
clojure -M:gate
```

Escalate to these when touching lowering, bytecode, determinism, or production
runtime paths:

```sh
cd ../clj-meta
clojure -M:fuzz-conformance
clojure -M:bytecode-witness
clojure -M:translation-validation
clojure -M:lowering-admission
clojure -M:bytecode-verifier
clojure -M:verified-compile
clojure -M:determinism-policy
clojure -M:audit-self-source
clojure -M:full-source-stage1
```

Do not port the old `pnix-hy` verification commands unless the matching
implementation exists in this repository.

---

## Immediate Next

1. Repository bootstrap
   - [x] Add `deps.edn`.
   - [x] Add `src/pnix_clj/...` and `test/pnix_clj/...`.
   - [x] Add local dependency path to `../clj-meta`.
   - [x] Add one smoke command that proves the dependency/runtime path loads.

2. Core AST and receipt schema
   - [x] Define source record, AST hash, and receipt schema.
   - [x] Add one literal expression parsed into Clojure data.
   - [x] Add a report function that can print `accepted/rejected/held` even before
         the language is broad.
   - [x] Include `:clojure-mirror`, `:px-runtime-hash`, and `:pnix-mirror` slots
         from the start.

3. Evaluator first slice
   - [x] Implement literals and a tiny expression evaluator.
   - [x] Add first structured held/error result maps.
   - [x] Add deterministic literal result comparison.

4. clj-meta compile lane first slice
   - [x] Lower the same literal/core expressions to Clojure forms.
   - [x] Evaluate them through `pnix.clj-meta.compiler/compile-form*`.
   - [x] Emit the clojure mirror row and compare evaluator result vs clj-meta
         result in one receipt.
   - [x] Add stage15 control-plan receipt for clj-meta backend hashes/gates.

5. `.px` runtime / pnix mirror first slice
   - [x] Define the first `.px` runtime artifact boundary, even if tiny.
   - [x] Define the first pnix mirror row for that artifact.
   - [x] Mark cases held until they can cross clojure mirror -> `.px` runtime ->
         pnix mirror.
   - [x] Add human-trackable runtime run-plan/import graph for
         `pnix-mirror-runtime/vm.px`.

6. Internal `.px` runtime / corpus first slice
   - [x] Inventory repo-owned `.px` runtime artifacts.
   - [x] Copy missing runtime `.px` artifacts into this repo as a one-time
         development import, then run from the internal copy only.
   - [x] Move normal runtime inventory to `resources/pnix_clj/pnix_runtime`.
   - [x] Store oracle rows as repo-owned data fixtures, not as a runtime
         dependency on any external pnix checkout.
   - [x] Mark missing or unsupported original behavior as `:held`.

7. Rust-grounded batch pressure
   - [x] Copy Rust-grounded invariance corpus rows into repo-owned
         fixtures.
   - [x] Add batch report that runs those fixtures through current pnix-clj
         receipt path.
   - [x] Add manifest for `RUST_*_CORPUS` + `STAGE7_CORE_CASES` authority suites.
   - [x] Add lane-summary/frontier reason counts before growing Rust suites.
   - [x] Pin copied fixture source revision/hash metadata in the manifest/report.
   - [x] Import Rust-grounded expected outputs instead of held oracle rows.
         All 10 now have repo-owned expected rows. `c10_mixed.px` preserves the
         captured Rust `pnixc-meta` parse-error as historical evidence while
         admitting the pnix-clj evaluator/clj-meta/internal `.px` runtime/pnix
         mirror agreed value as the current projection oracle.
   - [x] Import `RUST_EVAL_CORPUS`, `RUST_BUILTIN_CORPUS`, and
         `RUST_OVERFLOW_CORPUS` as static repo-owned data.
         Source-level Rust suite copies now live under
         `resources/pnix_clj/rust_grounded/suite_sources`; report records
         repo-owned hashes and 91 imported Rust `#[test]` functions. Executable
         pnix-clj case extraction remains a separate frontier.
   - [x] Add stage7 core lock-ins as repo-owned fixture cases.
         First clj-meta-compatible slice is
         `resources/pnix_clj/stage7_core/cases.edn` with 5 pnix-hy structural
         core cases. Report is wired through `clojure -M:stage7-core` and
         `clojure -M:report-stage7-core`; all 5 now cross evaluator,
         clj-meta, internal `.px runtime`, and pnix mirror as accepted.
   - [x] Drive first parser/evaluator/lowering expansion against
         `c01_arith.px`; now accepted through internal `.px` runtime.
   - [x] Drive attr path/merge/builtin first slice against `c04_attr.px`;
         now accepted through internal `.px` runtime.
   - [x] Drive non-recursive if/lambda/map first slice against
         `c08_bool.px` and `c09_lambda.px`; now accepted through internal `.px`
         runtime.
   - [x] Drive list builtin first slice against `c07_builtins.px`;
         now accepted through internal `.px` runtime.
   - [x] Drive recursive function first slice against `c05_recurse.px`;
         now accepted through internal `.px` runtime with recursive direct-lambda
         closure environment reconstruction and fixed-stack source execution.
   - [x] Drive list concat/fold/filter/length slice against `c03_list.px`;
         now accepted through internal `.px` runtime.
   - [x] Drive string interpolation/string builtins/JSON slice against
         `c02_strings.px` and `c06_nested.px`; both now accepted through
         internal `.px` runtime.
   - [x] Drive dynamic attr key / integer division slice against `c10_mixed.px`;
         evaluator, clj-meta, internal `.px` runtime, and pnix mirror now agree.
         The old Rust parse-error is provenance evidence, not the current
         semantic frontier.
   - [x] Close the current Rust-grounded batch frontier: 10/10 fixtures accepted.
   - [ ] Continue parser/evaluator/lowering expansion against the next imported
         projection fixture batch.

---

## Correctness Audit (2026-06-30)

A focused audit of host-faithfulness (the pnix evaluator/lowering lanes) and of
the clojure-mirror gates found and fixed the defects below. Each fix leaves the
existing fixtures unchanged (they only used inputs where old == new behavior) and
the full `bin/pnix-clj-gate`, `clojure -M:test`, and `clojure -M:benchmark` stay
green. Behavior changes were verified directly through `eval-source`.

- [x] `clojure-form` classified a genuine host-eval vs clj-meta value
      disagreement (both lanes succeed, values differ) as `:held` instead of
      `:rejected`, contradicting the `receipt/verdict` policy. `case-row` now
      rejects when every lane produced a value but they disagree, and only holds
      on a lane/projection failure. (Side note: the fn parameter is named `case`,
      which shadows `clojure.core/case`, so the reason lookup uses a literal map.)
- [x] List builtins `map`, `filter`, `concatMap`, `all`, `any`, `foldl'`,
      `groupBy`, and `partition` used `(if-let [item (first remaining)] ...)`,
      terminating the loop early on a `false`/`null` element. Switched to
      `(if (seq remaining) ...)`, so e.g. `map (x: x) [1 false 2]` returns the
      whole list. (Attr-key and AST-node loops were already safe — they iterate
      truthy keys/maps.)
- [x] `builtins.mod` evaluated/lowered with Clojure `mod` (floored) while
      `builtins.div` uses `quot` (truncated), so the pair disagreed on negative
      operands. Both lanes now use `rem` (Nix `%`): `mod (-7) 3 = -1` pairs with
      `div (-7) 3 = -2`.
- [x] `builtins.listToAttrs` kept the last duplicate name (`into {}`); Nix keeps
      the first. It now folds left and skips already-seen names.
- [x] `builtins.substring` threw (→ held) when `start` was past the end of the
      string; Nix returns `""`. Indices are clamped so it cannot raise.
- [x] `builtins.replaceStrings` replaced sequentially, so a later pair could
      rewrite an earlier pair's output (`["a" "b"] ["b" "c"] "a"` wrongly gave
      `"c"`). Now a single left-to-right pass yields `"b"`.
- [x] `clojure-projection` `control-flow-trace` unwrapped any map result as a
      trace wrapper (`result`/`effects`), losing a plain map value. It now only
      unwraps when the value actually carries `:result`, matching its siblings.

Deferred frontiers (real gaps, intentionally left as honest boundaries rather
than silently widened in this slice):

- [x] `builtins.compareVersions` special-cases the Nix `"pre"` component
      (which sorts before an empty component) and is covered by
      `evaluator-compare-versions-nix-rules`; the lowering lane also agrees.
- [x] `builtins.fromJSON` parses JSON objects/arrays/scalars through
      `clojure.data.json`, and lowering now calls `pnix-clj.json/read-json`
      instead of falling through an unbound `builtins` symbol.
- [x] String interpolation `${ ... }` close-brace scan handles nested attrset
      braces (`"v=${{ a = 1; }.a}"`) and escaped string literals inside the
      embedded expression (`"brace=${{ a = \"}\"; }.a}"`). Host parser and
      px-runtime parser both ignore braces inside embedded expression string
      literals, normalize escaped expression quotes before parsing the embedded
      source, and `string-interpolation-nested-braces` asserts all-lane
      acceptance.
- [x] The `substring`/`replaceStrings` fixes now cover the lowering lane too:
      clj-meta lowering uses the clamped `pnix-clj.lowering/substring` helper and
      single-pass `replace-strings`, including empty-needle matching.
      `lowering-lane-string-and-json-edges` asserts all-lane acceptance.
- [x] `clojure-projection/project-reader-value` now projects bare lazy seqs as
      `List` terms (`form=false`) and `BigDecimal` as `Scalar`/`bigdec` instead
      of routing them through the JavaObject envelope. Fixture count is now 45;
      `clojure -M:clojure-projection` accepts 45/45.

---

## Completeness Roadmap (research-grounded, 2026-06-30)

This roadmap is built from (a) external reference standards — real Nix / Tvix /
hnix / Nickel, Clojure self-hosting via `tools.analyzer.jvm` ->
`tools.emitter.jvm`, the verified-compiler literature, and differential-
conformance methodology — and (b) a role-translated capability scan of this
project's own architecture. It is NOT coupled to any sibling project; sibling
repos are at most loose idea sources translated by role, never a dependency.
Each milestone cites the external standard it rests on.

Framing invariant (consistent with this file's existing discipline): the four
lanes (semantic evaluator / lowered clj-meta compile / internal `.px` runtime /
pnix self-observation) form an N-version cross-check — i.e. HEURISTIC
differential agreement, not a machine-checked proof. External verified-compiler
results (CompCert semantic preservation; CakeML proof-grounded bootstrap;
verified translation validators) are cited as targets and analogies, never as
guarantees pnix-clj inherits. Per translation-validation discipline, a
validator/receipt must DEFAULT TO HELD/REJECT on uncertainty and stay far
simpler than what it checks. Common-mode risk (Knight-Leveson): lanes that share
code/semantics fail in correlated ways, so cross-lane agreement reduces but does
not eliminate shared blind spots — an independent oracle (Axis 4) is required to
catch a shared error.

Priority verdict from the scan: the completeness gap is concentrated in **Axis 1
(language semantics depth — laziness first)** and **Axis 4 (conformance /
differential corpus)**. Axes 2-3 (host projection, runtime/mirror) are already
role-complete and even lead on real-bytecode evidence and structured errors;
they get targeted depth work only, not reconstruction.

### Axis 1 - pnix language semantic depth  (dominant gap; do first)

Reference: Nix is call-by-need — every binding is a memoized parameterless thunk
forced only to weak-head normal form (WHNF); `builtins.length [ (1/0) ]` is `1`,
it never forces the element (nix.dev evaluation manual; Tvix). Keep the evaluator
decoupled from any build/store concern (Tvix), and treat a real Nix corpus as the
moving oracle rather than a bespoke dialect.

1. [ ] **Laziness / thunk core (L) - foundational, blocks most of this axis.**
   - [x] **`let` bindings are now memoized thunks in the evaluator lane**
     (`evaluator.clj`): a `Thunk` value (compute fn + memo cell + black-hole
     phase), forced at `:var` resolution, with `let` made recursive via a
     knot-tied final env. An unused binding is never evaluated; forward/mutual
     references and recursive *values* (not just lambdas) resolve; direct
     self-reference is a bounded `:infinite-recursion` held, not a hang. The
     per-closure `:env-ref` hack is gone from `let` (the rec-attrset path still
     uses one). Verified by `evaluator-lazy-let-defers-and-recurses`; the full
     gate (23 tests / 897 assertions, all report lanes, conformance 116/116)
     stays green.
   - [x] **Function application is call-by-need** (`evaluator.clj`): arguments are
     passed as thunks, so an unused argument to a simple-param lambda is never
     evaluated (`const 1 (1/0)` -> 1). Builtins and pattern lambdas force their
     argument (strict); thunks are forced at `:var`/builtin/pattern positions and
     never leak into results. Verified by
     `evaluator-lazy-arguments-are-call-by-need`.
   - [x] Lazy attrset values + lazy list items core slice. Attrset values and
     list elements are stored as memoized thunks; select/shape consumers force
     only what they inspect, public eval boundaries deep-realize final values,
     and `==`/`!=`/`toJSON`/`deepSeq` use deep force. Verified by
     `evaluator-lazy-collection-values-are-forced-on-demand`.
   - [x] Make the clj-meta lowering lane and the internal `.px` runtime lane lazy
     too. The clj-meta lane lowers recursive groups through `lazy-letrec`; the
     `.px` evaluator stores list/attr slots as thunks and forces on read. The
     lazy collection receipts now live in `mirror-pair` and reach full all-lane
     acceptance (`lazy-attr-select`, `lazy-list-head`, `lazy-list-length`,
     `lazy-list-elem-at`).
   - [x] `seq`/`deepSeq`/`toJSON` all-lane force slice. `seq` WHNF-forces the
     first arg; `deepSeq`/`toJSON` deeply force values, including attr values in
     `.px` where lazy attr reconstruction previously hid errors. Verified by
     `mirror-pair/seq-*`, `mirror-pair/deep-seq-attrset-ok`, and
     `mirror-pair/to-json-*`.
   - [x] Higher-order list ignored-input slice. `map`/`filter`/`concatMap`/
     `all`/`any`/`count`/`foldl`/`foldr`/`findFirst`/`groupBy`/`imap*` now pass
     lazy element slots through clj-meta and `.px` when callbacks ignore them.
     Verified by 12 `mirror-pair/lazy-*-ignored-input` receipts.
   - [x] Attr/list shape-producer slice. `listToAttrs`/`zipAttrsWith`/
     `catAttrs`/`foldlAttrs` preserve lazy values across clj-meta and `.px`
     while forcing only row/name/key shape. Verified by 5
     `mirror-pair/lazy-*-attr*` / `lazy-cat-attrs-*` receipts.
   - [x] `partition` exactness correction. Direct evaluator now forces elements
     before predicate classification, matching Nix's held behavior for
     `builtins.partition (x: true) [ (1 / 0) ]`.
   - [x] `genAttrs` lazy-value lift. Evaluator and clj-meta now store generated
     attr values as lazy slots; verified by `mirror-pair/lazy-gen-attrs-attrnames`.
   - [x] `elem` equality/tail-laziness lift. clj-meta uses `nix-equal`, `.px`
     stops before a later error after a match, and direct evaluator no longer
     forces an empty-list needle; `.px` handles direct `builtins.elem` lazily
     for that first argument too. Verified by
     `mirror-pair/elem-numeric-equality` and
     `mirror-pair/lazy-elem-stops-before-tail-error` plus
     `mirror-pair/lazy-elem-empty-list-needle`.
   - [x] `attrValues`/`values` lazy-value lift. `.px` now returns lazy attr
     value slots instead of deep-forcing the attrset values up front; verified
     by 3 `mirror-pair/lazy-*-values-*` receipts.
   - [x] `concatLists` inner-element laziness lift. clj-meta and `.px` now force
     only list shapes, not the concatenated element values; verified by
     `mirror-pair/lazy-concat-lists-*`.
   - [ ] Continue builtin-by-builtin laziness exactness audit beyond the core
     selector/shape/equality/force-boundary slices (folds/maps that inspect
     values, equality error-boundary receipts). Promote each case to mirror-pair
     or a dedicated receipt only after all lanes agree.
   - Trap: "referenced" != "forced"; forgetting memoization breaks call-by-need;
     do not let host Clojure eager seqs leak in.
   - Partially supersedes the section-C `[ ] Full laziness-aware recursive
     environment model` (the recursive-environment half is done) and
     `[ ] Laziness/strictness model` (let bindings are lazy; application is still
     strict).

2. [ ] **Grammar parity blockers (each unlocks corpus):**
   - [x] `with` expression (parser + evaluator): `with attrs; body` adds attrs
         as a fallback scope behind the lexical env (lexical bindings win; an
         inner `with` shadows an outer one). The scope list rides in the env, so
         closures defined under a `with` capture it. Evaluator lane, verified by
         `evaluator-with-expression`. (The attrset is currently evaluated
         eagerly; lazy-attrset `with` is part of the deferred lazy-structures
         work.)
   - [x] Indented/multiline strings `'' ''` (tokenizer + evaluator): new lexer
         group, common (space) indentation stripping, leading-newline drop, and
         `${}` interpolation with RAW literals. Verified by
         `evaluator-indented-strings`. Follow-up escape slice closed the
         indented-string escape frontier: `''$`, `'''`, `''\n`/`''\r`/`''\t`,
         and `''\X` decode in literal fragments while escaped `${...}` stays
         literal. Tabs remain intentionally not stripped, matching Nix's
         documented warning.
         Follow-up interpolation exactness slice: `${...}` now uses Nix-style
         interpolation coercion, not `builtins.toString` coercion. Strings,
         paths, attrsets with `__toString`, and attrsets with `outPath` coerce;
         ints/bools/null/lists/plain attrsets/lambdas are held with
         `:string-interpolation-coercion-failed`. Internal `.px` parsing lowers
         interpolation to `builtins.__pnixInterpolateString`. Positive rows are
         in `mirror-pair/interpolation-*`; the negative int row is in
         `mirror-error/interpolation-int-coercion`.
   - [x] Path literals `./x` `../x` `~/x` and search paths `<nixpkgs>`
         (tokenizer + parser + evaluator): a conservative lexer group (path group
         3, requiring a `.`/`..`/`~`/`<` prefix) so division `a / b` / `1/0`,
         `//`, and comparisons `a < b` / `<=` are unaffected (verified). A path
         evaluates to a tagged path value. Verified by
         `evaluator-path-literals`.
         Frontier: path resolution (relative-to-file, NIX_PATH) and path
         interpolation `./${x}`.
         Follow-up path-value slice added a distinct evaluator/lowering path
         value `{"__pnix_value_kind" "path" "path" ...}`, `builtins.isPath`,
         path-aware `typeOf`/`isAttrs`, path equality, pure raw-literal path
         `toString`, and path-aware `baseNameOf`/`dirOf`; `.px`/pnix-mirror
         parity is now covered by `mirror-pair/path-*` fixtures. Relative
         resolution, NIX_PATH lookup, path interpolation, and real-store
         string-context behavior remain frontiers.
         Follow-up absolute-path slice added `/foo` expression-start parsing
         and `/foo` as a function argument after select/lambda/apply callees,
         without changing spaced division (`1 / 2`). All-lane receipts:
         `mirror-pair/absolute-path-*`.
   - Quick wins (S each):
     - [x] `a.b or default` (parser + evaluator): catches a missing attr,
           including a missing intermediate attr on the path, but not other
           errors (an unbound variable still propagates); `builtins.or` still
           resolves normally. Verified by `evaluator-select-or-default`.
           Follow-up lift added clj-meta lowering and internal `.px` runtime
           support for static attr select defaults; all-lane receipts:
           `mirror-pair/select-or-*`.
           Follow-up grammar receipt: the Clojure parser now uses Nix's tight
           select-default grammar, so calls/infix bind outside `or` fallback,
           `or if`/`or let`/lambda defaults require parentheses, and `import`
           stays a callable fallback value (`a.b or import ./m` parses as
           `(a.b or import) ./m`). Added all-lane
           `mirror-pair/select-or-parenthesized-if-default`.
     - [x] `assert cond; body` (parser + evaluator): evaluates the body when the
           condition is true, else an `:assertion-failed` held; a failing
           condition propagates. Verified by `evaluator-assert-expression`.
           Follow-up lift added the true-assertion path to the clj-meta lowering
           and internal `.px` runtime lanes, verified by the 4-lane builtin
           dispatch corpus.
     - [x] `@`-patterns (`args@{...}:` and `{...}@args:`): tokenizer now lexes
           `@`; the whole argument attrset is bound alongside the destructured
           params, coexisting with `?`-defaults and `...`. Verified by
           `evaluator-at-patterns`.
     (The parser already has `?`-defaults, `...`, and `inherit (expr)`.)
   With all three quick wins landed, the remaining Axis-1 item-2 work is the
   larger grammar features (`with`, indented strings, paths) below.

3. [ ] **Impurity/store semantic layer (after thunks + strings):**
   - [x] String context core slice (2026-07-02): contextful strings are the
         string-keyed tagged map `{"__pnix_value_kind" "string-context",
         "string" s, "context" [deps...]}` (same idiom as path-value); a
         context-free string stays a plain JVM String, so existing behavior is
         byte-identical. `builtins.appendContext` creates context (per-key
         info attrsets accepted but not yet modeled); `hasContext`/`getContext`
         (attrset of empty info sets)/`unsafeDiscardStringContext` inspect it.
         Context propagates through `+` (content concat, context union),
         string interpolation (template joiner unions chunk contexts), and
         `toString` (kept intact). `typeOf`/`isString` say string; `==`
         compares content only (Nix semantics). Non-context-aware builtins are
         DENIED BY DEFAULT: a central finish-builtin guard + pnix-to-string/
         nix-coerce-to-string throw backstops hold `:string-context-frontier`
         instead of silently dropping/mangling context; grow
         `context-aware-builtins` builtin-by-builtin with tests. Direct
         evaluator lane only (lowering/.px lanes hold on the unknown builtins =
         declared frontier, consistent with trust.clj
         `:store-and-string-context-frontier`). Verified by
         `evaluator-string-context-core`. Parser fix in the same slice:
         template/indented strings are now valid call arguments
         (`f "a${b}c"`; `call-start-token?` accepted only plain `:string`) —
         `parser-interpolated-string-as-call-argument`.
   - [x] Context-aware string builtins batch (2026-07-02): `stringLength`/
         `hasPrefix`/`hasSuffix`/`hasInfix` = content-based results;
         `substring`/`toUpper`/`toLower` keep the whole context;
         `concatStringsSep` joins contents and unions separator+element
         contexts. Ctx branches live in `finish-context-builtin` and return
         nil for plain strings (legacy path untouched, lenient coercion
         preserved). `audit-string-arg`/`audit-string-list` now accept
         contextful strings as strings (`strict-string?`). Verified by
         `evaluator-context-aware-string-builtins`.
   - [x] `replaceStrings` context propagation (2026-07-02): needles match on
         content; result carries the source context plus the contexts of the
         replacements actually USED (exact — unused pairs contribute nothing;
         the single-pass loop records usage). Plain inputs keep the plain
         String path. Verified in `evaluator-context-aware-string-builtins`.
   - [x] Context kinds + match/split/toJSON (2026-07-02, all oracle-verified
         against local nix-instantiate 2.34.7 BEFORE implementing):
         `appendContext` interprets info attrsets (path=true -> `<p>`,
         allOutputs=true -> `=<p>`, outputs=[o..] -> `!o!<p>`; EMPTY info = NO
         context — fixed our earlier semantics bug); `getContext` decodes the
         encoded elements back to per-path info attrsets and merges kinds on
         one path; derivation `drvPath` context corrected to allOutputs
         (`=<drvPath>`), outPath stays `!out!<drvPath>` — both shapes now match
         the oracle exactly. `match`/`split`: contextful REGEX is held
         (`:regex-argument-has-context`, Nix errors); results from a contextful
         SUBJECT are context-FREE (oracle: Nix drops subject context on
         match/split results). `toJSON` KEEPS context: a strip-and-collect
         walker serializes ctx-string contents and unions all embedded
         contexts onto the result (also closes the silent-wrong hole where a
         nested ctx-string would have serialized as its tagged map). Verified
         by `evaluator-string-context-kinds-and-regex-json`.
   - [x] Multi-output derivations (2026-07-02, oracle-verified): `outputs =
         [..]` validated (non-empty, distinct strings); per-output pseudo
         paths with Nix's `-<output>` name suffix ("out" unsuffixed);
         `outputName`/`outPath` follow the FIRST output; each `d.<o>` is a
         NON-cyclic reduced derivation attrset (type/name/drvPath/outputName/
         outPath) so `"${d.dev}/inc"` tracks the `dev` output dependency;
         `derivationStrict` returns drvPath + one attr per output (oracle
         attrNames `["dev" "drvPath" "out"]`). Real Nix's cyclic `d.out == d`
         stays unmodeled (plain-map value model). Verified in
         `evaluator-derivation-pure-simulation`.
   - [x] toJSON outPath coercion (2026-07-02, oracle-verified): an attrset
         with `outPath` serializes as that path string, recursively — so a
         derivation in a toJSON structure becomes its store path with context
         kept (oracle: `toJSON { outPath = "/x"; other = 1; }` = `"\"/x\""`).
         `__toString` coercion in toJSON = follow-up (needs a call back into
         the evaluator from the strip walker).
   - [x] toString/__toString + fromJSON/chars/splitString + structural ops
         (2026-07-02, oracle-verified): `toString` is now fully context-aware
         coerceMore (list elements' contexts collected — oracle: `toString
         [ ctx "b" ]` carries context; `__toString` called with self, then
         outPath); `toJSON` `__toString` coercion (wins over outPath, must
         yield a string else `:to-json-tostring-not-string`); `fromJSON`
         REJECTS contextful input (`:from-json-argument-has-context`, oracle
         "not allowed to refer to a store path"); `stringToCharacters` keeps
         the full context per char (substring-based lib semantics);
         `splitString` pieces context-free (split-based lib semantics);
         structural list ops (`head`/`tail`/`elemAt`/`last`/`init`/`length`/
         `elem`) allowlisted — they pass contextful elements through
         untouched. Oracle also revealed Nix validates context KEYS as store
         paths lazily ("context key ... is not a store path") — our simulation
         accepts arbitrary keys (documented relaxation).
   - [x] String utils ctx batch (2026-07-02, oracle-verified):
         `removePrefix`/`removeSuffix` keep s's whole context (substring-based
         lib semantics); `toInt` accepts contextful input (oracle: lib.toInt
         parses "42"+ctx to 42); `concatStrings`/`concatMapStrings` union
         element/result contexts. Verified in
         `evaluator-string-context-kinds-and-regex-json`.
   - [x] Lowering-lane string-context lift (2026-07-02): `hasContext`/
         `getContext`/`unsafeDiscardStringContext`/`appendContext` lower to
         runtime helpers that DELEGATE to the evaluator's own builtins via
         `apply-callable` on force-normal'd args — zero semantic drift by
         construction (kinds encode/decode, empty-info no-op, merging all
         agree lane-to-lane). Verified by
         `lowering-lane-string-context-builtins`.
   - [x] `.px` runtime lane lift (2026-07-03): the 4 context builtins
         reimplemented IN PNIX inside `evaluator.px` (delegation impossible
         there) — ctx tagged map is the same value shape, kinds
         encode/decode with `builtins.match`/`substring`, per-path merge,
         empty-info no-op. Determinism fix surfaced by the lift: getContext
         inner info maps are now KEY-SORTED in both the host evaluator and
         the `.px` decoder (values were `=` across lanes but pr-str hashes
         differed by insertion order — mirror-pair value-hash quadruple
         caught it). 4 ctx cases joined the mirror-pair corpus (155→159;
         derived corpus counts 201→205 etc.); string-context sources now
         COLLAPSE through the whole tower, so the honest-degradation probe
         moved to the next true frontier (derivation values). Tower :read
         layer status fixed (was always :missing — run-source carries no
         :parse-result key). **The string-context frontier declared in
         trust.clj is now closed across all four lanes.**
   - [x] Lowering-lane derivation lift (2026-07-03): `derivation`/
         `derivationStrict`/`placeholder` lower via a new GENERIC
         `host-builtin` delegation helper (evaluator single implementation,
         zero drift — pseudo store paths and outPath context kinds agree
         lane-to-lane, multi-output sub-derivations included); `storePath`
         is an explicit purity-gated lowering held. The previous lowering
         ":ok" for derivation was an illusion (generic `(get builtins ..)`
         fallback form that failed at clj-meta). `.px` lane remains the
         derivation frontier — tower held-probe stays honest. Verified by
         `lowering-lane-derivation-builtins`.
   - [x] Lowered `+`/interpolation ctx propagation (2026-07-03): the inline
         string?/str form became a `plus` helper and templates join through
         `template-join` — both built on the evaluator's PUBLIC ctx-string
         constructor/accessors (string-content/string-ctx promoted), so
         contents concatenate and contexts union identically lane-to-lane;
         interpolate-to-string/coerce-to-string pass contextful strings
         through instead of mangling the tagged map. Verified by
         `lowering-lane-ctx-propagation` (ctx+ctx, ctx+plain, template,
         toString + plain regressions).
   - [x] `.px` `+`/interpolation/toString ctx propagation (2026-07-03):
         root cause found the honest way — a ctx tagged map built by a pnix
         ATTRSET LITERAL (as inside the .px runtime) carries lazy THUNKS in
         its slots, so the host ctx-string? predicate saw a thunk and the
         string-like `+` branch never fired. Fix A: the ctx accessors now go
         through a `forced-entry` thunk seam. Fix B (the deeper one): the
         .px-side ctx REIMPLEMENTATION was deleted and replaced by
         host-builtin delegation — the .px program runs ON this evaluator,
         so `builtins.hasContext/...` inside .px ARE the single
         implementation; the reimplementation had only worked while the
         host predicates were thunk-opaque, and hardening them exposed the
         drift. interpolateString/toString in .px pass ctx-strings through.
         2 ctx cases joined mirror-pair (159→161; derived counts updated).
         Verified by `px-lane-ctx-propagation`. **String-context now
         propagates through +/templates/toString on all four lanes.**
   - [x] `.px` derivation lift (2026-07-03): derivation/derivationStrict/
         placeholder/storePath DELEGATED inside evaluator.px (the .px
         program runs ON this evaluator, so builtins.derivation in .px IS
         the single host implementation — the hash-canonicalization
         question dissolved entirely; yesterday's assumption presumed
         reimplementation). 6/6 px-lane agreement incl. multi-output
         sub-drv and interpolation context; storePath propagates the
         host purity-gated held. 2 derivation cases joined mirror-pair
         (163; derived counts 209/202/242). **Derivation family now 4/4
         lanes; tower held-probe walked to functionArgs (lowering
         frontier).**
   - [x] specialize position-observing guard (2026-07-03) — **first live
         catch by the tower specialize-residual harness**: folding
         literalizes attrsets and erases position metadata (observable
         since codex surfaced source positions), and a residual is
         DIFFERENT source text, so no residual can preserve
         unsafeGetAttrPos answers. specialize now refuses such sources
         honestly (`:position-observing-source-not-specializable`);
         the tower shows the blocking reason. Verified by
         `specialize-refuses-position-observing-sources`.
   - [x] Pattern-lambda lowering + honest functionArgs (2026-07-03): the
         functionArgs probe frontier's ROOT was pattern-lambda lowering
         (`:pattern-lambda-lowering-not-wired`). Lowered pattern lambdas now
         bind formals sequentially from the argument attrset (defaults see
         earlier formals, mirroring the evaluator's bind loop), `@as` binds
         the whole attrset, extra keys accepted, and the fn carries
         `:pnix/function-args` metadata so `functionArgs` answers on VALUES
         (`function-args` runtime helper; the old syntactic special-case —
         which silently returned {} through variables — was deleted;
         `builtins.X` references constant-fold to {} since their generic
         lowering leaves a free `builtins` symbol). Lifting this EXPOSED the
         .px lane's own syntactic special-case (`functionArgsForAst`)
         silently returning {} through variables — cross-mirror caught it as
         a value mismatch; .px now answers exactly for LITERAL lambdas
         (AST-based, pattern maps included) and errors honestly otherwise
         (closures there carry no formals). 9/9 lowering-lane agreement;
         probe stays held on the .px side. `.px` pattern-lambda APPLICATION
         remains the recorded frontier. Verified by
         `lowering-lane-pattern-lambdas`.
   - [x] `.px` pattern-lambda application + MetaBuiltin seam (2026-07-03):
         the `.px` self-runtime now EVALUATES pattern lambdas —
         `mk_pattern_closure` carries `formals`, `apply_pattern_closure`
         binds sequentially (defaults eval in the ACCUMULATED env so they
         see earlier formals; present attrs bind the raw slot so unused
         extras stay lazy — `{ a, ... }: a` ignores `boom = 1/0`), wired at
         BOTH application sites (`applyValue` + `evalApply`) and at
         `closure_to_host` so pattern closures marshal through native
         builtins (`builtins.map ({x,...}: ...)`). Lifting application
         EXPOSED the next silent-wrong: the table's value-based
         `functionArgs` sat behind the generic native path, which
         marshals closures into opaque host fns — formals LOST, silent {}.
         Fix = MetaBuiltin marker: `builtins.functionArgs` is now a tagged
         VALUE (`mk_meta_builtin`), dispatched with the RAW meta value
         (`apply_meta_builtin`, never marshalled), so it survives variables
         AND aliases (`let g = builtins.functionArgs; in g f`) where the
         old syntactic special-case went silent-wrong; `typeOf`/`isFunction`
         answer "lambda"/true for the marker. Dead AST-based helpers
         (`functionArgsForAst`, `lambdaParamFunctionArgs`, unused
         `directFunctionArgs` binding) deleted. The SAME alias probe then
         caught the lowering lane's own gap — bare `builtins.functionArgs`
         in value position left a free `builtins` symbol (clj-meta eval
         failed) — fixed via `builtin-constant-form` → `function-args`
         helper var, which now also holds honestly on non-functions.
         Tower functionArgs probe COLLAPSED across all lanes; held-probe
         walked to `@as` (`({ a }@args: a + args.a) { a = 21; }` — the
         `.px` parser has no `@` yet = recorded frontier). +7 mirror-pair
         cases (170; corpus 216/209 strict-ok/249 rows). Verified by
         `px-lane-pattern-lambdas`.
   - [x] `.px` `@as` patterns (2026-07-03): the `.px` tokenizer already
         lexed `@` as a plain `sym` (single-char fallback) — only the
         parser + evaluator needed extending. `paramset_lambda_start`
         accepts `}@name:`; `parse_paramset_lambda` gained a leading-bind
         param (host-parser mirror: `a@{...}@b` fails at the ':' check)
         and emits the AST-spec tag `AttrSetWithBind{bind_name,fields,
         ellipsis}` — declared in pnix_ast.px long before the parser
         could read `@`; both `parse_lambda` variants (apply/no-apply)
         parse leading `name@{...}:`. Evaluator: `mk_pattern_closure`
         carries `bind`; `apply_pattern_closure` binds the WHOLE forced
         argument BEFORE the formals fold — host parity: defaults can
         reference the binding (`args@{ a, b ? args.a + 1 }`), and `@`
         binds the ACTUAL argument (defaults excluded: `({a?5}@args:
         args) {}` = `{}`). functionArgs reports formals only. 10/10
         lane agreement; tower @as probe COLLAPSED; held-probe walked to
         bare builtins as VALUES (`let g = builtins.map; in g ...` —
         lowering leaves a free `builtins` symbol the clj-meta host lane
         cannot execute = recorded frontier). +4 mirror-pair cases (174;
         corpus 220/213/253). Verified by `px-lane-pattern-lambdas`
         (@as block).
   - [x] Builtins as VALUES in the lowering lane — the lazy bridge
         (2026-07-03): a bare `builtins.X` (or `builtins` itself) in value
         position now lowers to `builtins-attrset`, the WHOLE builtin set
         DELEGATED to the evaluator (single implementation, zero drift)
         through a bidirectional LAZY bridge: lowered slots cross as
         evaluator thunks (`make-value-thunk`), lowered fns as the new
         `:lazy-host-fn` callable kind (argument arrives RAW — `:host-fn`
         forces to WHNF, which would have made every delegated function
         strict), and evaluator thunks/callables come back as slots/fns —
         so `length` never forces elements and `map` stays element-lazy
         ACROSS the bridge (`let g = builtins.map; in builtins.length
         (g (x: 1 / 0) [ 1 2 ])` = 2 on all lanes). Emission point is the
         `:var` branch: free `builtins` (only) becomes the table; lexical
         shadowing already worked via `*lexical-vars*`; the new branch
         sits BEFORE the with-scope lookup, which FIXED a live divergence
         (`with { builtins = 3; }; builtins` answered 3 in the lowered
         lane vs the full set in the direct evaluator — Nix: statically
         known `builtins` beats `with`). Table keys = the evaluator's
         full builtin key set (attrNames parity, 160); one override:
         `functionArgs` → `function-args` helper (lowered pattern
         metadata — the bridged builtin would silently answer {}).
         Partial application curries across the bridge (builtin records
         re-wrap per step). Evaluator gained the bridge seams
         (`value-thunk?`/`make-value-thunk`/`lazy-host-fn` + apply-callable
         branch with the fuel rethrow guard; typeOf/isFunction say
         "lambda"/true) — capabilities drift gate fired and was
         regenerated, as designed. Tower probe COLLAPSED; held-probe
         walked to application-argument laziness (`(x: 1) (1 / 0)` — the
         lowered lane evaluates general application arguments eagerly;
         px holds there too = recorded frontier). +5 mirror-pair cases
         (179; corpus 225/218/258). Verified by
         `lowering-lane-builtin-values`.
   - [x] Call-by-need application in the lowering lane (2026-07-03): the
         generic `:call` emission wrapped nothing — `(f arg-form)`
         inherited Clojure strictness, so `(x: 1) (1 / 0)` held where the
         direct evaluator answers 1. Arguments now cross as lazy slots
         (params were ALREADY slot-ready: lambda/pattern params live in
         `*force-on-read-vars*` and force-slot on read; the bridge
         converts delays to thunks). Scalars can't throw and bare symbols
         are already slots or realized values — both pass unwrapped.
         Curried chains defer each argument independently; unused
         erroring arguments survive into lazy lists
         (`let f = x: builtins.length [ x ]; in f (1 / 0)` = 1). The
         `.px` meta evaluator still applies eagerly — probe unchanged,
         blocking layer now px-runtime (= next slice: thunked apply args
         in evaluator.px; watch the table's `is_closure`/`ensureFunction`
         arg inspections — they must force first). No corpus additions
         (px-strict sources can't agree 4-lane yet). Verified by
         `lowering-lane-lazy-application`.
   - [x] tryEval catchability across lanes (2026-07-03): Nix tryEval
         catches ONLY throw/assert. LOWERED lane: throw/assert emissions
         now carry `:pnix/catchable` ex-info tags; tryEval's emission
         catches exactly that class and rethrows everything else (the old
         `catch Throwable` answered `{ success = false; }` for division
         by zero, missing attrs, abort — silent-wrong); `throw-held`
         propagates catchability across the builtin-value bridge
         (:throw-builtin-called/:assertion-failed), so
         `let t = builtins.throw; in tryEval (t "x")` keeps its error
         class; bare `throw`/`abort` vars lower to `throw-value`/
         `abort-value` helpers (were free symbols); table overrides for
         "throw"/"abort". PX lane: errors gained an explicit `catchable`
         flag (`mk_catchable_error` — throw/assert only); directTryEval
         honors it and PROPAGATES non-catchable errors (it used to catch
         every px error value); `builtins.throw`/`builtins.abort` were
         MISSING from nativeBuiltins — tryEval had been catching the
         accidental missing-attribute error, and bare `throw` (a host
         builtin record) escaped as held so px tryEval never caught it —
         both now real px-level entries. 9/9 battery: throw/assert/
         bare-throw/alias-throw catchable on ALL lanes; 1/0, head [],
         abort held on ALL lanes. +3 mirror-pair cases (182; corpus
         228/221/261); capabilities regenerated (throw-value/abort-value).
         Verified by `cross-lane-try-eval-catchability`.
   - [ ] DISCOVERED (2026-07-03): `(builtins.tryEval ...).success` —
         SELECT directly on a tryEval application holds at clj-meta
         (preexisting, also before the catchability fix; likely the
         emitted `try` in expression position inside `get`). Let-bound
         access (`let r = ...; in r.success`) works — recorded frontier.
   - [ ] px error taxonomy follow-up: px mk_error carries only a message
         (+ new catchable flag). Cross-lane REASON comparison (why held)
         is not yet possible for px-internal errors — grow kinds per
         need.
   - [x] Call-by-need application in the `.px` lane (2026-07-03): the meta
         evaluator's apply-arg is now a thunk (`mk_thunk "apply-arg"` —
         pure recompute-on-force call-by-name; values identical, no
         memoization in pure pnix). Var reads already forced
         (`force_value` at Var); the native branch forces AT the host
         boundary (genuine passthrough builtins are strict) and marshals
         AFTER forcing; directTryEval forces explicitly;
         `ensureFunction` forces before inspecting. ★Caught live by the
         corpus (lazy-zip/mapAttrsToList rows, 3 red): forcing in
         applyValue's native branch evaluated IGNORED lazy slots —
         applyValue serves table-wrapper internals whose host-fn callees
         are closure_to_host marshals that re-enter the evaluator and
         force on Var read, so it must keep passing RAW slots. Reverted
         there; the force lives only at the OBJECT-level evalApply
         boundary. `(x: 1) (1 / 0)` now collapses on ALL FOUR lanes —
         probe walked to select-on-tryEval-application
         (`(builtins.tryEval (builtins.throw "x")).success` — emitted
         `try` in expression position inside `get`, clj-meta cannot
         compile; let-bound access works). +3 mirror-pair cases (185;
         corpus 231/224/264). Verified by `px-lane-lazy-application`.
   - [x] clj-meta: try in expression position (2026-07-03): root cause of
         the select-on-tryEval probe was a genuine COMPILER bug in the
         host proof lane — `VerifyError: Operand stack underflow`. JVM
         exception-handler entry CLEARS the operand stack, so a `try`
         emitted while caller values sit on the stack (e.g.
         `(force-slot (get (try …) "success"))`) loses them. Fix in
         ../clj-meta compiler.clj = `hoist-expression-tries` pre-pass in
         `analyze-for-compile-form` (covers :root + wrapper paths):
         every `try` rewrites to a zero-arg `fn*` CALL — fn body is
         RETURN context, stack always empty — exactly Clojure
         Compiler.java's TryParser FNONCE wrapping; over-approximates
         (tail tries wrapped too; identical semantics — recur cannot
         cross try anyway), quote-interior untouched, metadata
         preserved. Select/attrNames directly on tryEval applications
         now collapse on ALL lanes. Tower probe walked to `import`
         (the M-scale epic; evaluation not wired without an in-memory
         module map). +2 mirror-pair cases (187; corpus 233/226/266).
         Verified by tower test (try-hoisting testing block) + full
         gate over the patched compile-form path.
   - [ ] String context follow-ups:
         derivation values across lanes (the new tower probe frontier);
         `baseNameOf`/`dirOf`/`concatMapStringsSep`/`optionalString`
         evaluator ctx (grow per need); context-key store-path validation
         (relaxed); `all` list of sub-derivations.
   - [x] Derivations pure-simulation slice (2026-07-02): `derivation` returns
         the input attrs merged with `type="derivation"`/`drvPath`/`outPath`/
         `outputName`; `derivationStrict` returns `{drvPath, out}`. Paths are
         deterministic pseudo store paths `/nix/store/<32-hex>-<name>(.drv)`
         hashed from the deep-forced input attrset (NOT byte-compatible with
         real Nix hashing — documented simulation scope, Tvix-style
         evaluator/builder separation, no on-disk store). drvPath carries its
         own path as context; outPath carries `!out!<drvPath>` — so
         `"${drv}/bin/x"` interpolation propagates the dependency through the
         string-context layer end-to-end. Validation held: missing
         name/system/builder, non-attrset input, function-valued attr
         (`:derivation-attr-not-coercible` — pr-str hashing must never see fn
         objects). `placeholder` = deterministic context-free pseudo hash;
         `storePath` = held `:store-path-purity-gated` (Nix pure-eval).
         Cyclic `out`/`all` self-references not modeled (plain-map value
         model). finish-builtin hit the JVM 64KB method limit; the
         string-context + derivation builtins moved to a
         `finish-context-builtin` pre-dispatch. Verified by
         `evaluator-derivation-pure-simulation`.
   - [ ] Derivation follow-ups: multi-output (`outputs = [..]`) with per-output
         placeholders/context; cyclic `out`/`all` (needs a value-model
         decision); context kinds (path/allOutputs/outputs) in getContext;
         `unsafeDiscardOutputDependency`; lift lowering/.px frontier.

4. [ ] **`import` / `scopedImport` across all lanes (M):** the AST `:import`
   node currently evaluates only in the direct lane and is held in lowering.
   Wire the import resolver through `core/run-source`, add lowering policy for
   `:import`, reuse the px-runtime import-graph/cycle receipts, add `scopedImport`.
   - [x] Host-lane in-memory import slice (2026-07-02): `run-source` now accepts
         `:import-modules` and threads it through both the direct evaluator and
         clj-meta lowering. Lowering resolves modules, nested imports,
         not-found, and cycles without filesystem access; lowering cache keys
         include import module hash/context to avoid stale cross-module forms.
         `.px` runtime import is still a separate frontier because its current
         import model is host `.px` file import, not pnix source module maps.
         Verified by `evaluator-in-memory-import`; full gate:
         `clojure -M:test` → 96 tests / 2386 assertions.
   - [x] scopedImport semantics + arg-order bug (2026-07-03): the tower
         `import` probe surfaced `scopedImport`, which was BROKEN on all three
         lanes — the arguments were swapped. ★Oracle (nix-instantiate 2.34.7):
         `scopedImport scope path` takes the SCOPE FIRST, PATH SECOND; the
         scope attrs are ADDED on top of the global env (base globals stay
         available, scope shadows on name conflict), and the scope does NOT
         propagate through nested plain `import`. All three lanes had
         `(first args)` as the target (= the scope) — so it could never
         resolve the real module and silently dropped the scope. Fixed the
         arg order on ALL lanes; the existing fixtures encoded the bug
         (`scopedImport ./m {}`) and were corrected to `scopedImport {} ./m`
         (receipts: oracle transcript, not a blind flip). Scope injection:
         the DIRECT lane injects fully (scope is host-shaped there — merge
         into `default-env`, `eval-ast` gained a base-env arity); the resolver
         signature grew a `scope` param (nil for plain import) across the
         core + both px resolvers (2-arg → 3-arg). The MIRROR lanes hold a
         NON-empty scope honestly — px because the scope reaches the host
         resolver `.px`-shaped (`typeOf` sees "set", its entries are `.px`
         thunks this evaluator cannot force — a marshalling frontier), clj
         because scope injection into lowered forms is a lowering frontier
         (`:scoped-import-scope-not-lowered`). An EMPTY scope == plain import
         and agrees on all four lanes. Import WITH modules already collapses
         4-lane; the tower probe stays on the no-module default (the
         module-map-into-corpus epic). Verified by
         `scoped-import-scope-semantics` (+ corrected
         `evaluator-in-memory-import`); gate 128 tests / 2907 assertions.
   - [x] import/scopedImport into the corpus + tower (2026-07-03): the tower
         probe pointed at `import` because the corpus carried NO import
         cases and the tower dropped module maps. Threaded `:import-modules`
         end-to-end: the mirror-pair `cases` fn now carries per-case
         modules (`pnix/report` already passes the whole row to
         `run-source`); the strict-audit mirror-pair source-rows carry
         `:import-modules` too (determinism + coverage consume those rows
         and already thread modules, so all three module-less reports now
         resolve the cases); and `run-tower` accepts a map
         `{:source :import-modules}`, binding `evaluator/*import-modules*`
         for the WHOLE climb so run-source AND specialize's fold + residual
         re-eval resolve against the same map (specialize marks `:import`
         heavy => residual == original, so a single dynamic binding suffices
         — no specialize surgery). +5 corpus cases (import module, arith
         through module, nested transitive, attrset-select, empty-scope
         scopedImport); all collapse 4-lane in the tower (192/192). The tower
         held-probe walked from `import` to scopedImport with a NON-EMPTY
         scope (direct injects, mirror lanes hold — the marshalling/lowering
         frontier from the previous slice). Corpus 238/231 strict-ok/271
         rows; gate 128 tests / 2910 assertions. Verified by the tower test
         (import-collapse + moved probe) and the mirror-pair report over 192
         cases.
   - [x] clj-meta scopedImport scope injection (2026-07-03): narrowed the
         non-empty-scope frontier from {clj, px} to {px}. At lowering time
         the scope is an AST, so the lowered lane injects it: scope keys
         become force-on-read parameters of the imported module, emitted as
         a zero-shadow function application `((fn [k..] module) v-slot..)`.
         Value slots are lowered in the CALLER context (a scope value that
         names another scope key sees the caller binding, matching the
         direct lane's non-recursive forced-attrset merge) and the module is
         lowered with the keys lexical; the slots are lazy, so an unused
         erroring scope key does not fail (`{ a = 1; boom = 1/0; } ./m`
         where m=`a` → 1 on both lanes). Held honestly when the scope
         shadows `builtins` (`builtins.X` lowers via a select special-case
         that never reads the lexical env, so it would diverge from the
         direct merge — `:scoped-import-scope-shadows-builtins`). direct ==
         clj-meta now for `{x;y}`, `builtins.add x 1`, `map`-shadow, 3-key,
         laziness; px still holds (scope reaches its host resolver
         `.px`-shaped — the last frontier lane). Not yet 4-lane, so not in
         the corpus; tower probe unchanged (still px-blocked). Verified by
         `scoped-import-scope-semantics` (direct+clj) and the lowering
         validation test; gate 128 tests / 2916 assertions.
   - [x] px scopedImport scope injection — scopedImport now 4-LANE
         (2026-07-03): the LAST frontier lane closed. Instead of forcing
         `.px` thunks host-side (architectural wall), the `.px`
         `scopedImport` wrapper DEEP-FORCES the scope in the `.px` world
         (`deep_force_value scope`) BEFORE it crosses the host boundary —
         it arrives as plain host values the resolver merges into the
         module's global env, exactly like the direct lane. Honest cost:
         eager, so a scope with an erroring UNUSED key holds on px where
         direct/clj stay lazy (`:scoped-import-scope-eval-held`). ★Also
         fixed a clj-meta capture bug the px lift EXPOSED via cross-mirror:
         the injecting `fn`'s param `x` lexically captured a same-named
         free var inlined from a NESTED import (clj gave 1 where direct/px
         held). Fix = munge scope params to collision-proof names
         (`x` -> `x*scope`; `*` is not a legal pnix ident char) via a new
         `*lexical-renames*` map, and lower imported modules HERMETICALLY
         (reset lexical/with, inject only the renamed scope keys — a latent
         module-hermeticity fix too). Nested scopedImport now holds on ALL
         lanes. scopedImport { x=10;y=5 }, builtins.add x, map-shadow,
         3-key all collapse 4-lane; +3 corpus cases (195; 241/234/274).
         Tower probe walked from scope-injection to the px-scope-laziness
         frontier (`{ a=1; boom=1/0 } ./m` where m=`a` — px eager-forces,
         holds). Verified by `scoped-import-scope-semantics` (4-lane) +
         tower test; gate 128 tests / 2922 assertions.
   - [ ] px scope laziness (px-only frontier): px deep-forces the scope
         eagerly, so an erroring unused scope key holds where direct/clj
         stay lazy. Lifting needs a lazy px→host thunk bridge (the same
         wall). Honest held; documented as the tower probe.

5. [ ] **Builtin breadth + exactness (M aggregate; fixture-driven cadence):**
   - Add missing builtins incrementally:
     - [x] Mirror-pair op/branch coverage slice (2026-07-02): internal `.px`
           lexer now tokenizes unary `!`, allowing `!false` to reach all-lane
           agreement. Added coverage receipts for `assert`, unary `!`, unary
           negation, `&&` right/short-circuit branches, and `||`
           right/short-circuit branches. Follow-up in the same lane added
           all-lane `->` implication parsing/lowering/eval, including
           right-association and false-antecedent short-circuit receipts.
           Error-path follow-up added `assert false; 1` to `mirror-error`,
           closing the `:assert/fail` executable branch with all-lane error
           agreement (`:assertion-failed`, `.px` ast tag `Assert`).
           Follow-up lifted `with` across lowering and `.px` runtime parser/
           evaluator, with Nix-verified receipts for basic lookup, lexical
           shadowing, inner-with shadowing, and closure capture.
           Import coverage follow-up added an honest `:import-module` source
           family for the already-wired in-memory host import seam; `.px`
           source-module import remains a frontier, so this is not filed as
           mirror-pair.
           Binary-operator follow-up added all-lane receipts for `!=`, `<=`,
           and `>=`; `%` was removed from coverage's Nix binary-operator
           target set because local Nix rejects `%` as syntax (use
           `builtins.mod` for remainder semantics).
           Nix builtin follow-up added all-lane receipts for `floor`, `ceil`,
           positive `bitAnd`/`bitOr`/`bitXor`, `genericClosure`, and
           `tryEval (builtins.throw ...)` (the latter covers `throw`; `tryEval`
           remains a special evaluator path, not a normal builtin dispatch
           counter).
           Type-predicate follow-up added all-lane receipts for `isBool`,
           `isInt`, `isFloat`, `isString`, `isList`, `isNull`, and
           `isFunction`.
           `clojure -M:mirror-pair` → 147 accepted;
           `clojure -M:mirror-error` → 4 accepted.
     - [x] Mirror-pair builtin breadth receipt slice (2026-07-02): promoted
           Nix-verified all-lane fixtures for `replaceStrings`, `fromJSON`,
           regex `match`/`split`, `compareVersions`, `parseDrvName`, and
           `splitVersion`. Excluded helpers missing from local Nix
           (`boolToString`, `hasPrefix`, `concatStrings`, etc.) from Nix
           builtin claims. `clojure -M:mirror-pair` → 116 accepted.
     - [x] First batch (evaluator lane): `attrValues` (alias of `values`),
           `concatStrings`, `hasPrefix`, `hasSuffix`. Verified by
           `evaluator-builtin-breadth-batch`. (Lowering policy / 4-lane fixtures
           are a later step; these are evaluator-lane unit-tested for now.)
     - [x] Second batch (evaluator lane): `bitAnd`/`bitOr`/`bitXor` (real Nix
           builtins), `foldr` (right fold), `attrByPath`. Verified by
           `evaluator-builtin-breadth-batch-2`.
     - [x] Third batch (evaluator lane): `dirOf`, `mapAttrsToList`, `optional`,
           `optionals`. Verified by `evaluator-builtin-breadth-batch-3`.
     - [x] Fourth batch (evaluator lane): `toLower`, `toUpper`,
           `stringToCharacters`, `range` (inclusive). Verified by
           `evaluator-builtin-breadth-batch-4`.
     - [x] Fifth batch (evaluator lane): `last` (held on empty), `init`,
           `unique`. Verified by `evaluator-builtin-breadth-batch-5`.
     - [x] Sixth batch (evaluator lane): `concatMapStrings`, `splitString`
           (keeps empty pieces; empty sep -> chars). Verified by
           `evaluator-builtin-breadth-batch-6`.
     - [x] `genericClosure` (worklist traversal deduped by `key`). Verified by
           `evaluator-generic-closure`.
     - [x] Eighth batch (evaluator lane): `min`, `max`, `imap0`, `imap1`.
           Verified by `evaluator-builtin-breadth-batch-8`.
     - [x] Ninth batch (evaluator lane): `optionalString`, `removePrefix`,
           `removeSuffix`, `concatMapStringsSep`. Verified by
           `evaluator-builtin-breadth-batch-9`.
     - [x] Tenth batch (evaluator lane): `id`, `flip`, `toInt`. `const` was
           intentionally skipped — it is lazy in its second argument in Nix,
           which a strict builtin cannot model (use `x: y: x`). Verified by
           `evaluator-builtin-breadth-batch-10`.
     - [x] Eleventh batch (evaluator lane): `replicate`, `findFirst`, `foldl` (alias of `foldl'`). Verified by `evaluator-builtin-breadth-batch-11`.
     - [x] Twelfth batch (evaluator lane): `recursiveUpdate` (deep attrset merge). Verified by `evaluator-recursive-update`.
     - [x] Thirteenth batch (evaluator lane): `hasInfix`, `pipe`. Verified by `evaluator-builtin-breadth-batch-13`.
     - [x] **DONE (owner-approved Frontier-marker path)** rec attrset forward references (`rec { a = b + 1; b = 10; }` currently `:unbound-var`). A knot-tied-thunk rewrite of `eval-attrs` (mirror of `eval-let`) makes the evaluator lane resolve forward refs + mutual recursion + recursive closures correctly, AND was REVERTED: the mirror-error fixture `mirror-error/rec-forward-reference` (source-hash `feb5e298…`) asserts all four lanes agree that rec-forward-ref is `:unbound-var`. Fixing only the evaluator lane makes it diverge from the clj-meta/clojure-mirror/px-runtime lanes (which genuinely don't support rec forward refs), failing `mirror-error-report-aligns-expected-error-boundaries`. To land this: fix all four lanes' rec semantics together (or reclassify/remove that mirror-error fixture) — a coordinated, user-supervised change. This is the concrete confirmation of the "lazy attrset values entangled with mirror-error fixture" warning. The working evaluator-lane patch is reconstructable from this conversation.
       - **Taxonomy audit done (read-only, no behavior change): `rec-forward-reference-taxonomy.md`.** Key evidence: (1) clj-meta/px-runtime lanes `held` on the *valid* `let a = b+1; b=10; in a` too, so they are a forward-ref **frontier**, not an error judge — the mirror-error "agreement" on rec-forward is spurious (frontier-held vs evaluator's unbound-var bug); (2) `let` is already Nix-correct (forward=ok 11, cycle=infinite-recursion), `rec` collapses all three cases to `:unbound-var`; (3) the evaluator-ahead divergence is ALREADY tolerated for `let-forward` (no fixture), so fixing `rec` yields the same accepted shape once the mis-filed mirror-error fixture is reclassified with a receipt. Proposed classes: `forward-ok` / `cycle-error(:infinite-recursion)` / `unbound-error(:unbound-var)`. Owner-proposed `let-forward => error` contradicts Nix (flagged, not implemented).
       - [x] **LANDED (Frontier-marker path)**: eval-attrs knot-tied fix (evaluator Nix-correct), `rec-forward-reference` removed from mirror-error corpus (test now 2 cases), and reclassified into a new `resources/pnix_clj/forward_reference/cases.edn` frontier corpus with `frontier-lanes` markers + `forward-reference-frontier-corpus` deftest asserting evaluator=Nix-verdict and clj-meta/stage15/px lanes held. Gate green. clj-meta/px-runtime recursive-binding support = separate larger frontier-lift work.
     - [x] **Bug fix** (RAW-FREE) fold/predicate builtins reject non-lists: `all`/`any`/`foldl'`/`foldl` no longer iterate a string (a fold accumulator could leak chars). Verified by `evaluator-fold-predicate-builtins-reject-non-lists`.
     - [x] **Bug fix** (RAW-FREE) more list builtins reject non-lists: `reverseList`/`take`/`drop`/`unique`/`elemAt`/`partition` no longer leak chars on a string. Verified by `evaluator-more-list-builtins-reject-non-lists`.
     - [x] **Bug fix** (RAW-FREE) list builtins reject non-lists: `map`/`filter`/`concatMap`/`sort` on a string used to leak raw Clojure chars as pnix values; now held (`:map-arg-not-list` etc.). Verified by `evaluator-list-builtins-reject-non-lists`. (foldl'/all/any type-checks = follow-up.)
     - [x] **Bug fix** `tryEval` scope: now catches only `throw`/`assert` (Nix semantics); abort, type errors, division-by-zero, out-of-bounds propagate instead of being swallowed as `success=false`. Verified by `evaluator-tryeval-only-catches-throw-assert`.
     - [x] **Bug fix** `isAttrs`: now excludes functions/builtins (tagged maps) via `attrset-value?`; was a bare `map?` so `isAttrs (x: x)` wrongly returned true. Verified by `evaluator-isattrs-excludes-functions`.
     - [x] **Bug fix** `compareVersions`: now applies Nix component rules (`pre` < absent < real component, numeric > non-numeric, empty padding) so `compareVersions "1.0" "1.0-pre"` is 1, not -1. Verified by `evaluator-compare-versions-nix-rules`.
     - [x] `replaceStrings` empty needle: empty `from` now matches at every position incl. end (`[""] ["X"] "ab"`->"XaXbX"), no longer a frontier; non-empty single-pass preserved. Verified by `evaluator-replacestrings-empty-needle`.
     - [x] **Bug fix** list-builtin guards: `head`/`tail`/`init` now held on empty list and on non-list (were silently nil/[]); `length` requires a list. Verified by `evaluator-list-builtin-guards`.
     - [x] **Bug fix** Nix equality: numbers compare across int/float (`1 == 1.0` -> true, recursively in lists/attrsets); functions never equal. Applied to `==`/`!=`/`eq`/`elem` via `nix-equal?`. Was Clojure `=` (`1 == 1.0` was false). Verified by `evaluator-nix-equality`. Follow-up all-lane lift added `pnix-clj.lowering/nix-equal` + `.px` `nix_equal`, with `mirror-pair/numeric-equality-*` receipts.
     - [x] `->` logical implication operator: lowest precedence, right-assoc, short-circuits on false antecedent (`!a || b`). `-`/`>` tokenization unaffected. Verified by `evaluator-implication-operator`.
     - [x] Attrset builtins added: `mapAttrs'`, `genAttrs`, `nameValuePair`, `foldlAttrs`, `addErrorContext` (passthrough), `unsafeGetAttrPos` (null). Verified by `evaluator-attrset-builtins-batch`.
     - [x] **Bug fix** `toString` coercion: `true`->"1", `false`/`null`->"", lists->space-joined element coercions (recursive), attrset->outPath, incoercible->held. Was naive `(str v)` (`true`->"true", `[1 2 3]`->"[1 2 3]"). Interpolation/concat left as-is. Verified by `evaluator-tostring-coercion`.
     - [x] **Bug fix** `split`: was a plain string split (`["x" "y" "z"]`); now Nix-faithful interleave of pieces with per-match capture-group lists (`["x" [] "y" [] "z"]`, `["" ["a"] "c"]`). Verified by `evaluator-split-interleaves-groups`.
     - [x] **Bug fix** `fromJSON`: was using `edn/read-string` (so compact `{"a":1}` became `{"a" :1}` keyword); now parses real JSON via `clojure.data.json` (added as explicit dep, `json/read-json`). toJSON/fromJSON round-trips. Verified by `evaluator-fromjson-parses-json`.
     - [x] Ordering comparisons `< > <= >=`: numbers numeric, strings + lists lexicographic (prefix ordering, nested), incomparable operands held. `<` is primitive, rest derived. Verified by `evaluator-ordering-comparisons`.
     - [x] Nested attr paths `{ a.b.c = v; }`: dotted LHS paths build/merge nested attrsets (incl. dynamic keys in the path, rec exposure); conflicts held `:duplicate-attr`; string keys not split. Verified by `evaluator-nested-attr-paths`.
     - [x] Nix default scope: bound the unprefixed builtin subset (`map`, `toString`, `throw`, `abort`, `removeAttrs`, `baseNameOf`, `dirOf`, `isNull`) at top level; non-default builtins still require `builtins.`. Verified by `evaluator-default-scope-builtins`.
     - [x] Dynamic attrset keys `{ ${e} = v; }`: `${` tokenizes as punct (no group renumbering), parser reads inner expr to `}`. String interpolation unaffected. Verified by `evaluator-dynamic-attrset-key` + `evaluator-dynamic-select-has-attr` (select `s.${e}`, has-attr `s ? ${e}`, with `or`).
     - [x] `let` `inherit`: both `inherit (e) a b` (recursive scope) and plain `inherit x` (enclosing scope, no self-cycle) now parse + evaluate. Verified by `evaluator-let-inherit`.
     - [x] In-memory `import`: `eval-source-with-imports` resolves `import <target>` against a pnix module map (no FS), cycle/not-found held, default behavior preserved. Verified by `evaluator-in-memory-import`.
     - [x] Fourteenth batch (evaluator lane): `count`, `zipListsWith`, `boolToString`. Verified by `evaluator-builtin-breadth-batch-14`.
     - [x] Remaining purity/path-gated names are explicit now:
           `pathExists`/`readFile`/`readDir`/`getEnv` are present in the builtin table but
           held with structured effect-policy reasons (`:file-read`,
           `:directory-read`, `:env-read`, `:path-exists`) instead of falling
           through as unknown builtins or reading host state. `.px` runtime
           mirrors the policy with purity-gated errors. `isPath` still waits
           on a real path value type (path-literal slice). Verified by
           `evaluator-impure-builtins-are-purity-gated`.
     - [x] Wire lowering policy (clj-meta lane) for the most common of the new
           builtins so they can reach 4-lane curated fixtures, not just the
           evaluator-lane unit tests. First pure/value builtin lift covers
           `attrValues`, `concatStrings`, `hasPrefix`, `hasSuffix`, `optional`,
           `optionals`, `min`, `max`, `optionalString`, `removePrefix`,
           `removeSuffix`, and `boolToString` in both clj-meta lowering and
           `.px` runtime. Verified by the expanded
           `builtin-dispatch-closes-attr-list-and-arithmetic-slice` 4-lane
           corpus. Follow-up lift added `baseNameOf` / `dirOf` to the same
           4-lane corpus (`dirOf` needed a small `.px` helper; `baseNameOf`
           already existed in the `.px` host builtin set). Follow-up lift added
           `attrByPath` and `mapAttrsToList` to the same 4-lane corpus
           (`mapAttrsToList` exercises the px closure bridge). Follow-up lift
           added `stringToCharacters`, `range`, `last`, `init`, and `unique`
           to the 4-lane corpus. Follow-up lift added `concatMapStrings`,
           `concatMapStringsSep`, `id`, and `flip` to the 4-lane corpus.
           Follow-up lift added `replicate`, `hasInfix`, `count`,
           `zipListsWith`, `imap0`, and `imap1` to the 4-lane corpus.
           Follow-up lift added `findFirst`, `pipe`, and `recursiveUpdate` to
           the 4-lane corpus (`pipe` explicitly marshals functions inside the
           list before host application in the `.px` runtime). Follow-up lift
           added `foldr` to the 4-lane corpus. Follow-up lift added
           `splitString` and `toInt` success cases to the 4-lane corpus.
           Follow-up lift added `toLower` / `toUpper` ASCII corpus cases to the
           4-lane corpus. Follow-up lift added `genericClosure` traversal,
           duplicate-key, and empty-start cases to the 4-lane corpus.
           Follow-up lift added positive-integer `bitAnd` / `bitOr` / `bitXor`
           cases to the 4-lane corpus; `.px` negative signed-i64 bit semantics
           remain an explicit frontier instead of guessed behavior.
           Follow-up lift added `match` and `split` representative regex cases
           to the 4-lane corpus through Java regex lowering helpers.
           Follow-up lift added `fromJSON`, `listToAttrs` duplicate-first, and
           `cons` cases to the 4-lane corpus.
           Follow-up lift added `mapAttrs'`, `genAttrs`, `nameValuePair`,
           `foldlAttrs`, and `addErrorContext` cases to the 4-lane corpus.
           Follow-up lift added `isString`/`isAttrs`/`isList`/`isFunction`/
           `isFloat`/`isBool`/`isNull`/`isInt`, `typeOf`, `all`, `any`, and
           `foldl` alias cases to the 4-lane corpus.
   - [x] i64 overflow guards on integer arithmetic; int/int `+`/`-`/`*`/`/`/
         `%`, unary neg/abs, and builtin arithmetic now surface structured
         `:integer-overflow` instead of generic dispatch/eval failures or JVM
         wraparound (`Long/MIN_VALUE / -1`, `abs Long/MIN_VALUE`). Mixed
         int/float arithmetic stays numeric and bypasses int-overflow guards.
         Verified by `evaluator-integer-overflow-is-structured`.
   - [x] Source positions (`unsafeGetAttrPos`, `__curPos`) surface parser spans.
         Attrset key spans are retained as evaluator metadata (including nested
         dotted attr paths) and `__curPos` returns its var span; both expose
         `{"span" [start end] "start" start "end" end}` until file/line/column
         tracking exists. Verified by
         `evaluator-unsafe-get-attr-pos-surfaces-parser-spans` and
         `evaluator-cur-pos-surfaces-var-parser-span`.
   - [x] Domain stubs (`koreanFinalConsonantKind`, `pnixMounts`) are marked as
         non-faithful extensions, not Nix-builtin coverage. They now held with
         `:nix-builtin? false` + extension metadata; `pnixMounts` no longer
         returns a fake successful empty list, and zero-arity builtin selection
         dispatches instead of leaking a builtin map. Verified by
         `evaluator-domain-extension-stubs-are-not-nix-coverage`.
   - Pin as fixtures the known pitfalls: `//` update semantics, dynamic attrs,
     `rec`/`let` mutual recursion + self-reference black-holing.

### Axis 4 - conformance / differential / coverage  (second gap; turns Axis 1 into proof)

Reference methodology (Marmsoler & Brucker, TAP 2022): make the semantic lane an
executable oracle, grammar-fuzz type-correct programs, run differentially across
lanes, diff result-states, and MEASURE completeness by coverage of the executable
semantics (definitions / alternatives / expressions). Fuzzers systematically
under-cover error/edge branches, so generate error paths explicitly.

1. [x] **Grow the ground-truth corpus (M):** vendor real Nix-evaluator
   ground-truth cases as repo-owned EDN oracle fixtures (provenance-only, no
   external runtime dep — section G policy), expanding as each Axis-1 feature
   lands. The harness (`pnix/report`, oracle compare, first-mismatch) exists; it
   is fixture-starved, not infra-poor.
   Implemented first expansion as
   `resources/pnix_clj/oracles/ground_truth.edn`: 20 repo-owned cases captured
   from `nix-instantiate --eval --strict --json -E` (Nix 2.34.7 provenance
   recorded, no runtime shell-out). `oracle/ground-truth-cases` is now the smoke
   and strict/determinism/coverage source-family input; `literal-cases` remains
   as a compatibility API. Verified all 20 ground-truth rows are all-lane
   accepted before wiring them into the default corpus.
2. [x] **pnix-evaluation determinism chain (S-M):** run each source K times and
   assert hash-stable result/AST — cheap, and a needed guard once thunk memo
   state exists.
   Implemented as `pnix-clj.determinism`: repeats parse/eval over the repo-owned
   fixture corpus, hashes AST + canonical result projection, and reports
   `:stable`/`:unstable` rows. CLI aliases: `clojure -M:determinism [K]` and
   `clojure -M:report-determinism`. Report artifact:
   `target/pnix-clj/reports/determinism.edn`. Verification:
   `clojure -M:determinism 2` → 39 sources stable / 0 unstable;
   `clojure -M:report-determinism` → 39 stable / 0 unstable, 3 runs;
   `clojure -M:test` → 84 tests / 2049 assertions.
3. [x] **Coverage metric over the executable semantics (M):** instrument the
   evaluator dispatch so the corpus reports which constructs / builtins /
   branches are exercised; this is the honest "completeness" number, not a count
   of green fixtures.
   Implemented as `pnix-clj.coverage` plus evaluator `*coverage*` hooks. The
   default report runs the repo-owned fixture corpus and records dynamic
   coverage for AST ops, builtin dispatches, binary operators, and executable
   branches, with covered/missing/pct summaries against the current evaluator
   surface. CLI aliases: `clojure -M:coverage` and
   `clojure -M:report-coverage`. Current fixture coverage:
   sources 193, ops 22/22, builtins 79/148, binary operators 15/15,
   branches 10/10. Verification:
   `clojure -M:coverage` and `clojure -M:test` → 96 tests / 2426 assertions.
4. [x] **Grammar fuzzer + differential gate (M-L):** a pnix grammar generator
   feeding the existing 3-lane cross-check, with explicit error-path generation;
   diff result-states and default to held on any lane disagreement.
   Implemented as deterministic seed/index generation in
   `pnix-clj.grammar-fuzzer`. Positive generated programs must reach
   `run-source` `:accepted`; generated error-path programs must reach `:held`;
   any `:rejected` / lane disagreement or expectation mismatch fails the gate.
   CLI aliases: `clojure -M:grammar-fuzzer [positive-count error-count seed]`
   and `clojure -M:report-grammar-fuzzer`. Verification:
   `clojure -M -m pnix-clj.grammar-fuzzer 5 2 0` → 7 ok / 0 failed;
   `clojure -M:report-grammar-fuzzer` → 11 ok / 0 failed;
   `clojure -M:test` → 86 tests / 2076 assertions.
5. [x] **Optional live oracle (M, gated):** compare to a reference (vendored
   expected table, or — license permitting — a real Nix/pnix binary), failing
   open when absent. This is the only way to catch a *shared* (common-mode) lane
   error that 3-lane voting cannot.
   Implemented as `pnix-clj.live-oracle`: discovers `nix-instantiate`, evaluates
   generated positive programs through `--eval --strict --json`, compares the
   JSON value with pnix-clj `eval-source`, and returns `:skipped` with 0 exit
   when no command is available. Tests inject a fake oracle so CI is not coupled
   to local Nix. CLI aliases: `clojure -M:live-oracle [positive-count seed]`
   and `clojure -M:report-live-oracle`. Verification on this host:
   `clojure -M -m pnix-clj.live-oracle 5 0` → 5 matched / 0 mismatched;
   `clojure -M:report-live-oracle` → 5 matched / 0 mismatched;
   `clojure -M:test` → 87 tests / 2088 assertions.

### Axis 2 - host projection depth  (already role-complete; targeted only)

Reference: the canonical Clojure self-host path is `tools.analyzer.jvm` ->
`tools.emitter.jvm`; `emit-form` round-tripping proves macroexpansion is
preserved and `emit-hygienic-form` proves hygienic binding; namespace -> .class
projection plus `:debug?` bytecode dumps are receipt evidence. pnix-clj already
emits real, ASM-verified bytecode (it leads here).

1. [x] **Macro-tower projection depth (M):** extend the single `macroexpand-1`
   receipt to full `macroexpand-all` step traces, and project
   `defmacro`/syntax-quote/unquote/unquote-splicing/`gensym` structure
   (a compile-time syntax -> syntax view).
   Implemented in `pnix-clj.clojure-projection`: macroexpand fixtures now emit
   `MacroexpandReceipt` with `:phase`/`"phase" = "macroexpand-all-trace"`,
   normalized auto-gensym symbols, `final_term`, `steps`, `step_count`,
   setup/source/features metadata, and `.px` validation for the new fields.
   Added `macroexpand-defmacro-syntax-quote` covering `defmacro`,
   syntax-quote, unquote, unquote-splicing, and auto-gensym structure.
2. [x] **emit-form round-trip self-check (S-M):** add an analyzer -> `emit-form`
   round-trip as an extra clojure-mirror determinism receipt (cite tools.*).
   Implemented as `pnix-clj.emit-form-roundtrip`: analyzes representative
   Clojure forms with `clojure.tools.analyzer.jvm`, emits canonical forms with
   `clojure.tools.analyzer.passes.jvm.emit-form/emit-form`, evaluates original
   and emitted forms in fresh namespaces, and records value equality plus
   form/AST/emitted hashes. CLI aliases: `clojure -M:emit-form-roundtrip` and
   `clojure -M:report-emit-form-roundtrip`. Verification: 6 cases ok / 0 held;
   `clojure -M:test` → 92 tests / 2138 assertions.
3. [x] **Bidirectional synthesis + value roundtrip (M):** add a pnix-term ->
   Clojure-form synthesizer (reusing `lowering.clj` as the forward leg) and a
   value-roundtrip report (host pnix value vs JVM value of the synthesized form),
   plus an involution/closure check.
   Implemented as `pnix-clj.value-roundtrip`: evaluator value and lowering ->
   clj-meta forward value must agree, the normalized pnix value is synthesized
   into a canonical Clojure form, clj-meta evaluates that form back to the same
   JVM value, and value -> form -> value -> form closure must keep the same
   canonical form/hash. Functions/builtins/effectful host objects are held
   instead of guessed. CLI aliases: `clojure -M:value-roundtrip` and
   `clojure -M:report-value-roundtrip`.

### Axis 3 - runtime / mirror / trust depth  (role-complete; trust-framing work)

Reference ladder of trust (cite, do not over-claim): proof-grounded bootstrap
(CakeML) > semantic preservation (CompCert) > verified translation validation >
proof-carrying code > N-version voting > full-source bootstrap. pnix-clj sits at
the N-version / per-translation-receipt rung — heuristic, not proven.

1. [x] **Frame each receipt as an explicit translation-validation validator (M):**
   document each receipt as a per-translation `Validate(S,C)` checker that
   DEFAULTS TO HELD on uncertainty and is far simpler than clj-meta; record its
   residual TCB honestly (parser, host reader/analyzer, JVM) — cite CompCert's
   Csmith lesson that bugs live in the *unverified* edges, not the verified core.
   Implemented as `pnix-clj.translation-validation`: a validator catalog for
   parse, evaluator↔oracle, lowering↔clj-meta, compile receipt,
   px-runtime, pnix-mirror, cross-mirror, stage15 execution, and external live
   oracle, each with `Validate(S,C)` framing, default-on-uncertainty, failure
   status where applicable, and residual TCB. The report also runs a real
   `run-source` sample (`42`) and records sample validator outcomes. CLI aliases:
   `clojure -M:translation-validation` and
   `clojure -M:report-translation-validation`. Verification:
   status ok, 9 validators; `clojure -M:test` → 91 tests / 2129 assertions.
2. [x] **Execute stage15 rather than plan it (M):** stage15 is currently a
   read-only NOT-executed control plan (`:stage15-gates-not-executed`); run the
   clj-meta gates from a controlled harness so the meta-circular stage is
   actually executed, or keep it explicitly by-design as planned evidence.
   Implemented as `pnix-clj.stage15/execute-plan` and
   `pnix-clj.stage15-execute`: controlled clj-meta command execution with
   timeout, stdout/stderr hashes, duration, selected command ids, and receipt
   hash. Default bounded execution runs `:compiler-smoke`, `:conformance`, and
   `:determinism-policy`; heavier plan rows (`:gate`,
   `:full-source-stage1`, etc.) remain explicit by id. CLI aliases:
   `clojure -M:stage15-exec [comma-ids timeout-ms]` and
   `clojure -M:report-stage15-exec`. Verification:
   `clojure -M:stage15-exec` → 3 commands ok / 0 held;
   `clojure -M:report-stage15-exec` → status ok, commands 3, held 0;
   `clojure -M:test` → 90 tests / 2117 assertions.
3. [x] **Common-mode risk note (S):** because the lanes share semantics, record
   (per Knight-Leveson) that cross-lane voting reduces but does not eliminate
   correlated failure — the Axis-4 independent oracle is the mitigation.
   Implemented as `pnix-clj.trust`: records the correlated-failure claim
   boundary, shared TCB (`pnix-parser`, evaluator semantics, lowering,
   clj-meta/JVM), residual risks, and mitigations (`live-oracle`,
   `grammar-fuzzer`, `coverage`, `determinism`, `mirror-error`). CLI aliases:
   `clojure -M:trust` and `clojure -M:report-trust`. Verification:
   `clojure -M:report-trust` → status ok, 5 mitigations;
   `clojure -M:test` → 89 tests / 2108 assertions.
4. [x] **Deterministic-classfile receipts (S):** pin the ASM artifact/version
   (`org.ow2.asm` vs shaded `clojure.asm` drift) so bytecode receipts stay
   byte-deterministic, and account for deftype/defrecord/reify/proxy emitted
   classes in any classfile enumeration.
   Implemented as `pnix-clj.classfile-receipt`: pins `org.ow2.asm/asm-util`
   9.7.1 from pnix-clj and clj-meta gate/verifier aliases, records shaded
   `clojure.asm` as clojure-runtime-owned, and enumerates bytecode/verified
   class counts + hashes for pnix compile and Clojure generated class forms.
   Added `proxy-runnable-form` to the clojure-form corpus so enumeration covers
   `deftype`, `defrecord`, `reify`, and `proxy`. CLI aliases:
   `clojure -M:classfile-receipt` and `clojure -M:report-classfile-receipt`.
   Verification: classfile receipt status ok, 5 rows; full gate above.

### Sequencing (by completeness impact)

1. Axis 1 laziness/thunks ->
2. Axis-1 quick parser wins + `with` / indented strings / paths ->
3. Axis 4 corpus + determinism chain + coverage metric ->
4. Axis 1 string-context -> derivations -> import/scopedImport ->
5. Axis 1 builtin breadth (fixture cadence) ->
6. Axis 4 grammar fuzzer + optional live oracle ->
7. Axis 2/3 depth (macro tower, synthesis, stage15 execution, trust-framing).

Verified sources: nix.dev evaluation manual; tvl.fyi "rewriting Nix" (Tvix);
shealevy string-context; clojure/tools.analyzer.jvm + tools.emitter.jvm;
clojure.org/reference/compilation; jgpc42/insn; CakeML dissertation (Kumar 2017);
Leroy CACM "Formal verification of a realistic compiler" (CompCert);
Pnueli/Siegel/Singerman TACAS'98 (translation validation); N-version programming
(Avizienis/Chen; Knight-Leveson 1986); Guix full-source bootstrap; Marmsoler &
Brucker TAP 2022 (differential conformance + coverage).

---

## clj-meta / pnix-clj Separation (architecture)

Full plan: `clj-meta-separation.md` (next to this file). Summary and tracker.

Branch reality: this is `feat/clj-meta-metacircular`, a clean clj-meta-hosted
rewrite (0 behind / 407 ahead of `origin/main`). `origin/main` is a different,
older line (content-addressed `cas.clj`, append-only `store.clj`, `stage.clj`,
`purity.clj`, gate-graph, 67-lang emit) — those are MAIN-ONLY reference assets to
port from if needed, NOT pillars missing from this branch. Classify only by this
branch's files.

Corrected principle: meta-circular capability is NOT just "mirror" — mirror is one
observation surface. Three layers, one bridge:

```
clj-meta = Clojure/JVM meta-circular compiler/evaluator PROOF lane (mature; consume it)
pnix-clj = pnix runtime on top of clj-meta
interop  = explicit bidirectional Clojure/JVM <-> pnix bridge (not a mirror)
```

Sharper model: **clj-meta is pnix-agnostic** and completes Clojure(JVM)
meta-circularity on its own (self-host ladder, kernel, import hook, artifact,
introspection — clj-meta's todo). **pnix-clj is purely the pnix layer; its host
IS clj-meta**, so it must not re-do host proof. pnix-clj's actual core =
`parser`/`evaluator`/`lowering`/`px_runtime`/`mirror`/`receipt` (the pnix
language). `clojure_form.clj` (host Clojure eval vs clj-meta compile agreement)
and the host-reflection half of `clojure_projection.clj` are
**Clojure-about-Clojure host-domain** work (clj-meta's lane, reached via interop),
NOT pnix core.

Verified current state (read before refactoring):
- clj-meta (`../clj-meta`) already owns the host proof lane (compiler, verified_
  compile, bytecode_verifier/witness, determinism_policy, translation_validation,
  conformance, kernel, selfhost, mirror, stm, gate). "Move to clj-meta" usually
  means "consume its API", not "build new".
- Host machinery in pnix-clj is confined to exactly THREE files: `clj_meta.clj`
  (interop seam), `clojure_form.clj` (host `eval`), `clojure_projection.clj`
  (large host reflection/introspection). These are the misplacement to fix.
- Absent today (TARGET, not current): README / `bin/stage3-gate` / launcher, and
  any CAS / store / term / stm / resolve / module / loss / verifier / search
  namespaces. The plan's event-store/CAS architecture is future, not present.

Phased refactor (each a gate-green slice; ★ = host machinery to relocate):
- [x] **A. Formalize the interop seam** — `clj_meta.clj` is now the explicit
      clj-meta host-compile interop client: its `eval-lowered` results carry an
      `:interop` classification (`:pnix-lowered-form->clj-meta-compile`,
      effect-class `:host-compile`, loss `:lossless`). Consumers ignore the extra
      key (select-keys), so behavior is unchanged. (A+C together establish the
      interop seam; capability/witness fields are added as the boundary grows.)
- [x] **B. ★ Extract projection host crossings from `clojure_projection.clj`**
      behind a host-side interop API:
      `pnix-clj.clojure-projection.host` now owns reader/form parsing,
      fresh-host-ns crossing, capability grants, interop metadata, witnesses,
      and host eval used by the Var/NS/Throwable/Class/Java-object/reflection/
      classloader/macroexpand/dynamic-binding/host-object/polymorphism/metadata/
      state/lazy/concurrency/coordination snapshot fixtures. The parent
      projection namespace keeps term construction, `project-reader-value`,
      `validate-term`, and report assembly; JVM objects still project as
      canonical `JavaObject` envelopes, never raw host objects. Verified by
      `clojure -M:clojure-projection` (45/45 accepted).
- [x] **C. ★ Route host `eval` out of `clojure_form.clj`** — new
      `pnix-clj.interop` namespace (host-side adapter) owns `fresh-host-ns` +
      `host-eval-form`, tagged with an interop classification
      (`:pnix-clj.interop.v0`: direction/effect-class/loss-status). `clojure_form`
      now calls `interop/host-eval-form`; the host-vs-clj-meta agreement stays a
      CHECK. (clojure_projection's own host helpers move under Phase B.)
- [x] **D. Consolidate the runtime mirror into a singleton** `run-mirror` with
      trace facets — `core/run-source` now calls `pnix-clj.mirror/run-mirror`
      once and exposes that owner receipt as `:mirror-run`; mirror row
      constructors remain implementation helpers under the singleton entrypoint.
- [x] **E. Delegate compile proof** (determinism/verified/bytecode) in
      `clj_meta.clj` to clj-meta — new
      `pnix.clj-meta.form-proof/compile-receipt` owns the per-form determinism,
      strict, bytecode-artifact, and verified-compile rows. pnix-clj now only
      compiles/evaluates the lowered form and calls that clj-meta API, with
      receipt `:proof-owner` recording the clj-meta proof lane and related global
      proof APIs (`determinism-policy`, `bytecode-witness`, `verified-compile`).
      Verified by `clojure -M:test` (76/1436) and `clojure -M:mirror-pair`
      (9 accepted / 0 held).
- [ ] **F. (only if a roadmap item needs it)** PORT content-addressed terms /
      event store / snapshot-resolve from `origin/main` (`cas.clj`/`store.clj`/
      `term.clj`/`resolve.clj`), adapting to this branch's value model — explicit
      branch comparison, not a standing scope.

This refactor is independent of the Completeness Roadmap feature work; sequence
them as the developer prefers (interop/mirror hygiene vs language depth).

Research-grounded interop principles (2026-07-01 /deep-research, verified;
full table in `clj-meta-separation.md` §10 — host/guest/interop distribution):
- **Deny-by-default boundary** (GraalVM Truffle): nothing from the host floor is
  reachable from pnix until explicitly exported; add one capability at a time.
- **Classify every crossing by effect class** (pure/host-call/host-eval/
  host-compile/macroexpand/dynamic-binding/reflection/require/resolve-var/
  classloader/file/network/mutation/time/random/thread); the interop layer is
  the only place that tags + gates them.
- **Opaque handles, never value-serialization**: host objects cross as opaque
  refs and MUST NOT enter a pnix canonical/content-addressed term (fixes
  `java-object-term`); object-capability discipline (authority only via handles).
- **Content-addressed cross-layer trust**: hash the *normalized* AST; keep human
  names as separate metadata (Unison model). PORT `cas.clj`/`store.clj` from
  origin/main when CAS/store land.
- Honest: cross-layer agreement is **heuristic, not sound**; lazy guest on eager
  Clojure needs explicit guest thunks (done); coarse host grants "grant all";
  singleton-mirror is our design choice, not an externally proven law.

Interop hardening progress (the distribution-sequencing step 1):
- [x] Opaque host-ref registry + value marshalling in `pnix-clj.interop`
      (`make-opaque-host-ref`/`opaque-host-ref?`/`opaque-ref-deref`/
      `release-opaque-ref!`, `host-object?`, `from-host`/`to-host`): host/JVM
      objects cross as opaque refs (Kernel-FFI pattern) and `host-object?` flags
      anything that must not enter a pnix canonical term; pure pnix values pass
      through. Verified by `interop-opaque-host-refs-and-marshalling`. (Library is
      built; wiring it into the evaluator/projection host-object paths and the
      effect-class/capability gate are the next interop slices.)
- [x] Effect-class taxonomy + deny-by-default capability gate in
      `pnix-clj.interop` (`effect-classes`, `effect-class?`, `check-capability`
      with `default-capabilities` = `{:pure}`): only `:pure` crosses without an
      explicit grant; unknown effect classes are held, not silently allowed.
      Verified by `interop-effect-class-capability-gate`.
- [x] Interop crossing witness in `pnix-clj.interop` (`make-witness`/`witness?`):
      pure, content-hashed evidence (direction/effect-class/loss-status/in+out
      hashes) — deterministic, distinct fields give distinct hashes, so it can
      live in an evidence log. Verified by `interop-crossing-witness`.
- [x] Live host-eval and host-compile crossings are wired through the gate:
      `interop/host-eval-form` defaults to denied and `clojure-form` passes an
      explicit `:host-eval` grant; `clj-meta/eval-lowered` passes an explicit
      `:host-compile` grant. Both attach witnesses to every result. Verified by
      `interop-host-eval-is-gated-and-witnessed`,
      `clojure-form-report-compares-host-and-clj-meta-semantics`, and
      `run-source-closes-mirror-spine-for-small-source`.
- [x] Projection host-term fixture helpers now run behind `interop/run-crossing`
      and expose row-level `:host-crossing` evidence. Effects are explicit
      (`:host-eval`, `:macroexpand`, `:dynamic-binding`, `:host-call`,
      `:reflection`, `:classloader`, `:resolve-var`, `:global-mutation`,
      `:thread`) and each crossing has a grant + witness. Verified by
      `clojure-projection-report-validates-reader-term-shape-in-px` and
      `clojure -M:clojure-projection` (43 accepted / 0 held).

---

## Master Checklist

### A. Repo and dependency structure

- [x] `deps.edn` exists in `pnix-clj`.
- [x] `../clj-meta` is a local/root dependency.
- [x] No Python/Hy runtime modules are introduced.
- [x] Smoke namespace can require/use `pnix.clj-meta.compiler`.
- [x] CI/local script runs pnix-clj smoke plus selected clj-meta gates.

### B. Parser and grammar parity

- [x] Literal parser.
- [x] Identifier/path parser.
- [x] Identifier parser first slice.
- [x] Attr path parser first slice.
- [x] Let/if/function/call parser.
- [x] Let/arithmetic parser first slice.
- [x] Function call parser first slice for selected builtins.
- [x] If/comparison/lambda parser first slice.
- [x] Basic literal attrset/list parser.
- [x] List concat parser first slice.
- [x] String interpolation parser first slice.
- [x] Parenthesized call parser inside list items.
- [x] Dynamic attr key parser first slice.
- [x] String context syntax.
- [x] Import syntax first slice for repo-owned relative `.px` runtime artifacts.
- [x] Runtime grammar first slice: comments, attr-exists `?`, logical
      `!`/`&&`/`||`, `!=`, dynamic select, and attrset `inherit`.
- [x] Unsupported syntax ledger with source span.
- [x] First literal/list/attrset oracle import into repo-owned static fixtures.

### C. Evaluator semantics

- [x] Literal value model.
- [x] Basic list/attrset value model.
- [x] Basic strict let environment model.
- [x] Recursive function environment first slice.
- [x] Lazy `let` bindings (memoized thunks + recursive `let`) in the evaluator
      lane — unused bindings unevaluated, forward/mutual refs and recursive
      values resolve, self-reference black-holed. See Completeness Roadmap Axis 1.
- [x] Full laziness-aware recursive environment model core slice. `let`,
      function arguments, recursive attrsets, attrset values, and list items are
      lazy/call-by-need across evaluator, clj-meta lowering, and `.px` runtime
      lanes; remaining work is builtin-by-builtin exactness, not the environment
      model itself.
- [x] Laziness/strictness model core slice. Default eval is call-by-need where
      Nix requires it; opt-in strict mode covers audited non-Nix leniencies.
      Remaining laziness work is exact builtin forcing taxonomy.
- [x] Builtin dispatch first slice: `attrNames`, `hasAttr`.
- [x] Builtin dispatch first slice: `map`.
- [x] Builtin dispatch first slice: `sort`, `head`, `tail`, `elemAt`, `elem`.
- [x] Builtin dispatch first slice: `length`, `filter`, `foldl'`.
- [x] Builtin dispatch first slice: `stringLength`, `concatStringsSep`,
      `substring`, `toString`, `toJSON`.
- [x] Runtime bootstrap builtin first slice: `isString`, `isAttrs`, `isList`,
      `isInt`, `isBool`, `isNull`, `all`, `any`.
- [x] Runtime evaluator builtin first slice: `isFunction`, `isFloat`, `typeOf`,
      `baseNameOf`, `pathExists`, `cons`, `match`, `replaceStrings`, `split`,
      scalar `fromJSON`, `listToAttrs`, `throw`/`abort` held rows.
- [x] Runtime evaluator builtin next slice: attr/list aliases and arithmetic
      names.
      `removeAttrs`, `concatLists`, `concatMap`, `append`, `take`, `drop`,
      `reverseList`, `add`, `sub`, `mul`, `div`, and `lessThan` now close
      through evaluator, clj-meta lowering, internal `.px` runtime, and pnix
      mirror for curated fixtures.
- [x] Runtime evaluator builtin next slice: attr/list accessor and transform
      aliases.
      `get`, `set`, `keys`, `values`, `merge`, `find`, `zip`, `flatten`, and
      `catAttrs` now close through all runtime/mirror lanes for curated fixtures.
- [x] Runtime evaluator builtin next slice: higher-order attr/list transformers.
      `mapAttrs`, `filterAttrs`, `intersectAttrs`, `groupBy`, and `partition`
      now close through all runtime/mirror lanes for curated fixtures.
- [x] Runtime evaluator builtin next slice: version/env helpers.
      `currentSystem`, `nixVersion`, `storeDir`, `compareVersions`,
      `splitVersion`, and `parseDrvName` now close through all runtime/mirror
      lanes for curated fixtures.
- [x] Runtime evaluator builtin next slice: direct strictness/effect helpers.
      Direct `tryEval`, `seq`, `deepSeq`, and `trace` now close through all
      runtime/mirror lanes for curated fixtures; aliased lazy `tryEval` remains
      a later laziness slice.
- [x] Runtime evaluator builtin next slice: generated list helper.
      `genList` now closes through all runtime/mirror lanes for curated
      fixtures and unblocks internal mirror-plate uses that need index lists.
- [x] Runtime evaluator builtin next slice: attrset zip aggregation.
      `zipAttrsWith` now closes through all runtime/mirror lanes for curated
      fixtures with deterministic key-union ordering.
- [x] Runtime evaluator builtin next slice: boolean/comparison/math aliases.
      `and`, `or`, `not`, `eq`, `lt`, `le`, `gt`, `ge`, `mod`, `neg`, `abs`,
      `pow`, `sqrt`, `floor`, `ceil`, `exp`, `ln`, `sin`, `cos`, and `atan2`
      now close through all runtime/mirror lanes for curated fixtures.
- [x] Runtime evaluator builtin next slice: direct `functionArgs` metadata.
      Inline attr-pattern lambdas such as `{ x, y ? 1, ... }: ...` now parse
      through Clojure and internal `.px` source lanes, and `functionArgs`
      returns named argument/default metadata without opening full pattern
      lambda application semantics.
- [x] Bootstrap runtime evaluator `getAttr` and curried closure env preservation
      first slice.
- [x] Internal `.px` evaluator strict user-let first slice.
      Removes the `rec_env` host-laziness dependency for non-recursive corpus
      cases; recursive user lets remain held honestly at the runtime frontier.
- [x] Internal `.px` evaluator recursive direct-lambda closure first slice.
      Direct lambda bindings carry their let lambda set and reconstruct the
      recursive function environment at apply time; general lazy let remains
      outside this slice.
- [x] Fixed-stack internal `.px` source execution.
      Runs source execution on a 32 MiB stack so recursive pnix runtime cases do
      not depend on the launcher thread stack.
- [x] Nix-style integer division first slice.
- [x] Dynamic attr key evaluator first slice.
- [x] Internal `.px` parser/evaluator dynamic string attr-key first slice.
      Allows `"k${...}"` attrset keys to evaluate inside runtime execution.
- [x] List concat evaluator first slice.
- [x] Fixed-stack evaluator lane for recursive corpus cases.
- [x] Non-recursive closure/call evaluator first slice.
- [x] Full builtin dispatch.
- [x] Structured errors.
      Evaluator held paths now attach `:pnix-clj.error.v0`, and mirror-error
      fixtures require eval schema/phase/reason alignment before acceptance.
- [x] Import/cache/cycle behavior.
      `runtime-run-plan` includes `:pnix-clj.px-runtime.import-graph.v0`, and
      runtime bootstrap/artifact evaluation/source runner receipts include
      `:pnix-clj.px-runtime.import-cache.v0`.
- [x] Deterministic literal normalization for comparison.

### D. Lowering to Clojure forms

- [x] Literal lowering.
- [x] Basic variable/let lowering.
- [x] Let/if/function/call lowering.
- [x] Let/arithmetic lowering first slice.
- [x] If/lambda/call lowering first slice.
- [x] Recursive all-lambda `let` to `letfn` lowering first slice.
- [x] Basic attr/list lowering.
- [x] Attr merge/select lowering first slice.
- [x] Builtin lowering policy first slice: `attrNames`, `hasAttr`.
- [x] Builtin lowering policy first slice: `map`.
- [x] Builtin lowering policy first slice: `sort`, `head`, `tail`, `elemAt`, `elem`.
- [x] Builtin lowering policy first slice: `length`, `filter`, `foldl'`.
- [x] Builtin lowering policy first slice: `stringLength`, `concatStringsSep`,
      `substring`, `toString`, `toJSON`.
- [x] Builtin lowering policy next slice: attr/list aliases and arithmetic names.
      Matches the evaluator slice for `removeAttrs`, `concatLists`, `concatMap`,
      `append`, `take`, `drop`, `reverseList`, `add`, `sub`, `mul`, `div`, and
      `lessThan`.
- [x] Builtin lowering policy next slice: attr/list accessor and transform
      aliases.
      Matches the evaluator slice for `get`, `set`, `keys`, `values`, `merge`,
      `find`, `zip`, `flatten`, and `catAttrs`.
- [x] Builtin lowering policy next slice: higher-order attr/list transformers.
      Matches the evaluator slice for `mapAttrs`, `filterAttrs`,
      `intersectAttrs`, `groupBy`, and `partition`.
- [x] Builtin lowering policy next slice: version/env helpers.
      Matches the evaluator slice for `currentSystem`, `nixVersion`,
      `storeDir`, `compareVersions`, `splitVersion`, and `parseDrvName`.
- [x] Builtin lowering policy next slice: direct strictness/effect helpers.
      Matches the evaluator slice for direct `tryEval`, `seq`, `deepSeq`, and
      `trace`; alias-preserving laziness is not claimed here.
- [x] Builtin lowering policy next slice: generated list helper.
      Matches the evaluator slice for `genList`.
- [x] Builtin lowering policy next slice: attrset zip aggregation.
      Matches the evaluator slice for `zipAttrsWith`.
- [x] Builtin lowering policy next slice: boolean/comparison/math aliases.
      Matches the evaluator slice for `and`, `or`, `not`, `eq`, `lt`, `le`,
      `gt`, `ge`, `mod`, `neg`, `abs`, `pow`, `sqrt`, `floor`, `ceil`, `exp`,
      `ln`, `sin`, `cos`, and `atan2`.
- [x] Builtin lowering policy next slice: direct `functionArgs` metadata.
      Matches the evaluator slice by reading lambda parameter metadata directly
      and returning `{name -> has-default?}` without lowering the pattern
      lambda body.
- [x] Nix-style integer division lowering first slice.
- [x] Dynamic attr key lowering first slice.
- [x] List concat lowering first slice.
- [x] Mixed lambda-prefix `let` lowering to `letfn` plus sequential `let`.
- [x] Full builtin lowering policy.
- [x] Stable generated names.
      Lowering cache keys are `{:ast-hash ... :policy ...}` and lowered forms
      keep `:source-string-codegen? false`; no global generated-name counter is
      used for receipts. Verified by `lowering-cache-is-keyed-by-ast-hash-and-policy`.
- [x] Unsupported lowering held reasons.
      Unsupported attr keys/ops, pattern lambdas, recursive dynamic attr keys,
      and imports return structured held reasons instead of falling through as
      successes.
- [x] Lowered form hash.

### E. clj-meta integration

- [x] Load `pnix.clj-meta.compiler`.
- [x] `compile-form*` integration for lowered literals.
- [x] strict compile integration.
      `pnix-clj.clj-meta/eval-lowered` now attaches a
      `compile-form-strict` row to the compile receipt. Strict failure is
      preserved as evidence, while strict success must agree with the primary
      value hash.
- [x] compile diagnostics/fallback capture.
- [x] bytecode hash capture when available.
      pnix-clj now calls clj-meta `compile-to-dir` for lowered wrapper forms,
      records per-class hashes, and exposes the aggregate artifact hash as
      `:bytecode-hash`.
- [x] determinism policy connection.
      Compile receipts include primary/repeat determinism, strict direct
      value agreement when available, and disabled compiled-artifact cache
      policy to preserve the clj-meta classloader boundary.
- [x] verified compile connection when forms become production runtime.
      Compile receipts now attach clj-meta `compile-classes-verified`
      verifier summaries and class hashes as evidence. The row is not yet a
      hard acceptance requirement for all lowered forms; it is present as the
      production artifact admission hook.

### F. Mirror and evidence

- [x] Receipt schema.
- [x] Accepted/rejected/held lifecycle.
- [x] Evaluator vs clj-meta comparison.
- [x] Clojure mirror row.
- [x] Stage15 clj-meta control-plan receipt.
- [x] Stage15..N clj-meta backend role and mirror-spine metadata in receipts.
- [x] `.px` runtime artifact hash.
- [x] `.px` runtime import graph / run-plan receipt.
- [x] `.px` runtime import cache/cycle receipt.
      Bootstrap/artifact/source-runner paths expose repo-owned artifact cache
      key policy, miss/hit counts, and held-on-cycle policy; run-plan exposes
      graph acyclicity and missing import counts.
- [x] `.px` runtime entry parse + VM bootstrap receipt.
      `pnix-mirror-runtime/vm.px` evaluates with P1-P12 imports from internal
      resources; acceptance still requires a separate source execution row and
      value comparison.
- [x] `.px` runtime source execution first slice.
      `runtime-source-execution` configures the internal `.px` evaluator with
      `exec/runtime.px`, runs small source expressions, and pnix mirror compares
      the runtime value against evaluator/clj-meta before acceptance.
- [x] Pnix mirror row.
- [x] Cross-mirror verdict.
- [x] Oracle comparison for literal/list/attrset fixture rows.
- [x] First mismatch report.
- [x] Held count and first-held report.
- [x] First frontier lane report.
- [x] Persisted report artifact.

### G. Internal `.px` runtime and copied pnix corpus

- [x] Inventory internal `.px` artifacts under `resources/pnix_clj/pnix_runtime`.
- [x] Copy `pnix-mirror-runtime`, `pnixc-pnix`, and selected `stdlib` entries into
      `resources/pnix_clj/pnix_runtime`.
- [x] Resolve `pnix-mirror-runtime/vm.px` P1-P12 imports from repo-owned files.
- [x] Identify small stable literal/list/attrset oracle rows.
- [x] Import/copy mirror runtime as repo-owned `.px` artifacts.
- [x] Import/copy Rust-grounded invariance corpus as repo-owned fixtures.
- [x] Track `RUST_*_CORPUS` and stage7 core case suites in a repo-owned manifest.
- [x] Keep original source path and commit/hash when available.
- [x] Separate unsupported syntax from semantic mismatch.
- [x] Persist smoke/rust batch reports as `target/pnix-clj/reports/*.edn`.
- [x] Bootstrap `pnix-mirror-runtime/vm.px` with repo-owned P1-P12 imports.
      The run-plan records `:px-runtime-bootstrap-ok` and
      `:px-runtime-run-plan-ready-source-required`; source-specific execution is
      recorded separately.
- [x] Execute first small source expressions through repo-owned
      `pnixc-pnix/eval/evaluator.px`.
      Smoke/oracle literal cases, all 5 stage7 structural lock-ins, and all 10
      Rust-grounded fixtures now close as accepted. Unsupported old Rust output is
      retained only as provenance evidence where it is not the current
      pnix-clj projection oracle.
- [x] Complete internal pnix runtime mirror generation first slice.
      The `.px` runtime must be able to emit pnix-side mirror evidence for its
      own parse/eval/result/error behavior, not only return a host-observed
      value through `runtime-source-execution`.
      `pnixc-pnix/eval/evaluator.px` now exports `runMirror`, and
      `runtime-source-execution` includes the `.px`-emitted mirror receipt in
      `:pnix-mirror-receipt`; `pnix-mirror-row` derives its value from that
      runtime receipt rather than only wrapping the host-observed value.
- [x] Complete clojure mirror generation for the runtime lane first slice.
      Clojure/clj-meta/JVM receipts must include stable form, compile/eval,
      bytecode/determinism, and fallback evidence for the runtime path being
      compared against pnix mirror rows.
      `pnix-clj.clj-meta/eval-lowered` now emits a compile receipt with primary
      and repeat compile/eval rows, generated class name hash, diagnostics hash,
      determinism verdict, and fallback mode/count evidence. Bytecode hash
      capture remains a later clj-meta API integration.
- [x] Add bytecode hash capture to clojure mirror rows when clj-meta exposes a
      stable per-form artifact API.
      clj-meta `compile-to-dir` provides the per-form class artifact boundary;
      pnix-clj records only deterministic class hashes and deletes temporary
      files after receipt construction.
- [x] Add first mirror-pair self-test fixtures and report.
      `clojure -M:mirror-pair` and `clojure -M:report-mirror-pair` now show both
      mirror rows, the cross-mirror verdict, value hashes, and the
      held/rejected/accepted lifecycle for curated internal runtime basics. Grow
      this before touching broad host-language projection.
- [x] Add first mirror-error self-test fixtures and report.
      Expected evaluator failures now have a separate acceptance gate:
      `clojure -M:mirror-error` and `clojure -M:report-mirror-error` verify that
      Clojure evaluator held reasons and internal `.px` runtime mirror error
      receipts agree for unknown var, missing attr selection, and recursive
      forward-reference boundaries. These expected failures stay out of the main
      success-value acceptance count.
- [x] Define Clojure projection term shape first slice inside the internal `.px`
      runtime.
      `pnixc-pnix/clojure/projection.px` now owns
      `pnix-clj.clojure-projection.v0` for Scalar/Symbol/Keyword/List/Vector/
      Map/Set terms, plus recursive validation and `.px` self-test evidence.
- [x] Add first host-faithful Clojure reader projection fixtures.
      `clojure -M:clojure-projection` and
      `clojure -M:report-clojure-projection` read Clojure values with host
      Clojure, project them to pnix terms, and validate the terms through the
      internal `.px` projection artifact.
- [x] Extend Clojure projection terms first runtime-object slice.
      The internal `.px` projection validator now accepts Var, Namespace,
      Exception, and ControlFlowReceipt terms. `clojure-projection` fixtures
      project real host Var/Namespace/Throwable values and a closed-form
      control-flow receipt through the same internal validator.
- [x] Extend Clojure projection terms for macroexpand receipts, dynamic binding,
      and Java interop first envelopes.
      `clojure-projection` now emits host-local MacroexpandReceipt,
      DynamicBindingReceipt, and JavaInteropReceipt terms and validates them
      through the internal `.px` projection artifact.
- [x] Add richer control-flow receipts for exception/finally effect traces.
      `ControlFlowReceipt` now carries an `effects` term list; the first trace
      fixture records a `try/catch/finally` result with the `:finally` effect.
- [x] Add first host-faithful Clojure form semantics fixtures after reader data.
      Start with `fn`, `let`, `letfn`, `loop/recur`, `do`, `if`, `case`, and
      `try/catch/finally` before touching macro or namespace-heavy cases.
- [x] Add first closed-form host/clj-meta semantics fixtures.
      `clojure -M:clojure-form` and `clojure -M:report-clojure-form` now compare
      host Clojure eval with clj-meta compile/eval for small closed forms
      (`fn` application, `let`, `if`, `do`, `try/catch`) and validate the same
      forms as pnix projection terms through the internal `.px` validator.
- [x] Expand closed-form host/clj-meta semantics fixtures through control forms.
      The same `clojure-form` gate now includes `letfn`, `loop/recur`, `case`,
      and `try/finally` fixtures before macro or namespace-heavy cases.
- [x] Grow corpus only after the receipt/report path is stable.
      `clojure-form` now includes first host-language semantics fixtures for
      macroexpand, namespace object naming, var object naming, dynamic binding,
      and Java instance/static interop after the report/receipt path stabilized.
- [x] Expand closed-form host/clj-meta semantics through Clojure function shape.
      `clojure-form` now compares host Clojure and clj-meta for multi-arity
      functions, variadic rest args, vector/map destructuring, and a bounded
      `locking` monitor form, while still validating the source form through
      the internal `.px` projection validator.
- [x] Expand closed-form host/clj-meta semantics through collection transforms.
      `clojure-form` now includes direct clj-meta fixtures for `reduce`, `mapv`,
      `filterv`, `into` with transducer, `transduce`, lazy `sequence` with
      composed transducers, and transient vector roundtrip.
- [x] Expand closed-form host/clj-meta semantics through object model forms.
      `clojure-form` now includes direct clj-meta fixtures for `deftype`,
      `defrecord`, `reify`, and `defmulti` dispatch, using stable scalar/
      collection results rather than generated JVM class names.
- [x] Expand closed-form host/clj-meta semantics through JVM interop forms.
      `clojure-form` now includes direct clj-meta fixtures for Java field read/
      write, `instance?`, char-array overload dispatch, static overload dispatch,
      and argument-bearing constructor calls.
- [x] Expand closed-form host/clj-meta semantics through reader/value literals.
      `clojure-form` now includes direct clj-meta fixtures for BigInt,
      BigDecimal, Ratio, regex literals, quoted lists, sets, and nested
      collection literals, comparing stable values/class names through the
      internal `.px` projection validator.
- [x] Expand closed-form host/clj-meta semantics through runtime namespace/Var
      resolution forms.
      `clojure-form` now includes direct clj-meta fixtures for
      `requiring-resolve`, `find-var`, `resolve`, `find-ns`, `intern`,
      `alter-var-root`, `ns-publics`, and `all-ns` observations. Same-form
      compile-time alias/import use remains outside this slice until the
      clj-meta namespace boundary can represent it without held rows.
- [x] Expand JVM projection beyond a single Java interop envelope.
      `clojure-projection` now has first-class `JavaClass` and `JavaObject`
      pnix terms plus JavaInteropReceipt class metadata. Fixtures cover instance
      call, static call, static field, constructor object, and class object
      receipts through the internal `.px` projection validator.
- [x] Add JVM reflection/classloader projection receipts.
      `clojure-projection` now emits `ReflectionReceipt` and
      `ClassloaderReceipt` terms for declared field lookup, method return type
      lookup, system classloader capture, and classloader-driven class loading.
      These remain host-local evidence envelopes, not cross-host parity claims.
- [x] Add namespace/import resolution projection receipts.
      `NamespaceResolutionReceipt` now records require-alias resolution,
      Java class import resolution, and local Var resolution from a fresh host
      Clojure namespace, then validates those receipts through the internal
      `.px` projection artifact.
- [x] Add host object construction projection receipts.
      `HostObjectConstructionReceipt` now records `deftype`, `defrecord`, and
      `reify` construction evidence, preserving JVM object identity, interface
      membership, record-ness, and projected value shape through the internal
      `.px` validator.
- [x] Add polymorphism dispatch projection receipts.
      `PolymorphismDispatchReceipt` now records host-local `defprotocol` and
      `defmulti` dispatch evidence, including observed dispatch value,
      projected args/classes, and projected result through the internal `.px`
      validator.
- [x] Add metadata projection receipts.
      `MetadataReceipt` now records `with-meta` and Var metadata evidence,
      including projected target, metadata map, and observed result through the
      internal `.px` validator.
- [x] Add state/effect projection receipts.
      `StateEffectReceipt` now records first host-local `atom` and `volatile!`
      state transitions as initial/final/result/effect evidence through the
      internal `.px` validator without claiming JVM atomicity proof.
- [x] Add lazy/deferred evaluation projection receipts.
      `LazyEvaluationReceipt` now records first host-local `delay`/`force` and
      `lazy-seq` realization evidence, including realized count, effect markers,
      and projected result through the internal `.px` validator without claiming
      complete laziness proof.
- [x] Add concurrency projection receipts.
      `ConcurrencyReceipt` now records first bounded host-local `future` and
      `promise` observations, including completion flag, effect markers, and
      projected result through the internal `.px` validator without claiming
      thread scheduling proof.
- [x] Add coordination projection receipts.
      `CoordinationReceipt` now records first bounded host-local STM `ref`/
      `dosync`/`alter` and `agent`/`send`/`await` observations, including
      initial/final/result/effect evidence through the internal `.px` validator
      without claiming STM isolation or agent scheduling proof.

### H. Runtime API

- [x] `parse-source`.
- [x] `eval-source`.
- [x] `lower-source`.
- [x] `compile-source` wrapper for pnix, not to be confused with
      `pnix.clj-meta.compiler/compile-form`.
- [x] All public calls return structured result maps or throw documented ex-info.
      `compile-source` now returns a `:pnix-clj.compile-source.v0` map with
      parse/lower/clj-meta compile receipt evidence and keeps acceptance in
      `run-source`/`report` gates.
- [x] `run-source`.
- [x] `report`.

### I. Performance and production hardening

- [x] Avoid source-string codegen in hot paths.
      Lowering emits Clojure data forms with `:source-string-codegen? false`;
      no `CLJ_AST_*_SOURCE` string blob path is introduced.
- [x] Cache parsed AST by source hash.
      `pnix-clj.parser/parse-source` now uses a source-hash cache with
      deterministic result maps and separate cache stats.
- [x] Cache lowered forms by AST hash and lowering policy.
      `pnix-clj.lowering/lower-ast` now caches by AST hash plus
      `:expr-core-v1` lowering policy, including recursive sub-AST results.
- [x] Cache compiled artifacts only with deterministic keys.
      Compiled artifact caching remains disabled for now, but clj-meta compile
      receipts now publish a deterministic cache key so future caching cannot be
      added without a stable form/wrapper hash and compiler-symbol key.
- [x] Preserve clj-meta classloader/determinism policy.
      The compiled artifact cache receipt is explicitly disabled with
      `:preserve-clj-meta-classloader-policy`; clj-meta primary/repeat
      determinism checks remain the acceptance gate.
- [x] Add benchmark only after semantic receipts are stable.
      `clojure -M:benchmark` runs only after a clean semantic preflight and
      records parse/lower cold-warm cache timing plus a full report baseline.

---

## Non-Goals

- [ ] Do not rebuild `pnix-hy`.
- [ ] Do not port the Python/Hy host interpreter.
- [ ] Do not make `pnix-clj` understand Hy/Python semantics.
- [ ] Do not build an AI agent, coding agent, task router, autonomous planner, or
      self-coding workflow in this repository.
- [ ] Do not optimize features for agent behavior. Optimize only for
      Clojure/Java/JVM ecosystem <-> pnix language projection, mirror evidence,
      and human-readable meta-circular research.
- [ ] Do not shrink the target to Clojure syntax alone. Java interop, JVM
      objects/classes, classloader behavior, reflection, bytecode evidence, and
      host runtime effects are valid research surfaces when they remain inside
      the clojure mirror -> pnix runtime (.px) -> pnix mirror receipt chain.
- [ ] Do not make `pnix-hy`/`pnix-clj` semantic parity a current-phase gate.
      Runtime sync, semantic parity, common brain, and semantic/common ABI work
      are future goals after host-local pnix-language runtimes have strong
      projections and mirrors.
- [ ] Do not design a common pnix brain or semantic ABI before the
      Clojure/Java/JVM <-> pnix projection is strong enough to preserve host
      semantics.
- [ ] Do not prematurely commonize host macro/eval/import/namespace/object/
      exception/type semantics. During this phase, only the outer
      receipt/mirror/evidence envelope may be common.
- [ ] Do not create `CLJ_AST_EVALUATOR_SOURCE` or `CLJ_AST_COMPILER_SOURCE` string
      blobs as the main architecture.
- [ ] Do not claim `clj-meta` proves the whole pnix language.
- [ ] Do not replace `clojure mirror -> pnix runtime (.px) -> pnix mirror` with a
      direct Clojure-only evaluator/compiler lane.
- [ ] Do not treat Clojure host eval as pnix semantics.
- [ ] Do not treat clj-meta direct emit success as pnix acceptance without evaluator
      and receipt comparison.
- [ ] Do not make `~/pnix-old` a runtime, test, report, or gate dependency.
- [ ] Do not make the new `~/pnix` ABI repository a runtime, test, report, or
      gate dependency.
- [ ] Do not read `~/pnix-old` from normal runtime/report paths; all needed `.px`
      artifacts and oracle rows must live inside this repo first.
- [ ] Do not read the new `~/pnix` from normal runtime/report paths; outer
      envelope standards must be vendored or versioned explicitly before use.
- [ ] Do not revive MSV/gate_graph/CEGIS as the main product inside `pnix-clj`.
- [ ] Do not use old `pnix` or old `pnix-clj` agent/cognition goals as
      acceptance criteria here. The acceptance target is projection fidelity plus
      mirror receipts.

---

## Resume Protocol

When resuming work:

1. Read this file first.
2. Check `git status --short --branch`.
3. Inspect `../clj-meta/todo.md` for backend changes before touching compiler APIs.
4. Run the smallest relevant smoke before edits if code already exists.
5. Make one small slice.
6. Run the matching pnix-clj report/test and relevant clj-meta gates.
7. Commit only the files touched for that slice.

If the worktree has unrelated `../clj-meta` changes, do not revert them. Treat
them as user work unless they directly block the current slice.

---

## Name Translation From the Copied pnix-hy Plan

```text
pnix-hy + hy-meta road
  -> pnix-clj + clj-meta road, same structure with Clojure roles

pnix-hy Python interpreter
  -> pnix-clj Clojure semantic evaluator

pnix-hy run_px / compile_px_source
  -> pnix-clj run-source / lower-source / compile-source wrappers

Hy evaluator/compiler source lane
  -> clj-meta compile/eval API plus pnix-clj receipts

Hy/Python mirror shape
  -> clojure mirror -> pnix runtime (.px) -> pnix mirror

Python Thunk / NativeFunc / PnixString
  -> Clojure pnix value model, to be defined here

pnix_mirror.self_test_report
  -> pnix-clj report command, to be defined here

rust/original corpus report
  -> repo-owned copied oracle fixture comparison; original paths are provenance only

stage15 in pnix-hy
  -> ../clj-meta stage15/N compiler/evaluator evidence
```

Any copied checklist item that cannot be translated through this table should be
deleted or rewritten before implementation.

## Host-language import of pnix product library (user intent, 2026-08-13)

**Canonical doctrine:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md)  
**HM mirror:** `~/dot-nix/dev/PNIX-HOSTS.md`

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


### clj — status (2026-08-14)

**Landed (product + docs):**

1. Dual-axis docs: monorepo `HOST_DEV_ENV.md`, host `CLAUDE.md` / `README.md`.
2. Host-main inject: `pnix-clj-clj` / bare `clojure` with `-Sdeps` local/root.
3. Host-language `.px` import helper: `pnix-clj.core/eval-file`.
4. Env: `PNIX_CLJ_ROOT` / `PNIX_CLJ_LIBRARY` (HM + wrappers).

**Still open (optional product polish):**

1. ~~Document public require surface~~ → `docs/HOST_IMPORT.md` (2026-08-14).
2. Optional: published Maven/local jar coordinate so projects need not local/root
   the monorepo path.
3. Any “compiled .px” artifact remains host-bound unless a common packaging
   contract is designed — do not claim otherwise.

## Post host-env plan (2026-08-14) — plan only unless owner pulls

Host dual-axis + `eval-file` / classpath inject are **closed** for day-to-day.
See monorepo `HOST_ENV_P2_P3.md`.

### Do not re-open as "residual menu"
Follow `docs/REMAINING_DECISION.md` and § REMAINING WORK above. No new
owner-gated residual list from host-env work.

### Optional product pulls (priority order when free)
1. **Machine fragment growth** (M-series) — only if a pillar needs it.
2. **Specialize residual fuel / fold options** — M1 follow-ups already listed.
3. **Maven publish of pnix-clj** — P3 registry; needs version + public API freeze
   (`docs/HOST_IMPORT.md` is the require surface).
4. **Conformance Phase D** — still DEFERRED (see backlog status at top).

### Host-import follow-ups (easy already done)
- [x] `eval-file` public helper
- [x] `examples/host-import/clj` mini project
- [x] monorepo `bin/host-import-smoke` + CI layout/clj example

### Host-import hard
- [x] Multi-file sample with `import` / modules map →
  `examples/host-import/clj-imports/` (2026-08-14)
- [ ] Published jar coordinate (P3)
