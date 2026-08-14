# 01 — pure eval boundary (.NET)

## 쉽게 말하면

C# `Eval` / 동적 컴파일로 임의 문자열을 돌리면 호스트 프로세스 전권이다.
pnix-clr는 **게스트 `.px`** 를 파서/평가기 경계로 넘긴다.

## plain .NET 한계 (`limit_dotnet.md`)

위험 패턴을 문서만으로 고정 (실행하지 않음).

## pnix-clr 방식

```bash
cd pnix-clr
./bin/pnix-clr -e '1 + 2 * 3'
./bin/pnix-clr -e '1 / 0'   # structured failure, not ambient host crash story
./bin/pnix-clr pnix-clr/examples/01-pure-eval-boundary/pure.px
```
