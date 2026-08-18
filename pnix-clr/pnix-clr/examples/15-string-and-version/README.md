# 15 — 문자열 · 버전 helpers

## 쉽게 말하면 (비유)
`splitVersion "1.2.3"`은 Nix 패키징에서 흔한 "버전 문자열을 부품으로
쪼개기"다 — C#이라면 `"1.2.3".Split('.')`로 직접 짜야 할 걸 pnix 쪽
builtin이 이미 제공한다.

## 무엇을
`substring`, `concatStringsSep`, `splitVersion`, `toString`, `hasAttr`
5가지 문자열/버전/introspection seed builtins.

## plain .NET의 한계
`string.Substring`, `string.Join`, `"1.2.3".Split('.')`로 개별 기능은 다
흉내낼 수 있지만, pnix 문자열 모델은 CLR `System.String`/char-index
연산이지 Nix의 UTF-8 바이트 오프셋이나 string-context 추적이 아니다(제품
`README.md`의 "Known gaps" 참고) — Nix 문자열 변환 규칙 자체를 pnix 쪽에서
그대로 가져다 쓰는 게 이 예제의 핵심.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e 'builtins.substring 1 2 "abcd"'
{"host":"pnix-clr","outcome_kind":"done",...,"value":"bc"}

$ ./bin/pnix-clr -e 'builtins.concatStringsSep "," ["a" "b"]'
{"host":"pnix-clr","outcome_kind":"done",...,"value":"a,b"}

$ ./bin/pnix-clr -e 'builtins.splitVersion "1.2.3"'
{"host":"pnix-clr","outcome_kind":"done",...,"value":["1","2","3"]}

$ ./bin/pnix-clr pnix-clr/examples/15-string-and-version/sample.px
{"host":"pnix-clr","outcome_kind":"done",...,
 "value":{"has":true,"join":"a,b","sub":"bc","ts":"42","ver":["1","2","3"]}}
```

## 어디에 쓰나
버전 문자열 파싱, 리스트를 사람이 읽는 문자열로 합치기, attrset에 특정
키가 있는지 조건부로 확인할 때.

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e 'builtins.substring 1 2 "abcd"'
./bin/pnix-clr -e 'builtins.concatStringsSep "," ["a" "b"]'
./bin/pnix-clr -e 'builtins.splitVersion "1.2.3"'
./bin/pnix-clr pnix-clr/examples/15-string-and-version/sample.px
```
