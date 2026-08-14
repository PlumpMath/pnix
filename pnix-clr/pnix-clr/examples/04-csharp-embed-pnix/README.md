# 04 — C# embed pnix (host-main)

## 무엇을

C#이 드라이버, `.px`가 게스트. `Pnix.Clr.Eval` process-spawn API가 기본 지원
표면이다.

## 실행

```bash
cd pnix-clr
./bin/export-pnix-clr-library   # if needed
export PNIX_CLR_LIBRARY="$PWD/pnix-clr/target/pnix-clr-library"
export PNIX_CLR="$PWD/bin/pnix-clr"
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
  --file csharp/examples/hello.px
```

Or monorepo:

```bash
./examples/host-import/clr/smoke
```
