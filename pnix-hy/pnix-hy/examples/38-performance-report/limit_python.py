"""한계: 언어 파이프라인의 parse/emit/compile/exec 각 레인을 의미 단위로
쪼개 반복 측정하는 표준 도구가 plain Python에 없다.

`timeit`은 임의 호출 가능 객체 하나를 잰다. "이 언어의 파이프라인 중 어느
단이 느린가"를 parse/emit/compile/exec 레인별로 나눠 재는 개념이 없다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

print("plain Python: 언어 파이프라인 레인별(parse/emit/compile/exec) 벤치마크가 없음.")
print("한계: '어디가 느린가'를 파이프라인 의미 단위로 쪼갤 표준이 없다.")
