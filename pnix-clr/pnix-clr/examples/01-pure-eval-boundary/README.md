# 01 — 순수 평가 경계 (.NET)

## 쉽게 말하면 (비유)
C# `Eval`/동적 컴파일로 임의 문자열을 돌리면 그 코드는 호스트 프로세스
전권을 갖는다 — 파일, 네트워크, 프로세스에 다 손댈 수 있다. pnix-clr는
게스트 `.px`를 파서/평가기라는 명시적 경계 뒤로 넘긴다 — 게스트 표현식이
할 수 있는 일은 그 평가기가 정의한 범위로 제한된다.

## 무엇을
같은 산술식(`1 + 2 * 3`)과 실패식(`1 / 0`)을 CLI로 평가해, 성공은 값으로,
실패는 **구조화된 JSON**(호스트 프로세스 크래시가 아니라)으로 나오는지
확인한다.

## plain .NET의 한계 (`limit_dotnet.md`)
```
Microsoft.CodeAnalysis.CSharp.Scripting / Eval 은 제한 호스트를
직접 만들지 않으면 프로세스·파일시스템·네트워크에 닿을 수 있다.
BCL 에 "순수 Nix 유사 게스트" 는 없다.
```
신뢰할 수 없는 표현 언어를 돌리려면 명시적 게스트 평가기가 필요하다 —
호스트 스크립팅 API 자체를 샌드박스로 취급할 수 없다.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr -e '1 + 2 * 3'
{"host":"pnix-clr","outcome_kind":"done","schema":"pnix-clr.cli-result.v1","value":7}

$ ./bin/pnix-clr -e '1 / 0'
{"error":{"class":"division-by-zero","evidence":{"operator":"/"},"phase":"eval"},
 "host":"pnix-clr","outcome_kind":"failed","schema":"pnix-clr.cli-result.v1"}
```
`1 / 0`은 호스트 프로세스를 죽이지 않는다 — `outcome_kind: "failed"`와
구조화된 `error.class`로 게스트 실패가 값으로 관측된다.

## 어디에 쓰나
신뢰할 수 없는 설정/스크립트 언어를 .NET 프로세스 안에서 안전하게 돌려야
할 때(호스트 프로세스에 직접 `Eval`을 노출하지 않고).

## 실행
```bash
cd pnix-clr
./bin/pnix-clr -e '1 + 2 * 3'
./bin/pnix-clr -e '1 / 0'
./bin/pnix-clr pnix-clr/examples/01-pure-eval-boundary/pure.px
```
