# 04 — C#에 pnix 임베드 (host-main)

## 쉽게 말하면 (비유)
`03-outcome-projection`까지는 pnix-clr가 **드라이버**(CLI)였다. 이 예제는
반대로 **C#이 드라이버**가 되고 `.px`가 게스트 콘텐츠가 된다 — 마치 C#
앱이 설정 파일을 읽듯, `Pnix.Clr.Eval` API로 `.px` 파일을 읽어 값을
받는다.

## 무엇을
C#이 드라이버, `.px`가 게스트. `Pnix.Clr.Eval` process-spawn API가 기본
지원 표면이다. `csharp/examples/HelloPnix`가 `csharp/examples/hello.px`를
읽어 평가한다.

## plain .NET의 한계
.NET에는 "임의의 신뢰 못 할 표현 언어 파일을 읽어 안전하게 평가"하는
표준 API가 없다 — `Pnix.Clr.Eval`이 그 자리를 process-spawn 경계(별도
프로세스에서 게스트를 돌리고 결과만 받는)로 채운다.

## pnix-clr의 방식 (실행 결과)
```
$ cat csharp/examples/hello.px
1 + 2

$ dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
    --file csharp/examples/hello.px
3
```

## 어디에 쓰나
C# 애플리케이션에서 사용자 정의 설정/규칙 파일을 pnix 문법으로 읽고 싶을
때 — 신뢰 못 할 콘텐츠를 앱 프로세스 안에서 직접 평가하지 않고 격리된
process-spawn 경계 뒤로 넘긴다.

## 실행
```bash
cd pnix-clr
./bin/export-pnix-clr-library   # 필요 시
export PNIX_CLR_LIBRARY="$PWD/pnix-clr/target/pnix-clr-library"
export PNIX_CLR="$PWD/bin/pnix-clr"
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
  --file csharp/examples/hello.px
```

모노레포:

```bash
./examples/host-import/clr/smoke
```
