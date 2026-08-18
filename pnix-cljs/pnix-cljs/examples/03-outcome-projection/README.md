# 03 — 결과 투영 (outcome)

## 쉽게 말하면 (비유)
JS 함수는 성공하면 값을 리턴하고 실패하면 예외를 던진다 — 성공/실패가
**서로 다른 제어 흐름**이라 한 곳에서 나란히 비교하기 번거롭다. pnix-cljs의
`evalSource`는 항상 **같은 모양의 값**(`{schema, outcome_kind, value|error}`)을
리턴해서, 성공/실패를 같은 방식으로 로그·비교·직렬화할 수 있다.

## 무엇을
세 가지 소스(정상 산술, 0으로 나누기, 정의 안 된 변수)를 같은 `evalSource`
로 돌려서 세 가지 `outcome_kind`(성공 + 두 종류의 실패 `class`)를 관측한다 —
clj 전체 receipt 타워와 같은 깊이는 아니다: seed 수준의 정직한 결과 모양.

## plain Node의 한계
`try { eval(src) } catch (e) { ... }`로 흉내 낼 수는 있지만, 실패 이유가
`e.message` 문자열 파싱에 의존한다 — "0으로 나눔"과 "정의 안 된 변수"를
프로그램적으로 구분할 안정된 `class` 필드가 없다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`)
```js
pnix.evalSource("1 / 0")
// => { outcome_kind: 'failed', error: { phase: 'eval', class: 'division-by-zero', evidence: {} } }
pnix.evalSource("missing_var")
// => { outcome_kind: 'failed', error: { phase: 'eval', class: 'unknown-variable', evidence: { name: 'missing_var' } } }
```
`error.class`가 실패 종류를 구조적으로 구분해 준다 — 문자열 매칭이 아니라
필드 비교로 분기할 수 있다.

## 어디에 쓰나
평가 결과를 로그/API 응답으로 그대로 직렬화해야 할 때, 실패 종류별로 다른
처리(재시도/사용자 메시지 분기)를 하고 싶을 때.

## 실행
```bash
node pnix-cljs/examples/03-outcome-projection/pnix_cljs_way.js
```
