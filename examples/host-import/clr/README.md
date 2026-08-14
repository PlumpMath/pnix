# clr host-import example

## C# (Pnix.Clr.Eval) — host-main smoke

```bash
# From monorepo root (or this directory):
./smoke
# => 3
# => OK  clr host-import HelloPnix eval_file => 3
```

Manual (same idea as the smoke):

```bash
cd ../../../pnix-clr
./bin/export-pnix-clr-library   # if needed
export PNIX_CLR_LIBRARY="$PWD/pnix-clr/target/pnix-clr-library"
export PNIX_CLR="$PWD/bin/pnix-clr"
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
  --file ../examples/host-import/hello.px
```

MSBuild: `pnix-clr/csharp/Directory.Build.props.sample`.

Optional experimental in-process (net10, needs substrate + artifact):

```bash
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- \
  --inprocess --file ../examples/host-import/hello.px
```

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
