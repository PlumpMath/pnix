# pnix-clj 레인 분류

이 문서는 범위 잠금 이후 pnix-clj 네임스페이스와 기능 표면을 분류합니다.

`SCOPE_LOCK.md`가 권위 문서입니다. 이 파일은 기존 레인을 어떻게 다뤄야 하는지
설명합니다.

## 분류 라벨

### CORE

pnix-clj 코어 게이트에 허용됩니다.

Clojure 호스팅 pnix 메타순환 증명 레인의 일부입니다.

### PROOF-ONLY

레인이 경계 있는 증명/동등성/증인 증거를 생성할 때만 허용됩니다.

제품 동작, 자율 행동, NL 라우팅, coding-agent 실행이 되어서는 안 됩니다.

### EXPERIMENTAL

경계 있는 연구/증명 실험으로만 허용됩니다.

게이트되고, 문서화되며, 비권위적이어야 합니다.

### QUARANTINE

pnix-clj 코어 밖입니다.

사이드 저장소로 분리하거나 명시적으로 재분류하지 않는 한 `src`, 테스트 게이트,
코어 런타임에 들어오면 안 됩니다.

---

## CORE 레인

| 레인 | 분류 | 이유 |
|---|---|---|
| parser | CORE | pnix 소스 → AST |
| lowering | CORE | AST → 정규/lower form |
| core evaluator | CORE | pnix eval-source / eval-from-ast |
| px-runtime | CORE | pnix 평가용 런타임 레인 |
| CAS | CORE | content-addressed 정체성 |
| store | CORE | append-only 증거 / term 저장 |
| snapshot | CORE | 결정적 핀된 상태 |
| persist | CORE | 내구성 있는 재연 지원 |
| mirror | CORE | 런타임 mirror 증거 |
| mirror-chain | CORE | 반복 mirror 수렴 |
| mirror-pair | CORE | mirror 경로 간 동등성 비교 |
| mirror-error | CORE | 구조화된 mirror 실패 증거 |
| determinism | CORE | 반복 실행 안정성 |
| purity | CORE | effect와 결정성 규율 |
| replay | CORE | 증인 재검증 |
| witness | CORE | 증명/증거 객체 표면 |
| witnessed-run | CORE | 실행 + 증인 바인딩 |
| receipt | CORE | content-bound receipt |
| safe-eval | CORE | 경계 있는 eval 표면 |
| capabilities | CORE | effect/capability 규율 |
| trust | CORE | trust 경계 증거 |
| classfile-receipt | CORE | JVM/class 아티팩트 증인 |
| version | CORE | 런타임/컴파일러 버전 바인딩 |
| clj-meta host reflection | CORE | 호스트 언어 증명 레인 |

---

## PROOF-ONLY 레인

| 레인 | 분류 | 규칙 |
|---|---|---|
| Futamura / specialize | PROOF-ONLY | 투영/동등성 증거로만 허용 |
| translation-validation | PROOF-ONLY | 동등성 검증으로만 허용 |
| stage7-core | PROOF-ONLY | staged 클로저 증명으로만 허용 |
| stage15 | PROOF-ONLY | 경계 있는 tower/self-hosting 증명으로만 허용 |
| oracle / live-oracle | PROOF-ONLY | 경계 있는 비교 oracle로만 허용 |
| coverage | PROOF-ONLY | 증명 표면 커버리지로만 허용 |
| grammar-fuzzer | PROOF-ONLY | parser/런타임 견고성 증거로만 허용 |
| property-fuzzer | PROOF-ONLY | 경계 있는 property 증거로만 허용 |
| arith-proof | PROOF-ONLY | 산술 증명 fixture로만 허용 |
| bool-proof | PROOF-ONLY | 불리언 증명 fixture로만 허용 |
| value-roundtrip | PROOF-ONLY | 값 브리지 증거로만 허용 |
| emit-form-roundtrip | PROOF-ONLY | Clojure form roundtrip 증거로만 허용, 다언어 codegen 아님 |

---

## EXPERIMENTAL 레인

| 레인 | 분류 | 필수 절제 |
|---|---|---|
| synthesize | EXPERIMENTAL | 경계 있는 후보 생성만; 자율 허가 없음 |
| generate | EXPERIMENTAL | 경계 있는 생성만; NL/coding-agent 확장 없음 |
| self-improve | EXPERIMENTAL | held/candidate/gated로 유지; 자율 mutation 없음 |
| self-mod-gate | EXPERIMENTAL | 게이트만; 직접 mutation 허가 없음 |
| rust-batch | EXPERIMENTAL | 증명/동등성 배치로 남을 때만, Rust 제품 레인 아님 |
| clojure-projection | EXPERIMENTAL | Clojure 호스트 투영 증거로만 |
| clojure-form | EXPERIMENTAL | 호스트 form 분석/roundtrip 증거로만 |
| form-analysis | EXPERIMENTAL | Clojure form 증명 분석으로만 |
| benchmark | EXPERIMENTAL | 측정만; 의미 권위 아님 |
| wiki | EXPERIMENTAL | 문서/인덱스만; 런타임 진리 아님 |

---

## QUARANTINE 레인

다음은 명시적으로 pnix-clj 코어 밖입니다.

| 레인 | 분류 | 이유 |
|---|---|---|
| Hangul codec | QUARANTINE | NL/의미 레인, pnix 메타순환 증명 아님 |
| MSV / meaning sentence variants | QUARANTINE | NL 의미 생성 레인 |
| Korean dictionary | QUARANTINE | 언어 지식 레인 |
| Korean mirror | QUARANTINE | NL mirror 레인 |
| domain-token / domain-generic matching | QUARANTINE | 의미 라우팅/매칭 레인 |
| graph-gate / gate-graph | QUARANTINE | agent graph/emit 레인 |
| multi-language emit registry | QUARANTINE | coding-agent/codegen 레인 |
| behavior-atom emit | QUARANTINE | coding-agent 동작 표면 |
| puck-cli bridge | QUARANTINE | 외부 executor 브리지 |
| tick-runner | QUARANTINE | 자율 루프/스케줄러 |
| redb ingest brain | QUARANTINE | 외부 지식/메모리 수집 |
| NL corpus / meaning graph | QUARANTINE | 자연어 의미 메모리 |
| answer composer | QUARANTINE | NL 응답 생성 레인 |

---

## 이후 작업 규칙

네임스페이스, 테스트, 별칭, 앱 러너를 추가하기 전에 여기서 분류하세요.

범위 잠금 아래 CORE, PROOF-ONLY, EXPERIMENTAL으로 분류할 수 없으면
pnix-clj 코어에 들어오면 안 됩니다.

불확실하면 QUARANTINE으로 분류하세요.

---

## 현재 정체성 잠금 부록

생성된 진실의 원천은 `docs/LANE_REGISTRY.md`입니다.

현재 최상위 레지스트리 개수:

- CORE: 38
- EXPERIMENTAL: 6
- PROOF-ONLY: 26
- TOTAL: 70

다음 표면은 CORE 정체성 표면입니다.

- interop: Clojure 런타임 ↔ pnix 런타임 메타순환 교차 경계
- nREPL: 메타순환 대화형 제어 표면; eval은 코어만 경유
- wiki: 자기 문서화 능력 및 로드맵 기판
- lane-registry: 생성된 레인 분류 레지스트리

`nrepl`, `wiki`, `interop`는 버릴 수 있는 개발 전용 표면이 아닙니다.

다음은 QUARANTINE으로 남으며 pnix-clj 코어에 들어오면 안 됩니다.

- Hangul codec
- MSV / meaning sentence variants
- graph-gate / gate-graph
- multi-language emit registry
- puck-cli bridge
- tick runner
- redb ingest brain
