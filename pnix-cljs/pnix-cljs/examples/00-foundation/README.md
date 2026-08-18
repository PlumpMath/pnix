# 00 — 파운데이션 (cljs seed)

## 쉽게 말하면 (비유)
Node에서 `eval("20 + 22")`를 돌리면 결과 `42`만 온다. pnix-cljs는 같은 문제를
**별도 언어(pnix)를 위한 전용 파서/평가기**로 푼다 — `require`도 `eval`도
아니라, 문자열을 pnix 문법으로 읽고 pnix 의미로 평가해서 구조화된 결과를
돌려준다. **코어 00–06** 경로의 첫 단계([FOUNDATION_PATH.md](../FOUNDATION_PATH.md)).

## 무엇을
ClojureScript로 컴파일된 `dist/pnix-cljs-module.js`의 `evalSource`로 pnix 소스
5가지를 돌린다: 산술(`20 + 22`), 람다(`let double = x: x * 2; in double 21`),
조건(`if`), 재귀 바인딩(`rec { }`), 그리고 의도적 실패(`1 / 0`) — 성공과
실패 모양을 처음부터 같이 보여준다.

## plain Node의 한계
`eval`/`new Function`은 게스트 언어 경계가 없다 — 넘긴 문자열이 `process`,
`require`, 전역까지 그대로 닿는다. 그리고 그건애초에 **다른 언어(pnix)**를
파싱하지도 못한다 — JS 문법만 이해한다.

## pnix-cljs의 방식 (`node.js`)
```js
const pnix = require("../../dist/pnix-cljs-module.js");
pnix.evalSource("rec { answer = base + 2; base = 40; }.answer");
```
`evalSource`는 pnix 전용 파서로 읽고, pnix 전용 평가기로 계산해서
`{schema, outcome_kind, value}` 모양의 구조화된 결과를 돌려준다 — 실패해도
(`1 / 0`) 예외로 죽지 않고 `outcome_kind: "failed"`로 관측 가능하다.

## 어디에 쓰나
pnix-cljs가 무엇을 하는 도구인지 가장 빠르게 보는 진입점. 다음 단계는
`01-pure-eval-boundary`(순수성 경계)와 `07-builtins-surface`(builtin 표면).

## 실행
```sh
cd pnix-cljs
./bin/build-cljs
node pnix-cljs/examples/00-foundation/node.js
# 또는:
# node pnix-cljs/dist/pnix-cljs.js pnix-cljs/examples/00-foundation/program.px
```

카탈로그 색인: [../README.md](../README.md).
호스트 간 균형: 모노레포 `examples/EXAMPLES_BALANCE.md`.
