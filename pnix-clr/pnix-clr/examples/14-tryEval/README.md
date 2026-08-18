# 14 — tryEval

## 쉽게 말하면 (비유)
`builtins.tryEval expr`는 pnix 소스 **내부**에서 쓰는 try/catch다 — C#의
`try { ... } catch { }`가 예외를 잡아 결과 객체를 스스로 만들어야 하는
것과 달리, pnix는 이걸 언어 표현식으로 직접 제공한다.

## 무엇을
`builtins.tryEval`로 throw/실패를 구조화된 `{ success, value }`로 받는다.
성공 경로(`tryEval (1 + 1)`)와 `throw`가 발생하는 실패 경로 둘 다 확인.
타입 오류 catch 범위는 호스트·Nix 버전마다 다를 수 있음 — seed 수준
스모크.

## plain .NET의 한계
C#에는 "표현식이 던지면 실패를 값으로 감싸서 돌려주는" 내장 연산자가
없다 — `try { v = expr(); } catch (Exception e) { v = Failure(e); }`처럼
매번 직접 감싸야 한다. pnix `tryEval`은 그 감싸기를 언어 차원에서
제공한다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e 'builtins.tryEval (1 + 1)'
{"host":"pnix-clr","outcome_kind":"done",...,"value":{"success":true,"value":2}}

$ ./bin/pnix-clr -e 'builtins.tryEval (throw "x")'
{"host":"pnix-clr","outcome_kind":"done",...,"value":{"success":false,"value":false}}

$ ./bin/pnix-clr pnix-clr/examples/14-tryEval/sample.px
{"host":"pnix-clr","outcome_kind":"done",...,
 "value":{"bad":{"success":false,"value":false},"ok":{"success":true,"value":2}}}
```
`throw`가 실제로 던져져도 CLI 최상위 `outcome_kind`는 여전히 `done`이고,
실패는 `success: false`로 값 안에 담긴다 — 언어 실행 자체는 안 끊긴다.

## 어디에 쓰나
"이 하위 표현식이 실패할 수 있는데, 전체 평가는 계속 진행돼야 한다"는
상황(예: 선택적 필드 접근, 검증 스크립트에서 실패 항목만 모으기).

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e 'builtins.tryEval (1 + 1)'
./bin/pnix-clr -e 'builtins.tryEval (throw "x")'
./bin/pnix-clr pnix-clr/examples/14-tryEval/sample.px
```
