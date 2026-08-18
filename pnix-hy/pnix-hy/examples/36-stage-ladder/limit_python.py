"""한계: 여러 실행 경로(직접/정규화/캐시/컴파일)가 같은 값을 내는지 검사하는
표준이 plain Python에 없다.

`eval()`은 단일 경로다. 파이프라인에 캐시나 컴파일 단계를 추가해도, 그
단이 원래 의미를 보존하는지 확인할 표준 "사다리" 개념이 없다 — 각 단을
따로 테스트하거나(귀찮음), 아예 검사를 생략한다(위험).
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

print("plain Python: 다단 실행 경로(stage tower/캐시/컴파일)의 값 일치를 검사할 표준이 없음.")
print("한계: '이 값이 모든 경로에서 같다'는 것을 사다리 하나로 볼 수 없다.")
