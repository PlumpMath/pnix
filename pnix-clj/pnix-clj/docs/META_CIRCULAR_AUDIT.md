# 메타순환 능력 감사 — pnix-clj / clj-meta

소유자의 58-능력 "Pure Meta-Circular Capability Checklist" (2026-07-04)에 대한
정직하고 증거 기반 스코어카드. 기억이 아니라 실제 코드(`file:symbol`)에 대해
검증. 과대 주장 없음 — 헌법 규칙 (history ≠ truth; 주장 전 검증).

## ✅ 갱신 2026-07-04 — 증거-저장소 SPINE이 feat에 재구축됨

연구 검증 계획(docs/SPINE_ROADMAP.md, option C = clean-rewrite 스타일로 새로
재구축)에 따라, 8개 spine gap 능력이 의존 순서로 각각 게이트 핀되어 feat에
LANDED:

| § | 능력 | 모듈 | 상태 |
|---|---|---|---|
| §3 | content-addressed TERM store; **α-canonical** (de Bruijn + 올바른 shadowing); hash = propose filter, 정확히 확인 | `cas.clj` | ✅ (§3b 포함) |
| §5 | append-only 변조 탐지 EVENT log (verifying trace + hermeticity guard) | `store.clj` | ✅ |
| §10/§13.1 | Clojure/JVM reflection 스냅샷 (결정적, 순수 EDN) | `reflect.clj` | ✅ |
| §8 | snapshot 런타임 pin + fail-closed match 게이트 | `snapshot.clj` | ✅ |
| §9 | purity/determinism as EVENTS (재실행으로 증인, first-divergent anchor) | `purity.clj` | ✅ |
| §17 | content-address + event + structural-similarity 검색 (+ §3c open-term summary) | `search.clj` | ✅ |
| §6.6-6.7 | mirror chain 수렴 + drift events | `mirror_chain.clj` | ✅ |
| §15 | 증인 스키마 + admission lattice (CAPSTONE) | `witness.clj` | ✅ |
| **integration** | **witnessed-run** — 한 실행이 spine을 기둥에 묶음 (term-keyed, snapshot-pinned, tower+chain+determinism as one §5 log, residual content-addressed, §15-admitted) | `witnessed_run.clj` | ✅ |
| **§14.3** | **self-modification 게이트** — 헌법 NO-AUTO-PROMOTION을 런타임 게이트로 (admitted 증인은 소유자 승인까지 HELD) | `self_mod_gate.clj` | ✅ |
| **durability** | **persist** — §3 terms + §5 events용 content-addressed 디스크 백킹, load 시 재검증 (Unison/Nix-store 형태) | `persist.clj` | ✅ |

Spine은 이제 실행 경로에 LIVE (witnessed-run), self-* 게이트 (자동 승격 없음),
내구성 있음 (persist). 세 runner 레인(pnix / pnix-clj-clj / clj-meta) 각각
일급 nREPL 서버, clj-meta nREPL은 자체 바이트코드 백엔드로 eval 라우팅.

열린 follow-up (roadmap.edn 등록): §11 pnix-macro/reader 레인 (낮은 적합 —
Nix에 Lisp 스타일 macro 없음; clj-meta에 이미 macroexpand).
아래 절별 스코어카드는 PRE-spine 상태를 반영; spine 행
(§3/5/8/9/10/13.1/17/6.6-6.7/15) + §14.3는 이제 ✅로 취급.

## ⚠ 두 설계 라인 (먼저 읽기)

pnix-clj 저장소에는 **갈라진 두 브랜치**가 있고 체크리스트 능력은 그 위에
나뉨:

- **`origin/main`** — 체크리스트의 **증거-인프라 spine** 운반:
  `cas.clj`, `store.clj`, `term.clj`, `stage.clj`, `purity.clj`, `stm.clj`,
  `resolve.clj`, `evidence.clj`, `mirror_journal.clj`, `verifier.clj`,
  `dirty.clj`, `search.clj`. → 체크리스트 §3, §5, §7(store-tower), §8, §9, §15,
  §17.
- **`feat/clj-meta-metacircular`** (작업 브랜치) — 메타순환 **능력 기둥**을
  강조하는 **clean rewrite**: 4-substrate self-hosting COLLAPSE tower,
  Futamura 1st+2nd 투영, 측정 Jones-optimality, safe-eval sandbox,
  content-addressed eval cache, capabilities drift-gate, synthesize 투영,
  form-analysis, property fuzzer, arith/bool PROVEN 동등성, interop
  (값 브리지 + 증인 + effect + capability 게이트).

정직한 한 줄 판정: **능력 PILLARS는 feat에서 강하고 (투영/증명 방향에서는
실제로 체크리스트를 넘어섬); 체크리스트 증거-STORE spine (§3/5/8/9/17)은
feat에 없고 `origin/main`에 산다.** 둘을 합치는 것은 소유자 결정 (하단).

## 스코어카드 (명시 없으면 feat 브랜치)

범례: ✅ 존재 · 🟡 부분/다른 형태 · ⬜ feat에 부재 (→ main =
origin/main에 존재) · ➕ 체크리스트 너머

| § | 능력 | 상태 | 증거 |
|---|---|---|---|
| 1 | clj-meta stage3 호스트 하한 / host-proof 분리 | 🟡 | `bin/pnix-clj-gate`; clj-meta = `../clj-meta` (호스트 증명), pnix-clj = 런타임. launcher stage3-jar 거부 없음. |
| 1.3 | content-bound 런타임/컴파일러/evaluator 버전 | 🟡 | `version.clj`, clj-meta compile-receipt determinism; 전체 content-bound-version lattice 아님. |
| 1.4 | class artifact hash | ✅ | `classfile_receipt.clj`, clj-meta `bytecode_witness`/`jarproof`. |
| 2 | tokenizer / parser / parse-error reification | ✅ | `parser.clj` (tokenize, parse-*), `error.clj` (pnix-error, spans). |
| 3 | pure pnix AST / canonical form / content-addr term hash | ⬜→main | AST는 순수 데이터; `hash.clj` data-hash. 그러나 feat에 `normalize-term`/`canonical-form`/term-store 없음 → main의 `cas.clj`/`term.clj`. |
| 3.1 | mutable runtime object guard | ⬜→main | feat에 CAS guard 없음 (main `cas.clj`). |
| 3.4 | open-term structural summary / alpha | ⬜ | 미구축. |
| 4 | eval-source / eval-from-ast / apply layer | ✅ | `evaluator.clj` (eval-ast, apply-callable), `core.clj` (run-source, eval-source). |
| 4.5 | runtime mirror mode | ✅ | `mirror.clj` run-mirror, `px_runtime.clj` runMirror receipt. |
| 5 | append-only event store / event hash / index / pointer | ⬜→main | feat에 event log 없음 → main `store.clj`/`evidence.clj`. 증거는 receipt 형태 (`receipt.clj`), append-only log 아님. |
| 6 | single mirror law / host+inner mirror / trace | ✅ | `mirror.clj`, cross-mirror-verdict; run-source가 단일 진입점. |
| 6.6-6.7 | mirror convergence / drift / chain stability | 🟡 | 실행당 cross-mirror agree/reject; drift-event 또는 반복-실행 chain-convergence log 없음. |
| 7 | stage tower (stage1..7 store-backed) | 🟡 | `tower.clj`는 **4-substrate COLLAPSE tower** (read→emit→direct→specialize→lowering→clj-meta→px→mirror), stage1-7 store/snapshot tower 아님 → main `stage.clj`. `stage7_core.clj`, `stage15*.clj`는 다르게 존재. |
| 8 | snapshot version pin / runtime-match gate / resolve | ⬜→main | feat에 없음 → main `resolve.clj`. |
| 9 | purity / determinism / mutation isolation / threaded stress | 🟡→main | `determinism.clj` (repeat-eval hash 안정성) 존재; 전체 purity-event/mutation-isolation → main `purity.clj`. |
| 10 | namespace / Var / metadata / dynamic-binding reflection | ⬜ | 어느 레인도 미구축 (clj-meta `host_reflection.clj` 부분). |
| 11 | macroexpand trace / pnix macro / reader form control / hygiene | 🟡 | clj-meta `compiler.clj`가 tools.analyzer macroexpand 사용; `clojure_projection.clj`. pnix-macro 계층 없음, reader-form control 레인 없음. |
| 12 | requiring-resolve witness / namespace load gate | 🟡 | `interop.clj` host-eval-form + capability 게이트; 명시 require-witness event 없음. |
| 13.1 | classpath / JVM version snapshot | ⬜ | feat에 없음 (clj-meta `jarproof`에 jar hashing). |
| 13.3 | pnix value ↔ Clojure value bridge / roundtrip | ✅ | `interop.clj` `to-host`/`from-host`, `value_roundtrip.clj`, `value-loss` markers. |
| 13.4-13.5 | Java opaque ref / IFn boundary / gate | ✅ | `interop.clj` make-opaque-host-ref, opaque-ref-deref, host-object?, gated crossings. |
| 14.1 | effect classification | ✅ | `interop.clj` effect-class?, `safe_eval.clj` impure-builtins 정적 분류. |
| 14.2 | capability gate | ✅ | `interop.clj` check-capability, `safe_eval.clj` pure-only 게이트. |
| 14.3 | self-modification gate | ⬜ | 미구축 (체크리스트 §6도 연기). |
| 15 | explicit witness schema / event-witness / admission lattice | 🟡 | `interop.clj` make-witness/crossing-witness, `receipt.clj` lane-summary/verdict, 곳곳 accepted/held/rejected. 전체 :witness/* event 스키마 아님. |
| 16 | roundtrip checks (source/store/stage/value/form) | ✅ | `emit_form_roundtrip.clj`, `value_roundtrip.clj`, `unparse.clj` roundtrip; store/stage roundtrips → main. |
| 17 | content-address / event / structural search | ⬜→main | main `search.clj`; feat에 없음. |
| 18 | source/AST cache / compile artifact cache / correctness | ✅ | `cached_eval.clj` (content-addressed EVAL cache), lowering cache, `specialize` cache. |
| 19 | debug / explain reports | 🟡 | 모든 능력에 `*/report` + `report-artifact`; feat에 `explain-*` 서술 fn 또는 `proof/*.edn` dir 없음. |
| 20 | file layout (src/pnix_clj/meta/, bin/, proof/) | ⬜ | `meta/` 하위 dir 없음, `bin/pnix-clj-gate`만, `proof/` dir 없음. |

### ➕ 체크리스트 너머 (feat 추가, 58에 없음)

| 능력 | 증거 |
|---|---|
| Futamura **2nd projection** (generating extension, cogen-free) | `futamura.clj` |
| **measured** Jones-optimality witness | `futamura.clj` jones-optimality-witness |
| **PROVEN** equivalence — arithmetic (polynomial) + boolean (truth table) | `arith_proof.clj`, `bool_proof.clj` |
| property-based **differential fuzzer** with shrinking (found+fixed 2 bugs) | `property_fuzzer.clj` |
| capability index + **drift gate** + machine wiki registry | `capabilities.clj`, `wiki.clj`, `roadmap.edn` |
| reverse projection Clojure→pnix + analyzer cross-check | `synthesize.clj` × `form_analysis.clj` |

## 정직한 하단 요약

58-항목 체크리스트 대비 feat 대략: **~24 present/✅, ~10 partial/🟡,
~9 absent-on-feat/⬜ (그중 5는 `origin/main`에 존재), + 6 beyond-checklist**.

메타순환 **능력 기둥** (투영, self-hosting collapse, 증명, 교차 검사, interop)은
잘, 정직하게 구축 중 (게이트 핀, receipt 운반). 체크리스트 **증거-저장소
spine** (정규 CAS term store §3, append-only event log §5, snapshot/resolve §8,
purity-as-events §9, search §17)이 feat의 진짜 공백 — 그리고 그것이
`origin/main`이 이미 구현한 것.

## → 소유자 결정 (경계 규율 — 자동 승격 아님)

조용히 고를 일이 아니라 진짜 소유자 호출:

**(A) `origin/main` 증거-spine (cas/store/term/stage/purity/resolve/
evidence/search)을 feat로 port**, 두 라인 통합 — feat가 전체 체크리스트 기판을
얻음.

**(B) feat를 기둥 중심 라인으로 유지**하고 store-spine을 main의 별도 관심사로.

**(C) feat에 최소 spine을 새로 재구축** (canonical term hash + append-only event
log + snapshot pin + purity events), feat rewrite 스타일로.

공백은 `resources/pnix_clj/roadmap.edn`에 등록 (§ 항목 as `:planned`)되어
결정과 무관하게 잃지 않음.
