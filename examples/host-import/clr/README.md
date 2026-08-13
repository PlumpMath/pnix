# clr host-import example

C# sample already lives in the product tree:

```bash
cd ../../../pnix-clr
./bin/export-pnix-clr-library   # if needed
export PNIX_CLR_LIBRARY="$PWD/pnix-clr/target/pnix-clr-library"
export PNIX_CLR="$PWD/bin/pnix-clr"
dotnet run --project csharp/examples/HelloPnix -- --file ../examples/host-import/hello.px
# or from this dir after export:
#   use csharp/Directory.Build.props.sample
```

MSBuild wiring: `pnix-clr/csharp/Directory.Build.props.sample`.

pnix-main:

```bash
pnix-clr ../hello.px
# or
pnix-clr -e '1 + 2'
```
