# 36. Stage ladder — 여러 실행 경로가 같은 값을 내는가

## 무엇을
"같은 프로그램을 평가한다"에는 여러 경로가 있다: stage1-7 tower 동등성(직접
평가부터 stage7 컴파일러까지), store-backed(내용주소 캐시를 거친 평가),
compiler-closure(인터프리터 vs 컴파일된 클로저). `pnix_stage_ladder`는 한
소스에 대해 이 경로들을 전부 밟고, 각 단이 같은 값/해시를 내는지 본다.

## 왜
파이프라인의 여러 단(파스/정규화/캐시/컴파일)이 늘어날수록, "어느 단에서
의미가 조용히 바뀌었다"는 회귀는 흔하다. plain Python `eval()`은애초에
이런 다단 파이프라인 개념이 없어서 비교할 대상조차 없다.

## 무엇을 게이트하나
| 항목 | 값 |
|---|---|
| stage-tower 1..7 동등성 | stage1/2/5/6/7 동일 해시군, stage3/4는 lowered-form 해시군(둘 다 내부 정합) |
| store-backed | cacheable 여부 + 캐시 hit/miss 표시 |
| compiler-closure | 인터프리터 결과 == 컴파일된 클로저 실행 결과 |

## 한 줄
> `let x = 1; y = 2; in x + y` 하나를 stage tower 7단 + store + compiler
> closure까지 전부 통과시켜도 값(3)이 안 흔들린다 — 사다리 전체가 같은 것을
> 가리킨다.

## 경계
- `02-determinism-and-drift`(단일 해시 안정성)와 `20-efficient-cogen`
  (compiled runtime 자체)의 상위 결합 뷰. 새 stage를 추가하지 않고, 기존
  경로들이 서로 일치하는지만 본다.
