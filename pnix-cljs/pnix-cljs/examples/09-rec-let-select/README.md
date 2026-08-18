# 09 — rec / let / select

## 쉽게 말하면 (비유)
`rec { a = b + 1; b = 41; }.a`처럼, attrset 안의 필드가 **선언 순서와 무관하게**
서로를 참조할 수 있다 — JS 객체 리터럴에서는 한 속성이 아직 정의 안 된
다른 속성을 참조하면 `undefined`가 나오지만, pnix의 `rec`는 이걸 안전하게
묶어 계산한다.

## 무엇을
Nix 유사 핵심 문법 4가지를 seed 평가기로 확인한다: `let`(지역 바인딩),
`rec { }`(상호 참조 가능한 재귀 attrset), 속성 선택(`.`), 중첩 선택
(`s.a.b`).

## plain Node의 한계
```js
const obj = { a: b + 1, b: 41 };  // ReferenceError: b is not defined
```
JS 객체 리터럴 필드는 **선언 순서대로, 아직 없는 형제 필드를 참조 못 하며**
평가된다. Nix/pnix의 `rec`처럼 "이 attrset 전체를 하나의 상호재귀
스코프로 묶는" 개념이 없다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`)
```js
pnix.evalSource("rec { a = b + 1; b = 41; }.a")
// => { outcome_kind: 'done', value: 42n }   (b가 아래 선언돼도 a가 b를 본다)
pnix.evalSource("let s = { a = { b = 1; }; }; in s.a.b")
// => 1   (중첩 선택)
```

## 어디에 쓰나
설정 파일처럼 서로 참조하는 필드가 많은 데이터를 pnix로 표현할 때(예:
`base`값 하나를 여러 파생 필드가 참조).

## 실행
```bash
node pnix-cljs/examples/09-rec-let-select/pnix_cljs_way.js
```
