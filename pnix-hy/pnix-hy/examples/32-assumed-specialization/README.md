# 32. 가정 기반 특화 — speculative + guard (proposal 0025)

## 무엇을
정적 값을 **가정(assumptions)** 으로 명시해 특화하는 `specialize_pnix(..., assumptions=)`,
가정이 아직 맞는지 검사하는 `assumptions_valid`, 값이 바뀌었을 때만 재특화하는 `respecialize_if_drifted`.

## 왜
"값이 대개 고정"이라 가정하고 특화하면 빠르다(speculative optimization). 하지만 plain하게는 (1) 어떤
가정 하에 특화했는지 증거가 없고, (2) 가정이 깨졌는지(drift) 검사해 자동 재특화할 수 없다 → 조용히 틀린
코드를 계속 쓰게 된다. 가정은 **명시되고 검사되고 재특화의 guard**가 되어야 한다.

## 예
```
rec = specialize_pnix("a*x+b", ("x",), assumptions={"a":3,"b":4})
rec["residual_hy"]                          # (+ (* 3 x) 4)  — 3,4 박힘
assumptions_valid(rec, {"a":3,"b":4})       # True
assumptions_valid(rec, {"a":5,"b":4})       # False  (drift)
respecialize_if_drifted(..., env={"a":5,"b":4}, record=rec)
  → respecialized=True, (+ (* 5 x) 4)       # 자동 재특화
```

## 한 줄
> 정적 값을 **가정으로 명시**해 특화하면 — 가정이 깨졌는지 검사하고, 깨졌을 때만 재특화한다(투기적
> 최적화 + 안전장치). 조용히 틀리지 않는다.

## 경계
- Futamura 특화(examples/03)에 assumption/boundary 주석을 더한 것. 정본 평가기·4-lane 미러 무접촉.
