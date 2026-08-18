# 07 — builtins / lib 표면

## 쉽게 말하면 (비유)
Nix에 익숙하다면 `builtins.typeOf`/`getAttrFromPath`/`lib.sum`이 낯익을
것이다. pnix-clr의 seed 평가기는 이런 Nix 스타일 builtin과 `lib` 별칭을
상당 폭 구현해 뒀다(2026-08-11 성숙도 패스로 math/bitwise/list/attrset
헬퍼가 크게 늘었다) — 이 예제는 그중 대표적인 3개를 확인한다.

## 무엇을
clr seed가 인정하는 builtins·`lib` 별칭을 CLI로 확인한다: `typeOf`(타입
introspection), `getAttrFromPath`(경로로 중첩 속성 접근), `lib.sum`(리스트
합산). README 제품 코퍼스와 같은 방향.

## plain .NET의 한계
.NET에는 Nix 스타일 attrset의 구조적 introspection(`typeOf`,
`getAttrFromPath`)에 대응하는 표준 개념이 없다 — 리플렉션으로 흉내는
내지만, "이 값의 Nix 타입 이름이 뭔가"라는 질문 자체가 BCL 쪽엔 없다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e 'builtins.typeOf 1'
{"host":"pnix-clr","outcome_kind":"done",...,"value":"int"}

$ ./bin/pnix-clr -e 'builtins.getAttrFromPath [ "foo" "bar" ] { foo.bar = 42; }'
{"host":"pnix-clr","outcome_kind":"done",...,"value":42}

$ ./bin/pnix-clr -e 'lib.sum [1 2 3 4]'
{"host":"pnix-clr","outcome_kind":"done",...,"value":10}

$ ./bin/pnix-clr pnix-clr/examples/07-builtins-surface/sample.px
{"host":"pnix-clr","outcome_kind":"done",...,"value":{"path":42,"sum":10,"t":"int"}}
```
`sample.px`는 세 builtin을 한 attrset 결과로 묶어 보여준다.

## 어디에 쓰나
Nix 문법에 익숙한 사람이 pnix-clr가 어디까지 같은 builtin을 지원하는지
감을 잡을 때. 전체 목록은 `../README.md`의 "Language surface" 절 참고.

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e 'builtins.typeOf 1'
./bin/pnix-clr -e 'builtins.getAttrFromPath [ "foo" "bar" ] { foo.bar = 42; }'
./bin/pnix-clr -e 'lib.sum [1 2 3 4]'
./bin/pnix-clr pnix-clr/examples/07-builtins-surface/sample.px
```
