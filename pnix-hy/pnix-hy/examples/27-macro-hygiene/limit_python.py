"""한계: 순진한 매크로/치환은 변수를 '실수로 포획'한다.

매크로가 도입한 임시 변수 이름이 사용자 코드의 변수와 우연히 겹치면, 확장 결과가 엉뚱한 변수를
가리킨다(hygiene 위반). plain 문자열 치환엔 이를 탐지할 수단이 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
# 순진한 매크로: 임시로 'tmp'를 쓴다
def macro_swap(a, b):
    return f"tmp = {a}; {a} = {b}; {b} = tmp"   # 'tmp'를 하드코딩

# 사용자 코드가 하필 'tmp'라는 변수를 쓰면? → 포획(capture)
expanded = macro_swap("tmp", "y")               # a='tmp' 자체
print("확장:", expanded)                         # tmp = tmp; tmp = y; y = tmp  → 망가짐
print("한계: 매크로 도입 심볼이 사용자 심볼을 포획해도 탐지 못함.")
