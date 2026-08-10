"""한계: Python `eval`은 전역을 공유한다 — 컨텍스트 격리가 없다.

같은 프로세스에서 두 평가 컨텍스트를 돌리면, 한 쪽이 정의한 이름이 다른 쪽으로 새거나 서로를
덮어쓴다. plain `eval`에는 이름·모듈을 격리한 '구획(compartment)'이 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
g = {}
exec("x = 10", g)           # 컨텍스트 A
exec("x = 99", g)           # 컨텍스트 B — 같은 전역을 덮어씀
print("두 컨텍스트가 전역 공유:", g["x"])   # 99 — A의 10이 사라짐
assert g["x"] == 99
print("한계: 이름 격리 없음 — 한 컨텍스트가 다른 컨텍스트를 덮어쓴다.")
