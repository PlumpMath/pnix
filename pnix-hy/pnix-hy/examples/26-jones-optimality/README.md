# 26. Jones-optimality — 특화기가 해석 계층을 남기지 않는가 (proposal 0014)

## 무엇을
특화 결과가 해석 오버헤드를 남기지 않음을 코퍼스 전체로 게이트하는 `jones_optimality_report`:
533개 소스에서 `ir(p) == ir(parse(emit(p)))` (특화-왕복 후 IR 불변).

## 왜
인터프리터를 프로그램에 특화(1차 Futamura 사영)하면, **이상적으로는 인터프리터 계층이 완전히 사라져**
직접 컴파일한 것과 같아야 한다. "정말 사라졌는가"를 재는 척도가 **Jones-optimality**(특화기 품질의
gold standard) — plain하게는 검증할 수단이 없다.

## 무엇을 게이트하나
| 항목 | 값 |
|---|---|
| corpus / checked | 533 / 533 (전수, zip-truncation 없음) |
| hash mismatches | 0 |
| fixpoint failures | 0 |

## 한 줄
> 특화기가 좋다는 것은 "특화 후에도 IR이 그대로"라는 것 — 해석 계층을 남기지 않는다. 코퍼스 전체로 그것을
> 게이트한다.

## 경계
- IR 수준 척도(잔여-프로그램 품질). 관련: `poly_specialize`(0029 엔진)·specializer BTI(examples/21).
  정본 평가기·4-lane 미러 무접촉.
