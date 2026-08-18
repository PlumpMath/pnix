# 40. Meta-circular tower — Hy 조각의 전체 여정 한 번에

## 무엇을
`meta_circular_tower`는 Hy 소스 한 조각의 **전체 여정**을 다섯 단계로 묶는다:
read(reader-form) → compile(Python + 매크로 tower) → run(추적 실행) →
pnix(합성 가능성/값 보존/닫힘) → collapse(특화 왕복). 지금까지의 예제들이
각 단계를 따로 보여줬다면(00: 기본 평가, 09: 물화, 16: 왕복, 35: staging
tower), 이건 그 전부를 **한 산출물**로 묶어서 본다.

## 왜
파이프라인의 각 단계를 따로 검증해도, "전체 사슬이 끊김 없이 하나로
이어진다"는 것은 별개 주장이다. plain Python은애초에 read/compile/run
이후의 "pnix로 갈 수 있는가", "다시 접어도(collapse) 값이 보존되는가"
같은 단계 자체가 없다.

## 무엇을 게이트하나
| 단계 | 확인 |
|---|---|
| read | reader-form 개수/내용 |
| compile | 생성된 Python 소스 + 매크로 tower(전개 흔적) |
| run | 실제 실행 값 + opcode 흔적(추적됨) |
| pnix | 합성된 pnix 소스, `synthesizable`/`value_preserved`/`comparable`/`closed` |
| collapse | 특화 왕복(`specialized_from` → 값 재확인) |

## 한 줄
> `(+ 1 2)` 하나가 read→compile→run→pnix→collapse 다섯 단계를 전부 통과하고
> 끝까지 값 3을 유지한다 — 사슬 어디도 안 끊긴다.

## 경계
- 이 예제는 **집계 뷰**다. 각 단계 자체의 상세 게이트는 `00`(read/run),
  `09`(reify), `16`(pnix 변환/보존), `35`(staging 내부)가 따로 진다.
