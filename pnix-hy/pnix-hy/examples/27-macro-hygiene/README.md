# 27. 매크로 hygiene — 실수 포획을 탐지 (proposal 0017)

## 무엇을
매크로/치환의 **변수 포획(capture)** 을 탐지하고 fresh binder의 청결을 게이트하는 `hygiene_report`:
심어둔 충돌에서 capture를 실제로 잡고, gensym binder가 사용자 심볼을 오염시키지 않음을 확인.

## 왜
매크로가 도입한 임시 이름이 사용자 코드 변수와 겹치면 확장 결과가 엉뚱한 변수를 가리킨다(hygiene 위반).
순진한 문자열 치환은 이를 **탐지할 수단이 없다**.

## 무엇을 게이트하나
| 항목 | 뜻 |
|---|---|
| `capture_detected` = True | 심어둔 충돌에서 포획을 **실제로 탐지**(탐지기 작동) |
| `fresh_binder_clean` = True | gensym binder는 사용자 자유변수를 오염 안 함 |
| `macro_expansion_ok` = True | 확장이 도입한 심볼이 자유변수를 포획 안 함 |

## 한 줄
> hygiene = "매크로가 도입한 이름이 사용자 이름을 우연히 포획하지 않는다" — 그 위반을 심어놓고 **탐지되는지**
> 게이트한다.

## 경계
- pnix 자체는 매크로가 없으므로 hygiene은 Hy-매크로 투영/치환 관점에서 정의됨. 정본 평가기·4-lane 미러 무접촉.
