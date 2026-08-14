# clr host-import 예제

## C# (Pnix.Clr.Eval) — host-main 스모크

```bash
# 모노레포 루트 또는 이 디렉터리에서:
./smoke
# => 3
# => OK  clr host-import HelloPnix eval_file => 3
```

수동 (스모크와 같은 뜻):

```bash
cd ../../../pnix-clr
./bin/export-pnix-clr-library   # 필요 시
export PNIX_CLR_LIBRARY="$PWD/pnix-clr/target/pnix-clr-library"
export PNIX_CLR="$PWD/bin/pnix-clr"
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
  --file ../examples/host-import/hello.px
```

MSBuild: `pnix-clr/csharp/Directory.Build.props.sample`.

선택적 실험 in-process (net10, substrate + artifact 필요):

```bash
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
  --inprocess --file ../examples/host-import/hello.px
```

## 다중 ns Clojure on bootstrap (파사드 아님)

디스크 위 2개 네임스페이스 — **업스트림** substrate:

```bash
cd ../../../pnix-clr/examples/clojure-clr-project
./smoke
# => PASS (42)
```

그 README 참고 — **clojure-clr-bootstrap** 사용 (`clojure-clr` 의 `-e`/단일 파일 아님).

## pnix-main

```bash
pnix-clr ../hello.px
# 또는
pnix-clr -e '1 + 2'
```
