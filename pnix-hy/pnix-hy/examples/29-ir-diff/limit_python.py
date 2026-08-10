"""한계: 텍스트 diff는 포맷 잡음에 속고, 의미 변화의 '위치'를 못 짚는다.

소스를 문자열로 비교하면
- 공백/줄바꿈만 달라도 diff가 뜨고(포맷 잡음),
- 실제 의미가 바뀐 곳이 AST의 '어디'인지 경로로 짚어주지 못한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import difflib
a = "let a = 1; in a + 2"
b = "let   a=1;\n in a + 2"     # 포맷만 다름, 의미 동일
diff = list(difflib.unified_diff(a.split(), b.split(), lineterm=""))
print("텍스트 diff (포맷만 다른데도 뜸):", [d for d in diff if d and d[0] in "+-"][:4])
print("한계: 포맷 잡음에 속고, 의미 변화 위치를 경로로 못 짚음.")
