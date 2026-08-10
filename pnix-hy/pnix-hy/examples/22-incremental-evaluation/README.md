# 22. 증분 평가 — 의미가 정체성 (proposal 0023)

## 무엇을
top-level `let` 정의마다 **의존성-치환 content hash**를 매겨, 안 바뀐 정의는 재사용하고 바뀐 것(+의존자)만
재계산하는 `incremental_eval`. 그리고 같은 IR은 평가 없이 결과를 증명하는 `realisation_record`(Nix-CA 유사).

## 왜
이름/텍스트를 정체성으로 쓰면 (1) 의미가 같아도 **alpha-rename**에 캐시가 무효화되고, (2) 무거운 정의
하나만 바뀌어도 **전체 재계산**한다. content 기반(Unison 방식)은 형제 참조를 그 정의의 hash로 치환한 뒤
해싱 → **이름은 메타데이터**, 정체성은 의미.

## 쉽게 말하면 (비유)
```
이름 기반 : "big"과 "huge"는 다른 서랍 → 같은 물건이어도 다시 산다.
content   : 물건 자체로 식별 → 이름을 바꿔도 이미 있는 걸 재사용.
```

## 실측 (`incremental_eval`)
| 상황 | hits / misses |
|---|---|
| cold (3 정의 처음) | 0 / 3 |
| warm (동일 재실행) | 3 / 0 |
| 정의 1개 변경 | 2 / 1 |
| **alpha-rename** (big→huge) | **3 / 0** (의미 불변) |
| realisation 재실행 | early cutoff (평가 없음) |

## 한 줄
> 정의의 정체성을 **이름이 아니라 의미(content hash)** 로 두면 — 부분 재사용 + alpha-rename 면역 +
> 같은 IR의 early cutoff.

## 경계
- 지원 대상 = 순수·데이터-값 top-level `let`; 그 외(비-let, 순환, 함수-값 정의, impure)는 **전체 평가로
  안전 폴백**(캐시 miss는 안전, 잘못된 hit는 불가). 정본 평가기 무접촉.
