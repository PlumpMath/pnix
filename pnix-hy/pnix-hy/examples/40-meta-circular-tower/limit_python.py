"""한계: read/compile/run 이후 "pnix로 이어지는가", "다시 접어도(collapse)
값이 보존되는가"를 하나의 산출물로 잇는 개념이 plain Python에 없다.

각 단계(파싱, 컴파일, 실행)는 있지만, 그 뒤에 다른 언어 표현으로 넘어갔다가
되돌아와도 같은 값인지를 확인하는 다섯 단계 사슬 자체가 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

print("plain Python: read->compile->run->pnix->collapse 5단계 사슬이 없음.")
print("한계: 파이프라인 전체가 끊김 없이 이어진다는 걸 한 산출물로 못 봄.")
