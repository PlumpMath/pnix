# 01 — 순수 평가 경계 (Node)

## 쉽게 말하면 (비유)
Node의 `eval`/`new Function`은 신뢰 경계가 없다 — 넘긴 문자열이 `process`,
`require`, 파일시스템까지 그대로 닿는다. pnix-cljs는 **문자열 게스트 소스**를
전용 파서/평가기 경계로 넘기고, 성공이든 실패든 예외로 죽는 대신 관측 가능한
`{outcome_kind, value|error}` 모양으로 돌려준다.

## 무엇을
`evalSource`로 순수 산술(`1 + 2 * 3`)과 의도적 오류(`1 / 0`)를 나란히 돌려서,
두 경로 모두 **구조화된 결과**로 남는 것을 확인한다.

## plain Node의 한계 (`limit_node.js`)
위험 패턴은 실행하지 않고 주석으로만 표시한다:
```js
// eval("require('fs').readFileSync('/etc/passwd','utf8')")
// new Function("return process.env")()
```
`eval`/`new Function`은 넘긴 문자열이 JS 전역 스코프에서 그대로 실행된다 —
`fs`/`process` 접근을 막을 표준 방법이 없다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`)
```js
pnix.evalSource("1 + 2 * 3")
// => { schema: 'pnix.machine.host-outcome.v1', outcome_kind: 'done', value: 7n }
pnix.evalSource("1 / 0")
// => { outcome_kind: 'failed', error: { phase: 'eval', class: 'division-by-zero', evidence: {} } }
```
게스트 소스는 평가기 API를 통해서만 다뤄지고, Node `require`/`fs`에 직접
닿지 않는다. (hy/rs 수준의 전체 effect 게이트를 주장하지 않는다 — admitted
seed만.)

## 어디에 쓰나
신뢰할 수 없는 pnix 소스를 Node 서버/CLI에서 평가해야 할 때, 실패를
`try/catch` 대신 값으로 다루고 싶을 때.

## 실행
```bash
cd pnix-cljs
./bin/build-cljs   # dist 필요 시
node pnix-cljs/examples/01-pure-eval-boundary/limit_node.js
node pnix-cljs/examples/01-pure-eval-boundary/pnix_cljs_way.js
```
