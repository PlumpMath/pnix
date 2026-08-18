"""한계: 호스트 함수/메서드 호출에 effect-class/capability/witness 증거가
남지 않는다.

`func(*args)`나 `getattr(obj, name)(*args)`는 그냥 호출될 뿐이다 — 이
호출이 순수했는지, 어떤 종류의 effect였는지, 재현 가능한 증거가 남았는지는
아무것도 기록되지 않는다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

print("plain Python: 호스트 함수/메서드 호출에 effect-class/witness 증거가 없음.")
print("한계: 호출 성공/실패를 구조화된 {success, value|error} 모양으로 못 받음.")
