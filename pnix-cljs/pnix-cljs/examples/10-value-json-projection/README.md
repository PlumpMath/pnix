# 10 — 값 JSON 투영

## 쉽게 말하면 (비유)
pnix 값(BigInt 정수, attrset, 리스트, bool)을 로그에 찍거나 다른 프로세스로
넘기려면 결국 텍스트가 필요하다. `evalValueJson`/`evalSourceJson`은 그
변환을 라이브러리가 대신 해준다 — 다만 그 JSON은 **값의 타입 권위**가
아니라 **관측용 투영**일 뿐이라는 걸 명확히 한다.

## 무엇을
`evalSourceJson`(결과 전체를 JSON으로)과 `evalValueJson`(값만 JSON으로)
두 표면으로 `{ a = 1; b = [ true false ]; }`를 투영해 본다.

## plain Node의 한계
`JSON.stringify`는 BigInt를 못 다룬다(`03-outcome-projection`에서 직접
겪은 문제) — pnix 값에 정수가 섞이면 그냥 `JSON.stringify`를 쓸 수 없다.
투영 규칙(BigInt→문자열/숫자, attrset 키 정렬 등)을 매번 직접 짜야 한다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`)
```js
pnix.evalSourceJson(src)
// => '{"outcome_kind":"done","schema":"...","value":{"a":1,"b":[true,false]}}'
pnix.evalValueJson(src)
// => '{"a":1,"b":[true,false]}'
```
라이브러리가 BigInt/attrset/리스트를 일관된 규칙으로 JSON 텍스트로
투영해준다 — **주의**: 이 JSON은 관측/직렬화용이지, pnix 값의 타입 자체를
정의하지 않는다(예: JSON 숫자는 pnix의 int/float 구분을 못 담는다).

## 어디에 쓰나
평가 결과를 로그, HTTP 응답, 다른 프로세스로 넘길 때 BigInt 직렬화를
직접 신경 쓰지 않고 싶을 때.

## 실행
```bash
node pnix-cljs/examples/10-value-json-projection/pnix_cljs_way.js
```
