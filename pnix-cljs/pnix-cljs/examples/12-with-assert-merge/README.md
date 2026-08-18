# 12 — with · assert · 병합 연산

## 쉽게 말하면 (비유)
`with { a = 1; }; a`는 JS의 (deprecated된) `with` 문과 이름은 같지만, pnix
쪽은 attrset의 키를 스코프에 명시적으로 끌어오는 안전한 구조다. `assert`는
조건이 거짓이면 그 자리에서 평가를 실패시키는 가드, `//`는 두 attrset을
오른쪽 우선으로 병합하는 연산자다.

## 무엇을
`with`(attrset 필드를 스코프로), `assert`(조건 가드), `++`(리스트 연결),
`//`(attrset 병합, 오른쪽 키 우선) 4가지, 그리고 주석이 평가에 영향 없음을
확인.

## plain Node의 한계
JS `with`는 strict mode에서 아예 금지된 문법이다. attrset 병합은
`{ ...a, ...b }` spread로 비슷하게 되지만, pnix `//`는 리스트가 아니라
attrset 전용 연산자로 문법에 내장돼 있고, `assert`처럼 "조건이 거짓이면
평가 자체를 실패로 만드는" 표현식은 JS에 없다(직접 `if (!cond) throw`를
써야 한다).

## pnix-cljs의 방식 (`pnix_cljs_way.js`, 실행 결과)
```
with { a = 1; }; a       => done 1
assert true; 9           => done 9
[1 2] ++ [3]              => done [ 1n, 2n, 3n ]
{ a = 1; } // { b = 2; } => done { a: 1n, b: 2n }
/* 주석 */ 1 + 1          => done 2
```

## 어디에 쓰나
설정 병합(`base // override`), 필드가 많은 attrset에서 반복적인 접두사
없이 필드를 참조하고 싶을 때(`with`), 함수 진입 조건을 강제하고 싶을 때
(`assert`).

## 실행
```bash
node pnix-cljs/examples/12-with-assert-merge/pnix_cljs_way.js
```
