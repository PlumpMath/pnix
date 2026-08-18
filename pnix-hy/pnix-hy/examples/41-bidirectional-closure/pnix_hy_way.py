"""pnix-hy 방식: pnix<->Hy 양방향 투영 닫힘.

Hy->pnix 소스 합성, pnix->Hy->pnix / Hy->pnix->Hy 왕복이 안정적으로
닫히는지(closed) 확인한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import hy_mirror as hm  # noqa: F401 - import order matters (circular init)
import pnix_hy as ph

syn = ph.synthesize_pnix_from_hy("(+ 1 2)")
print(f"synthesize Hy->pnix: pnix_source={syn['pnix_source']!r} synthesizable={syn['synthesizable']} gaps={syn['gaps']}")
assert syn["synthesizable"] and not syn["gaps"]

pc = ph.pnix_projection_closure("1 + 2")
print(f"pnix->Hy->pnix: value={pc['value']} comparable={pc['comparable']} closed={pc['closed']}")
assert pc["value"] == 3 and pc["comparable"] and pc["closed"]

hc = ph.hy_projection_closure("(+ 1 2)")
print(f"Hy->pnix->Hy: comparable={hc['comparable']} closed={hc['closed']}")
assert hc["comparable"] and hc["closed"]

print("→ 양방향 왕복(pnix->Hy->pnix, Hy->pnix->Hy) 전부 닫혀 있다 — 발산하지 않는다.")
