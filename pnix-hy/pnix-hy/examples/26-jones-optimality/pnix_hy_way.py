"""pnix-hy 방식: Jones-optimality 게이트 (proposal 0014).

Jones-optimality = 특화기가 **해석 계층을 통째로 제거**할 수 있는가. 즉 인터프리터를 프로그램 p에
특화한 결과가, p를 직접 컴파일한 것과 **IR 수준에서 같아야** 한다. `jones_optimality_report`는
533-소스 코퍼스 전체에서 `ir(p) == ir(parse(emit(p)))` (특화-왕복 후 IR 불변)을 게이트한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy.pnix_mirror as pm

r = pm.jones_optimality_report()
print(f"corpus={r['corpus']} checked={r['checked']} "
      f"hash_mismatches={r['mismatch_count']} fixpoint_failures={r['fixpoint_failure_count']}")
# 코퍼스 전체에서 특화-왕복 후 IR이 불변(해석 오버헤드 없음)
assert r["ready"]
assert r["mismatch_count"] == 0 and r["fixpoint_failure_count"] == 0
assert r["checked"] == r["corpus"]  # 전수 검사(zip-truncation 없음)
print(f"→ {r['checked']}개 소스 전부 특화-왕복 IR 불변: 특화기가 해석 계층을 남기지 않는다(Jones-optimal).")
