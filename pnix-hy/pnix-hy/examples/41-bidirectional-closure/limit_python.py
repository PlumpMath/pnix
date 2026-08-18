"""한계: 두 언어 사이를 오가는 소스 합성/왕복 안정성(closure) 개념이 plain
Python에 없다.

Python은 단일 언어라 "이 소스를 다른 언어로 합성했다가 되돌려도 다시 닫혀
있는가"를 물을 대상 자체가 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

print("plain Python: pnix<->Hy 소스 왕복 닫힘(closure) 개념이 없음.")
print("한계: 소스 합성 후 재왕복이 발산하지 않는지 확인할 방법이 없다.")
