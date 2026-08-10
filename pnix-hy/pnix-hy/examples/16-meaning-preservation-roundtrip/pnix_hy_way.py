"""pnix-hy의 방식 — 번역의 '의미 보존'을 값으로 증명하고 상태로 분류한다.

hy_to_pnix_value_roundtrip: Hy 조각을 pnix로 합성한 뒤, Hy의 Python 로우어링과 합성된 pnix를
'양쪽 다' 평가해 값이 같은지(meaning_preserved) 비교한다. roundtrip_status는 여러 프로젝션의
왕복을 공유 상태 어휘(lossless/lossy-ok/held/rejected)로 분류한다.

* Hy 1.3.0 proof Python 필요 (`nix develop` / PNIX_HY_PYTHON).
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402
from pnix_hy import pnix_mirror as pm  # noqa: E402


# 1) 의미 보존 증명: Hy '(+ 1 2)' -> 합성 pnix -> 양쪽 평가 -> 값 일치.
r = pm.hy_to_pnix_value_roundtrip("(+ 1 2)")
print("Hy -> pnix 합성:", repr(r["pnix_source"]), "| pnix 값:", r["pnix_value"])
print("의미 보존(meaning_preserved):", r["meaning_preserved"], "| 비교가능:", r["comparable"])
assert r["comparable"] is True and r["meaning_preserved"] is True and r["pnix_value"] == 3

# 2) 상태 어휘: 왕복 결과를 공유 vocabulary로 분류.
print("상태 어휘:", ph.ROUNDTRIP_STATUS_VOCAB)
st = ph.roundtrip_status("(x: x + 1)")
print("roundtrip 상태 분류:", st["statuses"])
assert set(ph.ROUNDTRIP_STATUS_VOCAB) == {"lossless", "lossy-ok", "held", "rejected"}

print("\n결론: 번역 전후 의미 일치를 값으로 증명하고, 결과를 표준 상태 어휘로 분류한다.")
