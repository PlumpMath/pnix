# pnix-cljs 런타임

이 패키지는 PNIX seed 런타임의 활성 ClojureScript 구현이다.
PNIX/Nix-superset 소스를 직접 파싱하고 명목상 `Done` 또는 `Failed` 값을 반환한다.

```clojure
(require '[pnix-cljs.core :as pnix]
         '[pnix-cljs.outcome :as outcome])

(outcome/project (pnix/eval-source "20 + 22"))
```

JavaScript 호출자는 `dist/pnix-cljs-module.js`를 사용한다:

```js
const pnix = require("./dist/pnix-cljs-module.js");
pnix.evalSource("let x = 20; in x + 22");
```

의미 페이로드는 네이티브 ClojureScript 값으로 남는다. JSON 대면 프로젝션은
관찰 증거일 뿐, 언어 값도 타입 권위도 아니다.

**이중 축 / 라이브러리:** monorepo [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md),
[`../HOST_IMPORT.md`](../HOST_IMPORT.md).
