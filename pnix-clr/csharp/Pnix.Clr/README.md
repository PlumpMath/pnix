# Pnix.Clr — C# host library for pnix-clr

Host-language import surface so **C#** projects can evaluate pnix (`.px`) and
wire **CLR guest AOT DLLs** without hand-copying cache paths.

## What this is / is not

| Surface | Role |
|---------|------|
| `Pnix.Clr.Eval` | Process-spawn `pnix-clr`, parse JSON CLI result |
| `lib/net10.0/runtime-artifact/*.clj.dll` | Guest AOT (ClojureCLR-bound) |
| `build/Pnix.Clr.props` + `.targets` (export layout; sources under `msbuild/`) | MSBuild HintPath / Reference wiring |

This is **not** a portable multi-host `.px` bytecode package. Artifacts are
**host-bound** to the CLR limb of pnix.

## Quick start (after `export-pnix-clr-library` or HM `pnix-clr-library`)

```csharp
using Pnix.Clr;

// Inline
var r = Eval.Source("1 + 2").EnsureDone();
Console.WriteLine(r.Value); // 3

// File import (.px)
var f = Eval.File("examples/hello.px").EnsureDone();
```

Environment (set by `dot-nix` / export):

- `PNIX_CLR` — path to `pnix-clr` executable  
- `PNIX_CLR_ROOT` — checkout or cache tree root  
- `PNIX_CLR_ARTIFACT` — runtime-artifact directory  
- `PNIX_CLR_LIBRARY` — exported library root (this layout)

## MSBuild

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <!-- optional: also Reference guest AOT DLLs -->
    <!-- <PnixClrImportGuestDlls>true</PnixClrImportGuestDlls> -->
  </PropertyGroup>
  <Import Project="$(PNIX_CLR_LIBRARY)/build/Pnix.Clr.props"
          Condition="'$(PNIX_CLR_LIBRARY)' != ''" />
  <Import Project="$(PNIX_CLR_LIBRARY)/build/Pnix.Clr.targets"
          Condition="'$(PNIX_CLR_LIBRARY)' != ''" />
</Project>
```

Or run `pnix-clr-refs` / `pnix-clr-library` to print absolute paths.

## Build / export (from pnix-clr checkout)

```bash
./bin/build-pnix-clr-artifact          # guest AOT if missing
./bin/export-pnix-clr-library          # → target/pnix-clr-library
# or: nix run .#pnix-clr-library
```

## Related CLIs

- `pnix-clr` / `pnix-clr-pnix` — eval / REPL  
- `clojure-clr` — focused `-e` / single-file facade over clr-meta  
- `pnix-clr-refs` — print artifact DLL paths (dot-nix helper)
