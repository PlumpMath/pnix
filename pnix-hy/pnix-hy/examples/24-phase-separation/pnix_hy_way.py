"""pnix-hy 방식: phase 대수 + compile/run 관측적 분리 (proposal 0022).

각 스테이징 연산은 정수 **phase shift**를 갖는다: quote/quasiquote/for-syntax = +1(컴파일 쪽),
unquote/for-template = -1(런타임 쪽), read/eval/collapse = 0. `phase_of`가 시퀀스를 하나로 합성한다.
그리고 `phase_separation_report`는 (P2) 대수(합성/상쇄/결합)와 (P4) **lowering이 런타임 상태를 안
건드림 + eval(source)==eval(lower(source))**(관측적 무관성)을 게이트한다.
"""
import os, sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
import pnix_hy as ph
from pnix_hy import phase

# P2: 대수 — quote(+1) 다음 unquote(-1)은 상쇄되어 0
assert phase.phase_of(["quote", "unquote"]) == 0
assert phase.phase_of(["quasiquote", "unquote-splice"]) == 0
# 합성: for-syntax(+1) 두 번 + for-template(-1) = +1
assert phase.phase_of(["for-syntax", "for-syntax", "for-template"]) == 1
# read/eval/collapse는 phase 0 (표현만 바꾸지 단계는 안 바꿈)
assert phase.phase_of(["read", "eval", "collapse"]) == 0
print("phase 대수: quote·unquote →", phase.phase_of(["quote", "unquote"]),
      "| for-syntax×2·for-template →", phase.phase_of(["for-syntax", "for-syntax", "for-template"]))

# 툴킷 표면이 어느 phase에 있는지 (문서-as-데이터, drift 가시화)
print("표면→phase:", phase.SURFACE_PHASES)

# P4: 관측적 분리 — 리포트가 lowering 순수성 + eval==eval(lower) 를 게이트
rep = phase.phase_separation_report()
print(f"algebra={rep['algebra']} lowering_pure={rep['lowering_pure']} "
      f"observational_irrelevance={rep['observational_irrelevance']}")
assert rep["ready"] and rep["algebra"] and rep["lowering_pure"] and rep["observational_irrelevance"]
print("→ 컴파일/실행 단계가 정수로 합성·상쇄되고, lowering은 런타임 상태를 안 건드린다.")
