# 28. 숫자 경계 — 변환 전에 무손실 판정 (proposal 0015)

## 무엇을
숫자 경계 변환을 **하기 전에** 안전한지 답하는 predicate `numeric_fits(value, kind)`
(GraalVM `fitsIn*` 스타일).

## 왜
Python은 큰 int→float에서 정밀도를 **조용히** 잃는다(오류·경고 없음). host↔pnix 경계에서 이건 은밀한
버그가 된다. 변환 전에 "무손실인가"를 물을 수 있어야 한다.

## kind
| kind | 묻는 것 |
|---|---|
| `int` | float이 정확한 정수를 담는가 (`3.0`✓ `3.14`✗) |
| `float` | int이 float 왕복(53-bit)에서 살아남는가 (`2^53`✓ `2^53+1`✗) |
| `json-number` | 유한하며 JSON 안전-정수 범위인가 |

## 한 줄
> 경계 변환을 **하기 전에** `numeric_fits`로 물으면 — 조용한 정밀도 손실 대신 미리 거절(blame 있는
> `InteropError`)할 수 있다.

## 경계
- interop 경계 전용. 관련: interop loss/effect(examples/04), 하드닝/blame(examples/23). 4-lane 미러 무접촉.
