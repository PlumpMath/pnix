# 11 — 리스트 고차 builtins

## 쉽게 말하면 (비유)
`map`/`filter`는 .NET LINQ의 `.Select`/`.Where`와 같은 감각이다 — 다만
pnix 소스 안에서 표현식으로 직접 쓴다는 게 다르다.

## 무엇을
`map`, `filter`, `genList`(인덱스로 리스트 생성), `concatLists`(리스트의
리스트를 평탄화), `head` 5가지 리스트 seed builtins.

## plain .NET의 한계
LINQ `.Select`/`.Where`/`.SelectMany`로 비슷한 걸 만들 수 있지만, 그건
C# 코드다 — pnix 쪽 값(설정 파일 등)을 파싱한 결과에 바로 리스트 변환을
적용하려면, 그 변환 자체도 pnix 소스 안에서 표현할 수 있어야 한다는 게
이 예제의 요점.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e 'builtins.map (x: x + 1) [1 2 3]'
{"host":"pnix-clr","outcome_kind":"done",...,"value":[2,3,4]}

$ ./bin/pnix-clr pnix-clr/examples/11-list-higher-order/sample.px
{"host":"pnix-clr","outcome_kind":"done",...,
 "value":{"filtered":[2,3],"flat":[1,2,3],"gen":[0,2,4],"head":9,"mapped":[2,3,4]}}
```

## 어디에 쓰나
평가 결과 리스트를 변환/필터링해서 다음 단계로 넘길 때, C# 쪽 LINQ 대신
pnix 소스 안에서 직접 변환을 표현하고 싶을 때.

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e 'builtins.map (x: x + 1) [1 2 3]'
./bin/pnix-clr pnix-clr/examples/11-list-higher-order/sample.px
```
