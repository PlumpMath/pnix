"""plain Python의 한계 — AST는 '정본 IR'이 아니다.

Python의 ast는:
  1) 소스 위치/속성이 섞여 있어 '정규화된 정본 표현'이 아니고,
  2) 안정적 내용해시가 없으며,
  3) "이 정규화 표현을 평가하면 소스 평가와 값이 같다"는 성질을 표준으로 보장/관찰하지 않는다.
"""
import ast

tree1 = ast.parse("1 + 2 * 3", mode="eval")
tree2 = ast.parse("1 +   2*3", mode="eval")  # 같은 의미, 다른 포맷

d1 = ast.dump(tree1)
d2 = ast.dump(tree2)
print("포맷만 다른 두 소스의 ast.dump 동일?:", d1 == d2)  # 여기선 같지만...
print("ast.dump는 정본 '해시'가 아니다 (직접 정규화+해시해야 한다)")

# AST로 평가는 가능하지만, '정규화 IR을 평가한 값 == 소스 평가 값'을 언어가 보장/관찰해주지 않는다.
val = eval(compile(tree1, "<ex>", "eval"))
print("AST 평가:", val, "| 값-동치 증명은 직접 해야 한다")

print("\n결론: 위치-무관 정본 IR + 안정 해시 + 값-동치(roundtrip)가 기본 제공되지 않는다.")
