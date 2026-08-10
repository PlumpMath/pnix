# 21. specializer 최적화 — binding-time-improvement 계열 (딥리서치 #2/#3 도출)

## 무엇을
`poly_specialize`(0029 cogen approach의 엔진)가 잔여 프로그램을 **작고 빠르게** 유지하는 4가지 BTI.
전부 순수 pnix라 **의미 불변** — 크기/공유만 개선.

| 기법 | 하는 일 | 예시 |
|---|---|---|
| **Q1-1** sharing-safe unfolding | dynamic 바인딩 다회 사용 → 공유 `let`(복제 금지) | `let y=x*x+7; in y+y` → `let y=((x*x)+7); in (y+y)` |
| **Q1-2** eta "The Trick" | dynamic `if`를 구조(attrset/list)로 분배 → 정적 필드 폴딩 | `(if b then {v=1} else {v=2}).v` → `(if b then 1 else 2)` |
| **I4** let-insertion | 분배로 생길 cond 중복을 최상위 공유 `let`으로 hoist | `…r.a + r.c` → `let __h1=..; in ((if __h1..)+(if __h1..))` |
| **0030** commuting conversion | 주변 연산을 `if` 브랜치로 밀어 폴딩(Bondorf CPS 효과) | `(if d then 10 else 20)+5` → `(if d then 15 else 25)` |
| **I1** bounded static variation | 위 분배가 크게 복제될 상황을 예산으로 억제 | `(if d..)+e`(e dynamic) → 밀지 않음 |

## 왜 (딥리서치가 확정한 것)
naive 부분평가는 (1) **sharing 손실**(dynamic 값을 각 사용처에 인라인 복제 → call-by-need 공유 파기,
잔여가 원본보다 느림; call-by-need는 CBV 문제 상속, 최대 33×, Brown&Palsberg POPL'18)과 (2) **문맥에
갇힌 정적계산**(dynamic `if`/구조 바깥이 안쪽 정적 부분을 dynamic으로 전파 → bloat, Danvy/Malmkjær/
Palsberg TOPLAS'96)으로 잔여를 부풀린다. 위 BTI들이 각각을 회복한다 — 단 "더 많은 정적계산"이 아니라
**sharing-safe 축약 + 문맥 전파 + 공유(let-insertion) + 예산(bounded)**으로.

## 쉽게 말하면 (비유)
```
naive:  같은 재료(x*x)를 요리마다 다시 손질하고(sharing 손실),
        상자(attrset) 통째로 들고 다니다 마지막에 하나만 꺼낸다(폴딩 실패).
pnix:   재료를 한 번 손질해 공유하고(let), 필요한 것만 미리 꺼내 둔다(분배·폴딩).
        결과(값)는 같다.
```

## 실측 (`pe_size_report`)
공유 부분식을 k=2/4/8/16회 사용해도 잔여에 **1회만** (33/45/69/117 bytes = 참조만 증가);
naive라면 k회 복제(선형 폭증). eta·let-insertion·commuting·bounded 전부 게이트로 고정.

## 코드 발췌
```python
import pnix_hy.tower as tw, pnix_hy.pnix_runtime as rt
r = tw.poly_specialize("let y = x * x + 7; in y + y", ("x",))["residual"]
assert r.count("x * x") == 1                          # 공유(복제 아님)
r = tw.poly_specialize("(if b then {v=1} else {v=2}).v", ("b",))["residual"]
assert "{" not in r                                   # 정적 필드 폴딩
```

## 한 줄
> 같은 부분평가라도 **BTI(sharing·eta·let-insertion·commuting·bounded)** 를 넣으면 잔여가 같은 의미로
> 더 작아진다 — 3회 딥리서치가 짚은 기법들을 순수 pnix specializer에 실현.

## 경계 (정직)
- 이 BTI들은 **제어흐름/공유 무거운 입력**에서 이득; 현재 헤드라인(인터프리터→컴파일러)은 static 분기라
  대부분 미등장 — 즉 **일반 specializer 강화**(툴킷 목적=언어 표현력 연구). 정직히 기록.
- 근거: `docs/audits/2026-07-02-cogen-stagepoly-research.md`, `…2026-07-03-laziness-stagepoly-research.md`,
  proposals `0029`(cogen)·`0030`(commuting). 정본 평가기(`pnix_runtime`)·4-lane 미러 무접촉.
