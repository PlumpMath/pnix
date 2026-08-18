"""pnix-hy 방식: 런타임 스테이지 사다리 (`pnix_stage_ladder`).

한 소스를 stage1-7 tower 동등성, store-backed(내용주소 캐시), compiler-closure
(인터프리터 vs 컴파일된 클로저) 세 단으로 통과시키고, 각 단이 같은 값을
내는지 게이트한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import hy_mirror as hm  # noqa: F401 - import order matters (circular init)
from pnix_hy import pnix_mirror as pm  # noqa: F401
from pnix_hy import stage as st

r = st.pnix_stage_ladder("let x = 1; y = 2; in x + y")
print(f"schema={r['schema']} passed={r['passed']} value={r['value']}")
for s in r["stages"]:
    print(f"  [{'OK' if s['ok'] else 'FAIL'}] {s['stage']}")
assert r["passed"]
assert r["value"] == 3
assert all(s["ok"] for s in r["stages"])
print("→ stage tower(7단) + store-backed + compiler-closure 전부 같은 값(3)을 가리킨다.")
