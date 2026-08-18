"""pnix-hy 방식: 공유 런타임 레인별 벤치마크 (`performance_report`).

프로세스 시작 잡음 없이 parse/정본 emit/컴파일러 emit/Python compile/
인터프리터 eval/컴파일+실행/exec-many 각 레인을 나노초 단위로 계측한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
from pnix_hy import hy_mirror as hm  # noqa: F401 - import order matters (circular init)
from pnix_hy import pnix_mirror as pm

r = pm.performance_report(iterations=5)
print(f"schema={r['schema']} ready={r['ready']} iterations={r['iterations']}")
print(f"generated_python_bytes={r['generated_python_bytes']} bytecode_op_count={r['bytecode_op_count']}")
for lane, t in r["timings"].items():
    print(f"  {lane:<16} per_iter_ns={t['per_iter_ns']:>10}")
assert r["ready"]
assert set(r["timings"]) >= {"parse", "canonical_emit", "compiler_emit", "python_compile", "interpreter_eval"}
print("→ 같은 파이프라인의 각 레인이 얼마나 걸리는지 나노초 단위로 분해된다.")
