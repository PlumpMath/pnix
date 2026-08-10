"""plain의 한계 — 표현을 번역/변환할 때 '의미 보존'을 증명하지 않는다.

한 표현을 다른 표현으로 옮길 때(예: Hy -> pnix, AST -> 소스), Python 표준엔 "번역 전후가 같은
값/의미인가"를 검사하고 '상태(lossless/lossy/…)'로 분류해 주는 도구가 없다. 직접 양쪽을
평가해 비교하고, 상태 어휘도 스스로 정해야 한다.
"""
import ast

src = "1 + 2"
# 소스 -> AST -> 소스로 '번역'했다고 하자.
back = ast.unparse(ast.parse(src, mode="eval"))
print("번역:", repr(src), "->", repr(back))
print("의미 보존 증명?: 없음 (양쪽을 직접 eval해 비교해야 하고, 상태 어휘도 표준 아님)")
print("의미보존 상태 어휘(lossless/lossy-ok/held/rejected)?: 표준 없음")

print("\n결론: 번역이 의미를 보존하는지 '증명 + 상태 분류'하는 표준 표면이 없다.")
