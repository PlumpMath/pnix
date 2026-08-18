# 12 — with · assert · 병합 연산

## 쉽게 말하면 (비유)
`with { a = 1; }; a`는 attrset의 키를 스코프에 명시적으로 끌어오는 구조다.
`assert`는 조건이 거짓이면 그 자리에서 평가를 실패시키는 가드, `//`는 두
attrset을 오른쪽 우선으로 병합하는 연산자다.

## 무엇을
`with`(attrset 필드를 스코프로), `assert`(조건 가드), `++`(리스트 연결),
`//`(attrset 병합, 오른쪽 키 우선) 4가지.

## plain .NET의 한계
C# 오브젝트 이니셜라이저나 record `with` 식은 문법은 비슷해 보여도 의미가
다르다 — attrset 병합은 `{ ...a, ...b }` 같은 spread가 BCL에 없고, 리스트를
`//` 없이 조건부로 오른쪽 우선 병합하려면 매번 직접 짜야 한다. `assert`
표현식(조건이 거짓이면 평가 자체를 실패시키는)도 C#엔 없다 — `Debug.Assert`는
표현식이 아니라 문이고, 조건부로 릴리스 빌드에서 빠질 수 있다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e 'with { a = 1; }; a'
{"host":"pnix-clr","outcome_kind":"done",...,"value":1}

$ ./bin/pnix-clr -e '[1 2] ++ [3]'
{"host":"pnix-clr","outcome_kind":"done",...,"value":[1,2,3]}

$ ./bin/pnix-clr -e '{ a = 1; } // { b = 2; }'
{"host":"pnix-clr","outcome_kind":"done",...,"value":{"a":1,"b":2}}

$ ./bin/pnix-clr pnix-clr/examples/12-with-assert-merge/sample.px
{"host":"pnix-clr","outcome_kind":"done",...,
 "value":{"assertOk":1,"cat":[1,2,3],"merged":{"a":1,"b":2},"w":11}}
```

## 어디에 쓰나
설정 병합(`base // override`), 함수 진입 조건을 강제하고 싶을 때
(`assert`).

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e 'with { a = 1; }; a'
./bin/pnix-clr -e '[1 2] ++ [3]'
./bin/pnix-clr -e '{ a = 1; } // { b = 2; }'
./bin/pnix-clr pnix-clr/examples/12-with-assert-merge/sample.px
```
