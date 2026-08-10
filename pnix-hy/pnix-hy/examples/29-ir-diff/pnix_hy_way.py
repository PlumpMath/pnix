"""pnix-hy 방식: 구조적 IR diff + 발산 경로 (proposal 0018).

`ir_diff(a, b)`는 두 소스를 **정규화된 IR**로 낮춘 뒤 구조적으로 비교한다.
- 포맷(공백/줄바꿈)만 다르면 `equal=True`(잡음 무시),
- 의미가 다르면 발산 지점을 **AST 경로**로 짚는다(first_divergence_path).
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy.ir as ir

# 포맷만 다름 → IR 동일
same = ir.ir_diff("let a = 1; in a + 2", "let   a=1;\n in a  +  2")
print("포맷만 다름:", "equal =", same["equal"], "| diff_count =", same["diff_count"])
assert same["equal"] is True and same["diff_count"] == 0

# 의미 변화(a+2 → a+3): 발산을 경로로 짚음
d = ir.ir_diff("let a = 1; in a + 2", "let a = 1; in a + 3")
print("의미 변화:", "equal =", d["equal"], "| 경로 =", d["first_divergence_path"])
assert d["equal"] is False
assert d["first_divergence_path"] == ["body", "rhs", "value"]   # 정확히 rhs 리터럴

assert ir.ir_diff_report()["ready"]
print("→ 포맷 잡음은 무시, 의미 변화는 AST 경로로 정확히 지목.")
