# 15 — 문자열 · 버전 helpers

## 쉽게 말하면 (비유)
`splitVersion "1.2.3"`은 Nix 패키징에서 흔한 "버전 문자열을 부품으로
쪼개기"다 — JS라면 `"1.2.3".split(".")`로 직접 짜야 할 걸 pnix 쪽
builtin이 이미 제공한다.

## 무엇을
`substring`, `concatStringsSep`, `splitVersion`, `toString`(정수/bool
양쪽), `hasAttr` 6가지 문자열/버전/introspection seed builtins.

## plain Node의 한계
`String.prototype.slice`, `Array.prototype.join`, `"1.2.3".split(".")`로
개별 기능은 다 흉내낼 수 있지만, pnix `toString`이 **불리언을 `"1"`/`""`**
같은 Nix 특유 규칙으로 바꾸는 것처럼, Nix 문자열 변환 규칙 자체를 JS가
미리 갖고 있지 않다 — 규칙을 pnix 쪽에서 그대로 가져다 쓰는 게 이 예제의
핵심이다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`, 실행 결과)
```
builtins.substring 1 2 "abcd"            => done bc
builtins.concatStringsSep "," ["a" "b"]  => done a,b
builtins.splitVersion "1.2.3"            => done [ '1', '2', '3' ]
builtins.toString 42                     => done 42
toString true                            => done 1
builtins.hasAttr "a" { a = 1; }          => done true
```
(`toString true`가 `"1"`이 아니라 표시상 `1`로 나오는 건 헬퍼가 원시값을
그대로 console.log한 결과 — Nix 규약대로 불리언 `true`→`"1"` 문자열 변환.)

## 어디에 쓰나
버전 문자열 파싱, 리스트를 사람이 읽는 문자열로 합치기, attrset에 특정
키가 있는지 조건부로 확인할 때.

## 실행
```bash
node pnix-cljs/examples/15-string-and-version/pnix_cljs_way.js
```
