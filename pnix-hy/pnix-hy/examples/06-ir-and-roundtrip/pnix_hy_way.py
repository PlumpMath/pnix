"""pnix-hy의 방식 — 정본 IR(위치-무관, 해시 안정) + 값-동치 roundtrip.

lower_to_ir는 소스를 위치-무관 정본 IR로 낮추고, ir_of는 거기에 안정 sha256을 준다. eval_ir로
IR을 '직접' 평가할 수 있고, 그 값은 소스 평가와 동일하다(값-동치). 즉 IR이 정본이고, host로
내보낸 코드는 실행 아티팩트(캐시)일 뿐이다.
"""
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph  # noqa: E402


src = "let a = 1; in a + 2"

# 1) 위치-무관 정본 IR + 안정 해시.
bundle = ph.ir_of(src)
print("root tag:", bundle["root_tag"], "| IR 해시:", bundle["ir_sha256"][:16], "…")
assert len(bundle["ir_sha256"]) == 64

# 2) IR을 '직접' 평가 -> 소스 평가와 값이 같다 (값-동치).
ir = ph.lower_to_ir(src)
from_ir = ph.eval_ir(ir)
from_src = ph.safe_eval(src)["value"]
print("eval_ir:", from_ir, "| safe_eval:", from_src, "| 값-동치:", from_ir == from_src)
assert from_ir == from_src == 3

# 3) 포맷이 달라도 같은 정본 IR 해시로 수렴 (정규화).
h_a = ph.ir_of("let a=1; in a+2")["ir_sha256"]
h_b = ph.ir_of("let   a = 1 ;   in   a + 2")["ir_sha256"]
print("포맷 무관 정본 수렴:", h_a == h_b)
assert h_a == h_b

print("\n결론: IR이 정본(해시 안정·직접 평가·값-동치) -> 캐시/재현/이식의 기준점이 된다.")
