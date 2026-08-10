"""pnix-hy 방식: SES 스타일 구획(compartment) — 이름·모듈 격리 (proposal 0021).

`Compartment`는 자기만의 바인딩·모듈 네임스페이스를 갖는 격리된 평가 구획이다. 서로 다른 구획은
같은 이름을 써도 섞이지 않고, intrinsics(순수 builtin)만 공유하며 back-leak이 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
import pnix_hy.compartment as cp

a = cp.Compartment()
b = cp.Compartment()
a.bind("x", "10")          # 구획 A의 x = 10
b.bind("x", "99")          # 구획 B의 x = 99 (독립)

print("A.eval('x + 1'):", a.eval("x + 1"), "| B.eval('x + 1'):", b.eval("x + 1"))
assert a.eval("x + 1") == 11 and b.eval("x + 1") == 100   # 서로 안 섞임
assert a.eval("x") == 10 and b.eval("x") == 99            # 각자 자기 x
print("A.names():", sorted(a.names()), "| B.names():", sorted(b.names()))

# 리포트: 상태 격리 + 모듈 격리 + intrinsics 공유 + back-leak 없음
r = cp.compartment_report()
print(f"binding_isolated={r['binding_isolated']} module_isolated={r['module_isolated']} "
      f"intrinsics_shared={r['intrinsics_shared']} no_backleak={r['no_backleak']}")
assert r["ready"] and r["binding_isolated"] and r["module_isolated"] and r["no_backleak"]
print("→ 구획마다 이름·모듈이 격리되고 back-leak이 없다 — 같은 이름도 안 섞인다.")
