# 남은 작업 결정 — F7b / F8 / gate-hog / 놓친 공백

`/deep-research` 판정 (2026-07-07, 105 agents, 3-vote adversarial verification;
10 findings, 하나 빼고 전부 3-0). 기계 체크리스트는
`resources/pnix_clj/roadmap.edn` (렌더 → `docs/WIKI.md`); 이 문서는 추론과
참조를 담음.

> ★ 2026-07-07 구축 순서 (C → D → F8 → F7b-held)는 DONE: C, D-probes
> (D1–D17), F8 모두 LANDED. 아래는 그 역사적 추론.
> 남은 모든 것에 대한 CURRENT 판정은 바로 아래 2026-07-08 절 — 먼저 읽기.

## 갱신 — 남은 백로그 판정 (2026-07-08, /deep-research, 104 agents, 20/25 claims 3-0 확인)

F8 이후 남은 백로그는 네 항목: **D1c**, **conformance Phase D**,
**`"`-in-`${}` splice leniency**, **F7b**. 신선한 외부 증거 (실제 Nix C++,
Tvix Rust, PL/PE 문헌)가 네 가지를 모두 해결 — 소유자 메뉴 불필요.
아래 "구축 순서" 표를 이 항목들에 대해 supersede.

| 항목 | 2026-07-08 판정 | 이유 (증거) |
|---|---|---|
| **splice `"`-in-`${}` leniency** | **REJECTED — 제거하지 말 것; corpus 마이그레이션 없음** | 전제가 FALSE. 실제 Nix는 `${…}` 안 double-quoted 문자열을 거부하지 않음: `${`가 전체 식 컨텍스트를 열고 nixpkgs가 nested strings에 광범위 의존 (`"${"foo"}"`→`foo`, `"a${"b${"c"}d"}e"`→`abcde`, nix-instantiate 2.34.7 전부 수락). D7 balanced-scanner는 CORRECT Nix이지 leniency 아님; 조이면 conformance 감소. (3-0; nix.dev string-literals manual.) 남은 유일한 micro-quirk는 splice 안 `\"`-*escaped* quotes — 연구가 격리하지 않았고, 어느 쪽이든 마이그레이션 가치 없음. |
| **D1c** (explicit-stack non-tail eval) | **DEFER; 한다면 conformance가 아니라 PILLAR 작업** | 실제 Nix는 deep non-tail stack-safety 보장 없음: native C++ stack + `max-call-depth` (기본 10000, 2.20 추가)는 **함수 호출만 세고 데이터 중첩은 안 셈** — 우리 exact nested-list/left-spine 형태에서 Nix 자체도 머신 의존 segfault (open upstream issue #9627). 우리 graceful structured `:stack-overflow` bound **가 이미 Nix-parity 답** → conformance 가치 ≈ 0. 건드릴 유일한 이유는 **functional correspondence** (closure-conversion + CPS + defunctionalization → CEK/Krivine machine; Ager-Biernacki-Danvy-Midtgaard PPDP'03), 자체로 메타순환/투영 아티팩트 = M-series pillar 작업. Tvix가 방향 확인 (bytecode VM + deep data용 상수 스택 generators), 그러나 Tvix TCO도 tail position만. ★함정: **trampoline**과 **"store-allocated continuations"** 정당화는 0-3 반박 (clojure.core/trampoline; arXiv 1007.4446) — 그 위에 D1c 짓지 말 것; call-by-need는 Krivine + memoizing-store (CESK) 정제 필요. |
| **conformance Phase D** (impurity/store) | **DEFER — conformance 재료; pillar가 필요로 할 때만, 그때도 pure 부분 집합만** | Tvix가 미러할 exact 아키텍처 보여 줌: 교체 가능 `EvalIO` trait (기본 `DummyIO`), `builder_pure`/`builder_impure`, `pure_builtins` 모듈 vs cargo feature 뒤 **7** impure builtins만 든 `impure` 모듈 (getEnv, hashFile, pathExists, readDir, readFile, readFileType, currentTime). 최고 가치 **pure/hermetic** 부분 집합 = hashString, fromTOML, toXML, toFile→content-addressed path, 결정적 path realization — daemon/network 없음. Fetchers / currentTime / currentSystem / flake-refs / findFile은 진정 impure → 같은 seam 뒤, 시뮬레이션만. Full Tvix도 store를 evaluator 밖에 유지 (tvix-store/castore crates). (3-0; docs.tvix.dev, impure.rs.) |
| **F7b** (self-applicable PE for call-by-need) | **RE-CONFIRMED OPEN — HELD 유지** | call-by-need-native self-applicable PE 존재하지 않음. 전부 (mix, Similix, …) STRICT 언어 대상; Similix가 non-strict *인터프리터*를 specialize하지만 specializer는 strict이고 laziness는 object-level suspension = **call-by-name (공유 없음)**, call-by-need 아님. 07-07 결론과 일치; open 상태를 반박하는 것 없음. (3-0; Springer PE chapter.) |

**순:** "소유자-결정 메뉴" 해소 — splice 거부, F7b held 유지, D1c와 Phase D
모두 연기 (D1c는 pillar 유도로 재프레이밍 가능, Phase D는
pure-subset-when-a-pillar-needs-it). 여기 어떤 항목도 긴급 소유자-게이트 호출
아님; 전진은 pillar 주도 (M-series) 또는 oracle-확인 발산만, 헌법에 따라.

주의: Tvix "~4000 LoC / TCO incomplete" 수치는 Sept-2022 스냅샷
(2024 소스에서도 아키텍처 유지). functional-correspondence 논문은 λ-calculus
코어 대상; lazy thunk-sharing evaluator에 적용하려면 call-by-need 정제 필요 —
D1c 권고는 표준 PL 이론에 기초하며, stack-safe lazy Nix machine을 문자 그대로
유도한 논문이 아님.

## 결정 — 구축 순서

| # | 항목 | 판정 | 정직한 라벨 |
|---|---|---|---|
| 1 | **C · gate report cache** | **DO NOW** — 검증된 기법, 연구 위험 0, ~123s/500s 이득 | §8 pin + §9 determinism 증인 아래 trace 이론상 sound |
| 2 | **D · gap probe** | 범위 한정, **oracle-확인 버그만** (★D-angle 주장 검증 통과 0 — 체크리스트 갈기 금지) | 각 수정은 먼저 nix-instantiate oracle 판정 필요 |
| 3 | **B · F8 weval spike** | C 이후 경계 있는 spike — ~2x 천장, perf 프로그램이 아닌 아키텍처 증명 | 정확성 = construction argument + differential tests; 성능 = heuristic |
| 4 | **A · F7b** | **HELD 유지** — 진정 open research, 소유자 승인 필요 | 성공해도 부분 정확성 TEST만 |

## A · F7b — pnix self-applicable specializer: HELD (open research)

- **call-by-need 선례 없음.** 모든 고전 self-applicable PE —
  flow-chart mix, Scheme0, lambda-mix, Logimix, Similix — STRICT 언어 대상
  ("Similix: a self-applicable partial evaluator for a higher order
  subset of the strict functional language Scheme"). Jones/Gomard/Sestoft 책
  전수 검색에서 lazy 언어 self-applicable PE 없음; 최근접 미스 =
  Mogensen normal-order λ-calculus PE (call-by-need 공유 없음). Nix-like
  언어용 F7b는 NEW research, engineering 아님.
- **검증된 lazy-언어 경로는 이미 가진 것.** Bondorf 1990:
  `delay` memoization은 1990 전 self-applicable PE가 다룰 수 없던 부수 효과;
  통한 길 (Jørgensen POPL'92, commercial-compiler-speed lazy-language
  compilers)은 lazy-language 인터프리터 위 STRICT-host specializer —
  정확히 pnix-clj의 Clojure specializer.
- **성공해도 한정된 이득** (Glück PEPM'09 §5.2): spec(spec,spec)
  mismatch는 오류 증명; 동의는 아무것도 증명하지 않음 — 기계 검사 가능한
  부분 정확성 테스트, 결코 증명 아님.
- **소유자가 언젠가 green-light하면**: Jones/Gomard/Sestoft §7.4 11-step
  레시피 — pnix를 bare-bones core로 자르기 (with/assert/contexts 없음),
  clean self-interpreter, hand annotations, specializer를 Clojure로 FIRST,
  pnix로 LAST 재작성; go/no-go 게이트 = self-interpreter specialization의
  Jones-optimality.

## B · F8 — IR-level PE spike: 경계 있음, C 이후

- **선례 견고**: weval (PLDI 2025) — 대부분 미수정 인터프리터 본문 위
  IR-level 1st Futamura, SSA basic-block CFG (~5 KLoC transform;
  SpiderMonkey +1045/−2 lines, 인터프리터 fn 133; production StarlingMonkey).
  Truffle/GraalVM (PLDI 2017)이 JVM 선례.
- **치명 함정 정확히 알려짐**: constant propagation이 인터프리터 루프
  backedge에서 붕괴 (pc merges non-constant → specialization이 인터프리터
  복사본 반환). 수정 = pc-as-specialization-context intrinsics
  (context별 split analysis, exponential unrolling 없이 merge reconnect).
  weval intrinsics와 Truffle `@TruffleBoundary` 모두 HAND-PLACED — Truffle이
  자동 heuristic 시도 후 "removed all heuristics again" (9년 후에도 참).
  수동 annotation 예산.
- **이득 천장 ~2x, 정직하게**: weval 측정 2.17x avg (SpiderMonkey/Octane),
  1.84x (Lua); 실제 JIT는 그 너머 3.86x. F8 = clj-meta가 IR-level PE를
  호스팅할 수 있다는 증명, 성능 프로그램 아님.
- **JVM 주의**: JVM 바이트코드는 stack-based; weval transform은 SSA-CFG IR
  가정. clj-meta spike는 SSA-ish view (또는 tools.analyzer AST)에서, deopt
  기계 없이, STATIC residual만.

## C · gate report cache: DO NOW

- **설계 라이선스 Build-Systems-à-la-Carte** (ICFP 2018 §4.2.2-3):
  VERIFYING trace = report-kind별 입력 해시 + 결과 해시 기록, 변경 없으면
  재렌더 스킵; CONSTRUCTIVE trace = 아티팩트도 저장해 copy-instead-of-render
  허가. Key = (report-renderer code version ⊕ capability corpus CAS hash ⊕
  §8 runtime-snapshot pin). Soundness 조건 (determinism + complete input
  tracking)이 §9 증인과 §8 pin이 이미 제공. 존중 주의: volatile tasks
  uncacheable (§6.3); Frankenbuild hazard는 determinism 전제 필요 (§4.2.4)
  — 우리는 그것을 증인.
- 123s hog: `report-artifact-is-persisted-as-edn`가 같은 게이트 JVM에서 자체
  deftest가 이미 렌더한 7 corpus reports (mirror-pair, determinism, coverage,
  forward-reference, clojure-form, clojure-projection, smoke)를 재렌더.
- Drift 게이트 미건드림: capabilities/wiki/lane-registry 검사는 캐시 경유 안 함.

## D · 놓친 공백: oracle-gated probe only

★정직 플래그: D-angle 연구 주장 (hnix/Tvix/Lix checklists, JVM-lazy-hosting
patterns) 중 adversarial verification 통과 0 — 아래 항목은 engineering
plausibility, verified-source-backed 아님. 헌법 적용: nix-instantiate oracle
probe가 실제 발산(버그)을 확인할 때만 작업, 체크리스트 갈기 금지.

Plausibility별 probe 대상:
1. **Deep-recursion stack safety** — JVM tree-walk evaluator vs deeply nested
   pnix (예: 100k-deep `let`/list nesting; Nix has recursion limits/deepSeq);
   Nix가 gracefully 오류 또는 성공하는 곳에서 가능한 StackOverflowError.
2. **builtins strictness matrix** — 어떤 builtin 인자가 force vs lazy vs Nix
   (Tvix가 per-builtin strictness 문서화).
3. **Catchable-vs-uncatchable error taxonomy** — `tryEval`이 `throw`/
   `assert`를 catch하지만 Nix에서 `abort`/type errors는 안 함; 우리 matrix
   probe.
4. **Float formatting/semantics parity** — floats의 toString/toJSON.

## 참조

Jones/Gomard/Sestoft, *Partial Evaluation and Automatic Program Generation*
(§6.4 optimality, §7.4 recipe) · Bondorf 1990 · Jørgensen POPL'92 · Glück
PEPM'09 (Thm 1 p.54, §5.2, §7.2) · Fallin, *weval*, PLDI 2025 · Würthinger et
al., Truffle PE, PLDI 2017 · Mokhov/Mitchell/Peyton Jones, ICFP 2018.
