"""pnix-hy 방식: 가정 기반 특화 + drift 시 재특화 (proposal 0025).

`specialize_pnix(src, dynamic, assumptions=)`는 정적 값을 **가정(assumptions)** 으로 명시해 특화하고
그 가정을 레코드에 남긴다. `assumptions_valid(record, env)`로 가정이 아직 맞는지 검사하고,
`respecialize_if_drifted(...)`는 값이 바뀌었을 때만 자동으로 재특화한다(speculative optimization + guard).
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
import pnix_hy.pnix_mirror as pm

# a=3, b=4 를 가정하고 특화 (x만 동적)
rec = pm.specialize_pnix("a * x + b", ("x",), assumptions={"a": 3, "b": 4})
print("특화 결과(Hy):", rec["residual_hy"], "| 가정:", rec["assumptions"])
assert rec["residual_hy"] == "(+ (* 3 x) 4)"     # 3,4 가 박혔다

# 가정이 아직 맞나?
assert pm.assumptions_valid(rec, {"a": 3, "b": 4}) is True
assert pm.assumptions_valid(rec, {"a": 5, "b": 4}) is False   # a가 5로 drift
print("가정 검사: a=3,b=4 →", pm.assumptions_valid(rec, {"a": 3, "b": 4}),
      "| a=5 →", pm.assumptions_valid(rec, {"a": 5, "b": 4}))

# drift 시에만 자동 재특화
keep = pm.respecialize_if_drifted("a * x + b", ("x",), env={"a": 3, "b": 4}, record=rec)
redo = pm.respecialize_if_drifted("a * x + b", ("x",), env={"a": 5, "b": 4}, record=rec)
print("가정 유지 → respecialized:", keep["respecialized"],
      "| drift → respecialized:", redo["respecialized"], "새 코드:", redo["record"]["residual_hy"])
assert keep["respecialized"] is False and redo["respecialized"] is True
assert redo["record"]["residual_hy"] == "(+ (* 5 x) 4)"      # a=5로 재특화됨

assert pm.pe_annotations_report()["ready"]
print("→ 가정을 명시해 특화하고, drift를 검사해 필요할 때만 재특화한다.")
