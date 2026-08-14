# cljs-meta fixed point

`cljs-meta`는 명시적이고 작은 JavaScript 런타임 커널 위에 ClojureScript
컴파일러 fixed point를 구축한다.

## Stage 시퀀스

```text
JVM-built stage 0 compiler
  -> stage 1 self-compiled compiler artifact
  -> stage 2 self-recompile
  -> stage 3 self-recompile
```

빌더는 최소 15 compiler generation을 실행한 뒤, 연속 두 artifact가
바이트 동일해질 때까지 계속한다(`PNIX_CLJS_MAX_STAGES` 상한, 기본 32).
Fixed-point 게이트는 다음을 모두 요구한다:

```text
stage 2 artifact bytes == stage 3 artifact bytes
stage 2 source closure == stage 3 source closure
stage 2 compiler input hash == stage 1 artifact hash
stage 3 compiler input hash == stage 2 artifact hash
stage 0 bootstrap-only namespaces are absent from the final artifact
```

명시적 trust root:

```text
Node.js
Google Closure runtime
cljs.core runtime
cljs.reader / cljs.tools.reader runtime
cljs.core macro bootstrap kernel
fixed-point stage harness
embedded cljs.core analysis cache
```

Analyzer, compiler, source-map 구현, 및 `cljs.js`는 self-compiled payload로
emit된다. Stage 0 JVM 컴파일러는 fixed artifact에 패키징되지 않는다.

## 빌드 및 검사

```sh
./bin/build-cljs
cat cljs-meta/dist/fixed-point/receipt.json
node cljs-meta/test/fixed_point_test.js
```

## Fixed 컴파일러 사용

```js
const cljs = require("./cljs-meta/dist/fixed-point/cljs-meta-fixed.js");

const evaluated = await cljs.evaluate("(let [x 20] (+ x 22))");
const compiled = await cljs.compile("(defn answer [] 42)");
```

`evaluate`와 `compile`은 `pnix.cljs-meta.result.v1` 투영을 반환한다.

## 크로스 플랫폼 클로저 체크리스트

현재 증거는 `x86_64-darwin`에 한정된다. `flake.nix`에 나타나거나 평가에
성공한다는 이유만으로 지원 플랫폼으로 보지 않는다.

- [x] `x86_64-darwin`
- [ ] `aarch64-darwin`
- [ ] `x86_64-linux`
- [ ] `aarch64-linux`

체크되지 않은 각 플랫폼은 독립적으로 다음을 만족해야 한다:

- [ ] 깨끗한 `target/`과 `dist/`에서 `./bin/build-cljs` 성공.
- [ ] Stage 2와 stage 3 artifact가 바이트 동일.
- [ ] Stage 2와 stage 3 source closure가 동일.
- [ ] Stage input hash가 stage 1이 stage 2를, stage 2가 stage 3를 컴파일했음을 증명.
- [ ] 최종 artifact에 stage 0 bootstrap-only namespace 없음.
- [ ] `node cljs-meta/test/fixed_point_test.js` 통과.
- [ ] `node cljs-meta/examples/fixed-point.js` 통과.
- [ ] `./bin/pnix-cljs-gate` 통과.
- [ ] `nix flake check path:. --no-write-lock-file` 네이티브 통과.
- [ ] `compile`과 `evaluate`가 이미 지원된 플랫폼과 동일한 정본 투영 생성.

다른 플랫폼의 artifact 해시는 비교·설명해야 한다. 플랫폼별 path, tool
version, timestamp, host rendering은 크로스 플랫폼 바이트 결정성을 주장하기
전에 정규화해야 한다. 위 항목이 모두 닫히기 전까지 문서와 receipt는
multi-platform 완료를 주장하지 말고 `platform-pending`이라고 써야 한다.
