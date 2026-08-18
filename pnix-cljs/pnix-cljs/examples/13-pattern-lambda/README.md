# 13 — 패턴 람다

## 쉽게 말하면 (비유)
`({ a, b ? a }: b)`는 JS의 구조분해 인자(`({ a, b = a }) => b`)와 거의
같은 느낌이다 — attrset을 받는 함수가 필요한 키를 이름으로 뽑아 쓰고,
없으면 기본값이나 다른 인자를 참조하도록 선언한다.

## 무엇을
attrset 인자 패턴 3가지: 필수 키(`{ a }`), 기본값(`{ a ? 2 }`), 다른
패턴 필드를 기본값으로 참조(`{ a, b ? a }`), 그리고 커링(`x: y: x + y`).

## plain Node의 한계
JS 구조분해 기본값(`({ a, b = a }) => b`)은 실제로 가능하지만, pnix 쪽
패턴은 그 자체가 함수의 **유일한 형식 인자 형태**로 문법에 내장돼 있고
Nix 문법과 정확히 대응한다는 점이 다르다 — pnix 소스를 그대로 옮겨써도
같은 패턴-매칭 규칙이 pnix-cljs/clj/hy/rs/clr 전 호스트에서 동일하게
작동해야 한다(이 예제는 그중 cljs 쪽 확인).

## pnix-cljs의 방식 (`pnix_cljs_way.js`, 실행 결과)
```
({ a }: a) { a = 7; }              => done 7
({ a ? 2 }: a) {}                  => done 2
({ a, b ? a }: b) { a = 3; }       => done 3
let f = x: y: x + y; in f 1 2      => done 3
```

## 어디에 쓰나
함수형 설정 API(예: `{ name, version ? "0.1.0" }: ...`)를 pnix 소스로
표현할 때, 커링으로 부분 적용 함수를 만들 때.

## 실행
```bash
node pnix-cljs/examples/13-pattern-lambda/pnix_cljs_way.js
```
