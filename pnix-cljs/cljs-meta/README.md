# cljs-meta

`cljs-meta`는 PNIX의 ClojureScript 호스트 메커니즘이다. 첫 실행 가능한
슬라이스는 `cljs.js`로 JavaScript/Node에서 ClojureScript를 평가하며,
proof receipt를 일상적인 PNIX 실행의 일부로 만들지 않는다.

```sh
../bin/build-cljs
node dist/cljs-meta.js -e '(let [x 20] (+ x 22))'
```

CommonJS:

```js
const cljsMeta = require("./dist/cljs-meta-module.js");
cljsMeta.evaluate("(+ 20 22)").then(console.log);
```

## 상태 / 기본 게이트

[STATUS.md](STATUS.md) 참고. 기본 게이트: `./bin/cljs-meta-gate` (fixed-point 테스트; 필요 시 빌드).

## Fixed point

`../bin/build-cljs`는 또한 격리된 컴파일러 stage 셋을 세 개 구성한다. Stage 2
와 stage 3는 빌드가 성공하기 전에 바이트 동일해야 한다.

```text
dist/fixed-point/cljs-meta-fixed.js
dist/fixed-point/cljs-meta-fixed-cli.js
dist/fixed-point/receipt.json
```

Self-hosted 컴파일러 클로저는 ClojureScript analyzer, compiler, reader, 및
`cljs.js`를 포함한다. 명시적 런타임 trust root는 `cljs.core`,
`cljs.tools.reader`, Google Closure runtime, 및 Node.js이다.

평가기는 호스트 메커니즘이다. PNIX 언어 의미론, 서비스 admission, 또는
artifact 승인을 소유하지 않는다.
