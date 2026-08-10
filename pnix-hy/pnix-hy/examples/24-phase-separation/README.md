# 24. phase 분리 — 컴파일/실행 단계의 대수 (proposal 0022)

## 무엇을
스테이징 연산에 정수 **phase shift**를 부여해 합성·상쇄를 계산하는 `phase_of`, 그리고 컴파일과 실행이
**관측적으로 분리**됨(lowering이 런타임 상태 무변경 + `eval(source)==eval(lower(source))`)을 게이트하는
`phase_separation_report`.

## 왜
plain eval/매크로는 "코드 생성(컴파일 시)"과 "실행(런타임)"을 대수적으로 구분하지 않는다. 그래서
quote/unquote가 상쇄되는지, 소스를 IR로 낮추는 일이 런타임 상태를 건드렸는지 **보장할 수단이 없다**.

## phase 값 (Racket 스타일)
| 연산 | shift |
|---|---|
| quote / quasiquote / for-syntax | **+1** (컴파일 쪽) |
| unquote / unquote-splice / for-template | **−1** (런타임 쪽) |
| read / eval / collapse | **0** (표현만 바뀜, 단계 유지) |

```
phase_of(["quote","unquote"])                         == 0   # 상쇄
phase_of(["for-syntax","for-syntax","for-template"])  == 1   # 합성
```

## 무엇을 게이트하나
- **P2 대수**: 합성·상쇄·결합이 성립.
- **P4 관측적 분리**: `lower_to_ir`가 어떤 런타임 상태도 변경하지 않음(empty-store) + `eval(source)`와
  `eval(lower(source))`가 값-동일(interleaving 무관성).

## 한 줄
> 스테이징을 **정수 phase**로 다루면 컴파일/실행 단계가 대수적으로 합성·상쇄되고, lowering의 순수성이
> 게이트된다 — "언제 계산되는가"가 값으로 관리된다.

## 경계
- pnix는 매크로가 없으므로 phase는 Hy-관측 투영(quasiquote/defmacro 등)과 collapse에 대해 정의됨. 정본
  평가기·4-lane 미러 무접촉.
