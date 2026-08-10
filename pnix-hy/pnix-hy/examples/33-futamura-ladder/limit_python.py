"""한계: 인터프리터 하나로 컴파일러·컴파일러생성기를 '파생'할 수 없다.

plain하게는 인터프리터를 짜면 그걸로 끝 — 컴파일러를 원하면 따로 손으로 짜야 하고, 컴파일러
생성기(cogen)는 더더욱 그렇다. 세 가지(해석·컴파일·생성)가 하나의 인터프리터에서 파생된다는
Futamura 사다리를 관측할 수단이 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
INT = "if node==num: return v; elif node==add: return e(l)+e(r) ..."  # 개념
print("plain: 인터프리터는 그냥 인터프리터 — 컴파일러·cogen은 따로 손으로.")
print("한계: 1차(해석 붕괴)·2차(컴파일러)·3차(cogen) 사영을 파생·관측 불가.")
