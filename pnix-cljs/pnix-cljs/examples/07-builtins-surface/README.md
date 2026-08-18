# 07 — builtins 표면 (seed)

## 쉽게 말하면 (비유)
Nix에 익숙하다면 `builtins.typeOf`/`attrNames`/`getAttr`가 낯익을 것이다.
pnix-cljs의 evaluator는 이런 Nix 스타일 builtin을 **150개 가까이** 구현해
뒀다(2026-08-11 maturity pass로 math/bitwise/list/attrset 헬퍼가 크게 늘었다) —
이 예제는 그중 대표적인 5개로 표면을 맛본다.

## 무엇을
게스트 표현에서 `builtins.typeOf`, `attrNames`, `getAttr`, `length` 등
**인정된 seed builtins**를 돌려 본다. clj 전 레인·전체 Nix 패리티를
주장하지 않는다 — 이 5개는 대표 샘플이지 전수 목록이 아니다.

## plain Node의 한계
JS에는 Nix 스타일 attrset(`typeOf`, `getAttrFromPath` 같은 구조적 introspection)
개념이 없다 — `typeof`/`Object.keys`로 비슷하게 흉내는 내지만, "이 값의
Nix 타입 이름이 뭔가"라는 질문 자체가 JS 쪽엔 없다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`)
```js
pnix.evalSource('builtins.typeOf 1')          // => 'int'
pnix.evalSource('builtins.attrNames { b = 2; a = 1; }')  // => ['a', 'b'] (정렬됨)
pnix.evalSource('builtins.getAttr "a" { a = 42; }')      // => 42
```
evaluator의 `builtins-value`가 `add`부터 `zipListsWith`까지 리스트/attrset/
문자열/수학/비트연산 builtin을 한 attrset으로 등록해 두고, `invoke-builtin`이
이름으로 디스패치한다.

## 어디에 쓰나
Nix 문법에 익숙한 사람이 pnix-cljs가 어디까지 같은 builtin을 지원하는지
감을 잡을 때. 전체 목록은 `src/pnix_cljs/evaluator.cljs`의
`builtins-value` 정의 참고.

## 실행
```bash
cd pnix-cljs
./bin/build-cljs   # dist 필요 시
node pnix-cljs/examples/07-builtins-surface/pnix_cljs_way.js
```
