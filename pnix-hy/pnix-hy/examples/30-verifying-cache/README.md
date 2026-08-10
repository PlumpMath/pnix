# 30. verifying 캐시 — 상태-해시로 검사 재사용 (proposal 0019)

## 무엇을
패키지 소스 상태의 해시 `package_state_hash()`와, 같은 상태면 자가검사를 다시 돌리지 않는
`cached_run(name, fn, state_hash=)`. 재사용본은 `cached=True` 마커를 남긴다(verifying trace).

## 왜
plain하게는 자가검사 스위트를 매번 전부 재계산한다 — 코드가 하나도 안 바뀌어도 줄일 수 없고, "이 결과가
지금 코드 상태에 대한 것"이라는 증거도 없다. 상태-해시 키는 둘 다 해결한다.

## 예
```
h = package_state_hash()
cached_run("ex30", check, state_hash=h)              # 같은 상태 재요청 → cached = True (replay)
cached_run("ex30", check, state_hash="ex30-other")   # 상태 바뀜 → 재계산 (cached 아님)
```
같은 상태는 검사를 replay(재계산 안 함), 상태가 바뀌면 해시가 달라져 **자동 무효화**. 실패는 캐시 안 됨.

## 한 줄
> 검사 결과에 **상태-해시** 키를 매기면 — 안 바뀐 코드는 재검사하지 않고, 재사용본은 "이 상태에 대한
> 결과"임을 마커로 증명한다.

## 경계
- `--check --cached`가 이 경로를 쓴다. 관련: content-addressed cache(examples/12), incremental
  eval(examples/22). 정본 평가기·4-lane 미러 무접촉.
