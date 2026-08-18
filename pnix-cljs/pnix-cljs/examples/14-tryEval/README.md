# 14 — tryEval

## 쉽게 말하면 (비유)
`builtins.tryEval expr`는 pnix 소스 **내부**에서 쓰는 try/catch다 — JS의
`try { ... } catch {}`가 예외를 잡아 `{success, value}` 비슷한 걸 스스로
만들어야 하는 것과 달리, pnix는 이걸 언어 표현식으로 직접 제공한다.

## 무엇을
성공 경로(`tryEval (1 + 1)`)와 `throw`가 발생하는 실패 경로 두 가지로
`{ success, value }` 구조를 확인.

## plain Node의 한계
JS에는 "표현식이 던지면 실패를 값으로 감싸서 돌려주는" 내장 연산자가
없다 — `try { v = expr() } catch (e) { v = { success: false } }`처럼
매번 직접 감싸야 한다. pnix `tryEval`은 그 감싸기를 언어 차원에서
제공한다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`, 실행 결과)
```
builtins.tryEval (1 + 1)        => done { success: true, value: 2n }
builtins.tryEval (throw "x")    => done { success: false, value: false }
```
`throw`가 실제로 던져지면 `tryEval`의 최상위 평가(`outcome_kind`)는 여전히
`done`이고, 실패는 `success: false`로 값 안에 담긴다 — 언어 실행 자체는
안 끊긴다.

## 어디에 쓰나
"이 하위 표현식이 실패할 수 있는데, 전체 평가는 계속 진행돼야 한다"는
상황(예: 선택적 필드 접근, 검증 스크립트에서 실패 항목만 모으기).

## 실행
```bash
node pnix-cljs/examples/14-tryEval/pnix_cljs_way.js
```
