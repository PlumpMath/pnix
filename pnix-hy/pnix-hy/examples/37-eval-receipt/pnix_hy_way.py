"""pnix-hy 방식: 재현성/감사 영수증 (`eval_receipt`).

정본 emit + 소스 해시, 값 + 값 해시, 4-lane 수렴(host interp/compiler +
stage7 runtime/compiler), 실행 흔적, 순수성을 한 장으로 묶는다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import hy_mirror as hm  # noqa: F401 - import order matters (circular init)
from pnix_hy import pnix_mirror as pm

SRC = "let x = 1; y = 2; in x + y"
r1 = pm.eval_receipt(SRC)
r2 = pm.eval_receipt(SRC)  # 재현성: 같은 소스 -> 같은 영수증 해시
print(f"value={r1['value']} pure={r1['pure']}")
print(f"source_sha256={r1['source_sha256'][:12]}... value_sha256={r1['value_sha256'][:12]}...")
print(f"convergence={r1['convergence']['converged']} lanes={r1['convergence']['lanes']}")
print(f"trace={r1['trace']}")
assert r1["ok"] and r1["value"] == 3
assert r1["convergence"]["converged"]
assert r1["trace"]["runtime_calls_total"] > 0
assert r1["source_sha256"] == r2["source_sha256"]
assert r1["value_sha256"] == r2["value_sha256"]
print("→ 한 장의 영수증에 값·수렴·실행흔적이 담기고, 재현하면 두 해시가 그대로다.")
