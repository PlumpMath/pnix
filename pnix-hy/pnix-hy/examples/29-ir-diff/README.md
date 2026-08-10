# 29. IR diff — 포맷 무시, 의미 변화 위치 지목 (proposal 0018)

## 무엇을
두 소스를 정규화된 IR로 낮춘 뒤 구조적으로 비교하는 `ir_diff`: 포맷만 다르면 `equal=True`, 의미가 다르면
발산 지점을 **AST 경로**(`first_divergence_path`)로 짚는다.

## 왜
텍스트 diff는 (1) 공백/줄바꿈만 달라도 잡음이 뜨고, (2) 의미가 바뀐 곳이 AST의 어디인지 짚지 못한다.
IR 수준 비교는 포맷을 무시하고 변화 위치를 정확히 지목한다.

## 예
```
ir_diff("let a=1; in a+2", "let  a = 1 ;\n in a  +  2")   # 포맷만 다름
  → equal = True,  diff_count = 0
ir_diff("let a=1; in a+2", "let a=1; in a+3")             # 의미 변화
  → equal = False, first_divergence_path = ["body","rhs","value"]
```

## 한 줄
> 소스가 아니라 **정규화된 IR** 을 비교하면 — 포맷 잡음은 사라지고, 의미가 바뀐 **정확한 위치**가 경로로 남는다.

## 경계
- IR은 정본(canonical), 방출 소스는 실행 아티팩트/캐시. 관련: ir/roundtrip(examples/06), pass reification
  (`ir_pipeline`). 정본 평가기·4-lane 미러 무접촉.
