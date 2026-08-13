# clr host-import example

## C# (Pnix.Clr.Eval) — host-main

```bash
cd ../../../pnix-clr
./bin/export-pnix-clr-library   # if needed
export PNIX_CLR_LIBRARY="$PWD/pnix-clr/target/pnix-clr-library"
export PNIX_CLR="$PWD/bin/pnix-clr"
dotnet run --project csharp/examples/HelloPnix -- --file ../examples/host-import/hello.px
```

MSBuild: `pnix-clr/csharp/Directory.Build.props.sample`.

## Multi-ns Clojure on bootstrap (not facade)

Two namespaces on disk via **upstream** substrate:

```bash
cd ../../../pnix-clr/examples/clojure-clr-project
./smoke
# => PASS (42)
```

See that README — uses `clojure-clr-bootstrap`, **not** `clojure-clr` (`-e`/one file only).

## pnix-main

```bash
pnix-clr ../hello.px
# or
pnix-clr -e '1 + 2'
```
