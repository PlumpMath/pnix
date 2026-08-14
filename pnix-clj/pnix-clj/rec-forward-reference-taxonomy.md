# rec / let 전방 참조 Fixture 분류 (감사, 동작 변경 없음)

상태: **감사 + 증거만.** 이 문서를 만들기 위해 evaluator/parser/runtime/fixture
동작을 바꾸지 않았습니다. 시도했다가 되돌린 `rec` 전방 참조의 *감독된*
다중 레인 수정에 대비합니다(todo.md 및
`docs(pnix-clj): record rec-forward-ref as supervised multi-lane work` 커밋 참고).

소유자 방향에 따른 목표: fixture 공간을 분류하고, 현재
`mirror-error/rec-forward-reference` fixture를 재분류해야 하는 이유에 대한
**증거/receipt**를 남기며, 부정 fixture를 성공 fixture로 조용히 뒤집지 않기.

---

## 1. 네 레인

`pnix-clj.core/run-source`는 모든 소스를 네 레인으로 돌리고 `:lane-summary`를
기록합니다.

```
pnix-clj-evaluator            직접 의미 evaluator (이것이 런타임)
pnix-clj-lowering-clj-meta    pnix AST -> clj-meta form -> 호스트에서 eval
clojure-stage15-mirror        clj-meta lowering 위 stage15 mirror
px-runtime-pnix-mirror        내부 .px 런타임 mirror
```

`mirror-error` corpus(`resources/pnix_clj/mirror_error/cases.edn`)는 레인이
**오류 경계에 동의**할 때 행을 "수락"합니다. 자체 lineage 메모:

> "Accepted here means the Clojure evaluator and internal .px runtime mirror agree
> on the error boundary."

이는 *동의* 주장이지 *Nix 의미론* 주장이 아닙니다. 그 구분이 아래 전체의
요점입니다.

---

## 2. 증거: 4-레인 판정 (run-source로 캡처, 읽기 전용)

베이스라인 — 단순 구성은 모든 레인에서 완전 지원:

| source | evaluator | clj-meta | stage15-mirror | px-runtime |
|---|---|---|---|---|
| `1 + 1` | ok | ok | ok | ok |
| `{ a = 1; }` | ok | ok | ok | ok |
| `{ a = 1; }.a` | ok | ok | ok | ok |
| `let a = 1; in a` | ok | ok | ok | ok |
| `rec { a = 1; }.a` | ok | ok | ok | ok |
| `if true then 1 else 2` | ok | ok | ok | ok |

전방 참조와 사이클:

| source | evaluator | clj-meta | px-runtime | Nix-correct |
|---|---|---|---|---|
| `let a = b + 1; b = 10; in a` | **ok = 11** | held (clj-meta-eval-failed) | held (px-runtime-run-error) | **11** |
| `let a = a + 1; in a` | held :infinite-recursion | held | held | infinite-recursion |
| `let a = z + 1; in a` | held :unbound-var | held | held | unbound-var |
| `rec { x = y; y = 1; }.x` | **held :unbound-var** | held | held | **1 (ok)** |
| `rec { a = a + 1; }.a` | held :unbound-var | held | held | infinite-recursion |
| `rec { a = z + 1; }.a` | held :unbound-var | held | held | unbound-var |

(`clojure-stage15-mirror`는 이 행들에서 `clj-meta`를 따르며 폭을 위해 생략.)

---

## 3. 발견

### F1. Nix에서 `let`과 `rec`는 같은 재귀; 우리 evaluator는 `let`만 했음.
`let ... in`과 `rec { ... }` 모두 하나의 상호 재귀 스코프를 도입합니다. 우리
evaluator의 `eval-let`은 knot-tied 메모이즈 thunk를 쓰며 완전히 올바릅니다
(forward=11, cycle=infinite-recursion, unbound=unbound-var). `eval-attrs`(rec)는
환경을 *점진적으로* 만들어 전방 이름이 스코프에 없을 뿐입니다 — 모든 rec
forward/cycle 케이스가 `:unbound-var`로 붕괴. 따라서:

- `rec { x = y; y = 1; }.x` → **1**이어야 하나 `:unbound-var` (전방 참조 누락).
- `rec { a = a + 1; }.a` → **infinite-recursion**이어야 하나 `:unbound-var`
  (이름이 바인딩되지 않아 사이클조차 탐지 불가).

수정은 `eval-attrs`에 `eval-let`과 같은 knot-tied-thunk 스코프를 주는 것입니다
(되돌린 패치가 정확히 이 작업을 했고 직접 단위 테스트를 통과).

### F2. clj-meta / px-runtime 레인은 전방 참조에 대한 FRONTIER이며, 그 의미
판사가 아님.
그 레인들은 `let a = 1; in a`와 `rec { a = 1; }.a`에 `ok`를 주지만,
evaluator가 `11`로 계산하는 완전히 유효한 식 `let a = b + 1; b = 10; in a`에는
`held`를 줍니다. let/rec를 **순차** 바인딩으로 가정해 lower/execute하므로,
의미적으로 유효한지와 무관하게 전방 참조는 거기서 실패합니다. 따라서
`rec { x = y; y = 1; }.x`에 대한 그들의 `held`는 그 식이 오류인지를 **아무
것도** 말해 주지 않습니다 — 유효한 `rec { x = 1; y = x; }.y` 형태 전방 케이스에도
`held`할 것입니다.

### F3. rec-forward-reference에 대한 mirror-error "동의"는 SPURIOUS.
`rec { x = y; y = 1; }.x`에 대해 현재 모든 레인이 `held`이므로 행이
"수락(동의)"됩니다. 그러나 동의는 우연입니다:
- evaluator held = **rec 전방 참조 버그** (`:unbound-var`),
- clj-meta/px held = **전방 참조 frontier** (F2).
*단어* held에는 동의하지만 어떤 오류 의미론에도 동의하지 않습니다. 그 동의를
고정하면 evaluator 버그를 "예상됨"으로 고정한 셈입니다.

### F4. evaluator-ahead 발산은 이미 수락된 상태 — `let`에 대해.
`let a = b + 1; b = 10; in a`는 오늘 `evaluator=ok 11`이고 clj-meta/px는
`held`입니다. 그 발산은 지금 존재하며 **어떤 fixture도 플래그하지 않습니다**.
`rec { x = y; y = 1; }.x`를 `evaluator=ok 1`로 고치면 *정확히 같은 형태*의
발산 — 알려진 frontier보다 앞선 evaluator — 이 생기며, `let`에 대해 이미
허용됩니다. rec를 특별하게 만드는 유일한 것은 수정 전 버그 동의를 캡처한
mirror-error fixture뿐입니다.

### F5. 소유자가 제안한 `let-forward => unbound-var/error`는 Nix와도 우리
코드와도 맞지 않음.
분류 스케치에서 `let a = b + 1; b = 10; in a`가 `unbound-var/error`로 나열됨.
증거: Nix는 `11`로 평가하고, 우리 evaluator도 이미 `ok 11`을 반환. Nix에서
`let`은 `rec`처럼 재귀적. 분류가 `let-forward`와 `rec-forward`를 같은 클래스
(`*-forward-ok`)로 다루도록 권고 — 증거가 지지하는 것. 조용히 구현하지 않고
여기 플래그.

---

## 4. 제안 분류 (목표 판정, 네 레인 모두)

| 클래스 | 예 | evaluator | frontier 레인 (clj-meta/mirror/px) |
|---|---|---|---|
| `forward-ok` | `rec { x = y; y = 1; }.x`, `let a = b+1; b=10; in a` | **ok** | held @ frontier (레인이 전방 참조를 지원할 때까지) |
| `cycle-error` | `rec { a = a + 1; }.a`, `let a = a+1; in a` | held **:infinite-recursion** | held @ frontier |
| `unbound-error` | `rec { a = z + 1; }.a`, `let a = z+1; in a` | held **:unbound-var** | held @ frontier |

참고: `forward-ok`와 `cycle-error`는 "모든 레인이 오류에 동의"가 아닙니다.
"evaluator가 Nix 판정을 주고; 다른 레인은 선언된 전방 참조 frontier"입니다.
따라서 오류-동의 `mirror-error` corpus가 아니라 **명시적 frontier 마커가 있는
전방 참조 corpus**에 속합니다.

---

## 5. Receipt: 무엇이 바뀌어야 하고 왜 (조용히 하지 말 것)

1. **`mirror-error/rec-forward-reference`는 잘못 분류됨.** 그 `:source`
   `rec { x = y; y = 1; }.x`는 `forward-ok` 케이스(Nix = 1)이며, F1(evaluator
   버그) + F3(spurious 동의) 때문에 오류로만 캡처됨. mirror-error corpus를
   떠나야 합니다. Receipt = 위 F1–F4. 제거/이동은 서면 이유가 있는 fixture
   재분류이지 "부정을 녹색으로 뒤집기"가 아닙니다.

2. **`rec-cycle` 오류 fixture가 누락 / 이유가 잘못됨.**
   `rec { a = a + 1; }.a` → infinite-recursion을 주장하는 fixture가 없음; 오늘은
   `:unbound-var`(F1). 수정 후 기존 `let a = a + 1; in a` 동작을 미러링해
   이유 `:infinite-recursion`인 cycle-error corpus에 합류해야 함.

3. **`rec-unbound` fixture는 정당함.** `rec { a = z + 1; }.a` → `:unbound-var`는
   evaluator에서 올바르며 진짜 오류 케이스로 남을 수 있음(F2에 따라 frontier
   레인 표시).

4. **Frontier 레인에 명시적 마커가 필요함.** clj-meta/px-runtime이 모든 전방
   참조(유효 여부와 무관)에 `held`하므로, 전방 참조 fixture는 그 레인을
   `:frontier`(알려진 미지원)로 기록해야 함. 미래 레인 업그레이드가 조용한
   녹색이 아니라 의도적·증거 있는 변경이 되도록.

---

## 6. 다음 (감독된) 단계 — 여기서 하지 않음

1. §5에 따라 fixture 재분류(위 receipt 포함) — 소유자 검토.
2. knot-tied `eval-attrs` 수정(되돌린 패치) 적용해 evaluator 레인이
   `forward-ok = ok`, `cycle = infinite-recursion`을 주도록.
3. frontier 정책 결정: clj-meta/px-runtime을 선언된 전방 참조 frontier로
   표시하거나, 그 레인을 재귀 바인딩 지원으로 확장해 네 레인이 모두 `ok`에
   도달(더 큰 별도 작업).
4. 그 다음에야 게이트 기대를 뒤집고 receipt 트레일을 유지.

---

## 7. 연산자 엄격성 (별도 트랙) — 감사 전용, 동작 변경 없음

소유자 방향에 따라 `if <non-bool>`, `!<non-bool>`, `assert <non-bool>`, `+`
문자열 강제(`1 + "a"`)는 **관대하게 유지**. 계획은 엄격 의미론 아래 *실패할*
것을 증거/경고로 기록하는 `--strict-audit` 모드이며, 단계적 전환 전 동작
변경 없음. 여기서 시작하지 않음; rec 작업과 혼동하지 않도록 기록.
