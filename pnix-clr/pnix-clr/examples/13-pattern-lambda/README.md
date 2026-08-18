# 13 — 패턴 람다

## 쉽게 말하면 (비유)
`({ a }: a)`는 C# 레코드 패턴 매칭(`{ A: var a }`)과 감각은 비슷하다 —
attrset을 받는 함수가 필요한 키를 이름으로 뽑아 쓰고, 없으면 기본값을
쓰도록 선언한다.

## 무엇을
attrset 인자 패턴 2가지: 필수 키(`{ a }`), 기본값(`{ a ? 2 }`), 그리고
커링(`x: y: x + y`).

## plain .NET의 한계
C# 레코드/패턴 매칭은 **타입 선언이 먼저 있어야** 구조분해가 된다 — pnix
쪽 attrset 패턴은 타입 선언 없이 함수 시그니처 자체에 직접 쓰는 구조적
매칭이라는 게 다르다. 기본값(`{ a ? 2 }`)도 C# 선택적 매개변수와 감각은
비슷하지만, attrset 형태 자체에 붙는다는 점이 다르다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e '({ a }: a) { a = 7; }'
{"host":"pnix-clr","outcome_kind":"done",...,"value":7}

$ ./bin/pnix-clr -e '({ a ? 2 }: a) {}'
{"host":"pnix-clr","outcome_kind":"done",...,"value":2}

$ ./bin/pnix-clr pnix-clr/examples/13-pattern-lambda/sample.px
{"host":"pnix-clr","outcome_kind":"done",...,"value":{"curried":3,"def":2,"need":7}}
```

## 어디에 쓰나
함수형 설정 API(예: `{ name, version ? "0.1.0" }: ...`)를 pnix 소스로
표현할 때, 커링으로 부분 적용 함수를 만들 때.

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e '({ a }: a) { a = 7; }'
./bin/pnix-clr -e '({ a ? 2 }: a) {}'
./bin/pnix-clr pnix-clr/examples/13-pattern-lambda/sample.px
```
