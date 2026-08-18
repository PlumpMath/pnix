"""한계: Hy 언어 구성체(매크로 정의, import, quasiquote, reader macro, 매크로
전개 단계)를 구조화된 값으로 조회할 표준이 plain Python에 없다.

`ast` 모듈은 컴파일 후 트리만 준다 — "이 매크로가 정의될 때 어떤 파라미터를
받았나", "전개가 몇 단계 걸렸나"를 알 방법이 없다(Python엔 매크로 자체가
없다).
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))

print("plain Python: 매크로 정의/전개 단계/quasiquote 구멍을 구조화된 값으로 조회 불가.")
print("한계: Hy 언어 구성체 자체를 다루는 표준 프로젝션이 없다.")
