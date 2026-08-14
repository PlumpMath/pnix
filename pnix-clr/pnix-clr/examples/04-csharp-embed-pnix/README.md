# 04 — C# 에 pnix 임베드 (host-main)

## 무엇을

C# 이 드라이버, `.px` 가 게스트. `Pnix.Clr.Eval` process-spawn API 가 기본
지원 표면이다.

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
