# 후보 GENERATOR 결정 — pnix-clj self-* 루프용

self-* 루프에서 빠져 있던 한 조각은 이미 증인 + 게이트 + 순위화하는
`self-improve`에 공급하는 후보 GENERATOR였습니다. 이것이 그 결정이며,
`/deep-research`(16 claims 3-0 확인; 워크플로의 자체 합성 단계는 API 주간
한도로 끊겨, 검증된 증거로 여기서 결정을 완성)에서 합성했습니다.

## 증거가 수렴 (전부 3-0 확인)

| 기법 | lazy pure functional Nix-like 언어 적합도 | 치명 함정 | proven-vs-heuristic |
|---|---|---|---|
| **Escher** — observational-equivalence 축소 (CAV'13) | ✅ exact fit: **예제 위 value-vector**로 행동 클래스당 대표 하나 유지; 제거 시 폭발 | 예제 입력 필요 | 유한 예제 observational-equiv = **heuristic PROPOSE** |
| **Myth/λ²** — evaluate-during-enumeration (POPL'15) | ✅ pure functional 재귀 ADT, higher-order | **trace-complete 예제** 필요 (실무에서 어려움); 탐색 여전히 폭발 | example-driven |
| **Smyth** — live bidirectional eval (2020) | ✅ trace-complete 집합 없이 재귀; sketch를 통해 예제를 **역방향** 전파 | 부분 명세가 여전히 underdetermine | verifier-drives-generator (CEGIS-like) |
| **Burst** — bottom-up + angelic + FTA (2021) | ✅ 재귀 함수형; 3 명세 모드 (examples / reference-impl / logical); **trace-completeness 불필요**; incremental 명세 = **FTA intersection** | angelic 가정을 discharge 해야 함 | CEGIS refinement 루프 |
| **Synquid** — refinement types (PLDI'16) | ✅ **증명 가능 정확** 합성; 명세 분해가 폭발 억제 | ★**논리 명세 작성이 수동 창작 단계** — self-improve에 자율 공급 불가 | **PROVEN** (decidable) |
| **Knuth-Bendix equivalence reduction** (VMCAI'19) | ✅ 등식 명세 → 합류 종료 TRS → 정규 정규형; 검증 전 **~80% 후보 가지치기** (21M→20% at 11 nodes) | 등식 이론 필요 | canonical dedup = sound pruning |

## 결정

**먼저 구축 — observational-equivalence-reduced bottom-up enumerative
synthesizer** (Escher 메커니즘, C11). pnix-clj의 정확한 상황에 단일 최적합:

1. **Dedup oracle이 이미 존재.** Escher value-vector 축소는 후보 행동을 예제
   입력에 대해 계산할 evaluator가 필요 — pnix-clj는 **이미**
   `core/eval-source`(및 전체 `run-witnessed` verifier)를 가짐. 이 기법의
   비싼 의존성 하나가 여기서 무료.
2. **Lazy pure functional 언어에 직접 적합** (Escher/Burst가 정확히 이 클래스용)
   — 호스트 interop 없음, mutation 없음, 값은 순수 EDN.
3. **헌법의 proven-vs-heuristic 경계에 깨끗이 착륙.** 유한 예제 value-vector
   매치는 *observational equivalence*이며 **heuristic PROPOSE**, 증명 아님 —
   헌법 규칙 그대로. 따라서 generator는 PROPOSED 후보를 방출; `run-witnessed`가
   잘 동작하는 pnix 프로그램임을 증명하고, `arith-proof`/`bool-proof`가
   적용 가능한 곳에서 동등성을 PROVEN으로 승격. 모두 HELD (self-mod-gate).
4. **치명 함정을 피함.** Myth의 trace-completeness를 요구하지 않음(전체 식을
   enumerate, 재귀 트레이스 아님), Synquid의 수동 논리 명세도 요구하지 않음
   (refinement-type 합성이 self-improve에 자율 공급 불가가 되는 함정 C10).

**플러그인 지점:** 새 `pnix-clj.generate`가 후보 pnix 식 문자열을 방출;
`synthesize-and-propose`가 `self-improve/evaluate-round`에 넘겨
증인(`run-witnessed`) + 게이트(`self-mod-gate`) + 순위 + HELD 검토 큐로 지속.
Value-vector dedup은 `core/eval-source` 사용.

### 그다음 순서

2. **CEGIS refinement** (Smyth/Burst — C6, C12-C14): `run-witnessed` +
   `property-fuzzer` COUNTEREXAMPLE를 되먹여 예제 집합을 강화하고 재-enumerate
   (angelic → analyze → strengthen → retry). 강한 verifier를 generator 드라이버로.
3. **Canonical equivalence-reduction pruning** (Knuth-Bendix — C15-C16):
   평가 전 후보를 *구문적으로* 가지치기, pnix-clj 기존 정규 형태
   (§3 α-canonical + arith-proof polynomial + bool-proof truth-table)를
   정규형 oracle로 (~80% 가지치기).

### 의도적으로 먼저 하지 않음

- **Synquid refinement-type 합성** — 증명 가능하지만 수동 명세 필요 (C10):
  루프에 자율 공급 불가. 명세 소스에서 *proven-by-construction* 후보를 원할 때
  재검토.
- **Library-learning / LLM (DreamCoder/babble/LILO)** — 휴리스틱, corpus 또는
  모델 필요; 이후 배율기, 첫 정직한 벽돌 아님.

## 우선 TODO

1. `pnix-clj.generate` — 작은 pnix 문법 위 bottom-up enumerator (입력 변수 +
   int/bool literal + `+ - *`, 비교, `if`, 안전 builtin),
   `core/eval-source`로 value-vector 평가, **observational-equivalence
   dedup** (value-vector당 대표 하나). 매치(예제 출력과 value-vector가 같은
   식)를 pnix 소스로 반환 — HEURISTIC 라벨.
2. `synthesize-and-propose` — 각 매치를 증인 가능 pnix 프로그램으로 래핑해
   `self-improve/evaluate-round` → 순위 HELD 제안; 추가로
   `arith-proof`/`bool-proof` PROVEN인 매치 보고.
3. CEGIS: 매치에서 `property-fuzzer`로 counterexample 탐색; 찾으면 예제 집합에
   추가하고 재합성 (Burst/Smyth 루프).
4. Knuth-Bendix 스타일 정규 사전 가지치기, §3/arith/bool 정규 형태 사용.
5. (이후) refinement-type 레인; (이후) corpus/library-learning 배율기.
