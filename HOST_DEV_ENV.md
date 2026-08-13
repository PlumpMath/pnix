# Host development environment — dual axis (canonical)

**Audience:** humans, Claude/Codex sessions, and anyone wiring `~/dot-nix` or
a host flake. Read this before inventing a third naming scheme.

**Last updated:** 2026-08-14 (import smoke + packaging tiers)  
**HM mirror:** `~/dot-nix/dev/PNIX-HOSTS.md` (PATH packages, ShellCheck rules)

---

## Doctrine (do not conflate)

Each of the five hosts is **self-contained**. There is **no** portable multi-host
`.px` bytecode package today. A “library” built by a host is a **host-language
library** for *that* host only.

| Pattern | Orientation | Meaning |
|---------|-------------|---------|
| `pnix-<host>-pnix` | **pnix-main** | Evaluate / REPL **pnix** (`.px`) on this host |
| `pnix-<host>-<lang>` | **host-main** | Day-to-day **host language** toolchain that **loads** this host’s pnix product library |
| `pnix-<host>-<host>` | **host-main** (short form) | Same as above when lang name equals host id (`-clj`, `-cljs`, `-hy`, `-rs`, `-clr`) |
| `pnix-<host>-library` / `*-refs` | either | Materialize or print the product library + env contract |
| `<host>-meta` | mechanism | Host-language self-host / stages; **pnix-agnostic** |

**Both orientations must exist** for a complete host ecosystem setup:

1. **Host-main** — developer lives in Clojure / Node / Python / Rust / C#; tools
   start with env so `require` / `import` / link / MSBuild can see the pnix product.
2. **Pnix-main** — developer lives in `.px`; `pnix-<host>-pnix` (and eval CLIs)
   work; host tools stay available on PATH; library env remains set for reverse
   interop.

Historical **pnix-meta** “one portable `.px` core for all hosts” is a **later**
track. Do not claim it closed from PATH wrappers or host libraries.

---

## Matrix (product + HM names)

| Host | Host-main entry | Product library | Env contract (product / HM) | Pnix-main | Meta |
|------|-----------------|-----------------|-----------------------------|-----------|------|
| **clj** | `pnix-clj-clj` (bare `clojure`) | `pnix-clj` sources on classpath (`-Sdeps` local/root) | `PNIX_CLJ_ROOT`, `PNIX_CLJ_LIBRARY` | `pnix-clj-pnix` | `clj-meta` |
| **cljs** | `clojurescript` → `pnix-cljs`; `node` via host join | `share/pnix-cljs` JS module | `NODE_PATH`, `PNIX_CLJS_SHARE`, `PNIX_CLJS_LIBRARY`, `PNIX_CLJS` | `pnix-cljs-pnix` | `cljs-meta` / `pnix-cljs-cljs` |
| **hy** | `pnix-hy-python` / `pnix-hy-hy` (bare `python`/`hy`) | installable `pnix_hy` package | `PYTHONPATH`, `PNIX_HY_HOME`, `PNIX_HY_LIBRARY`, `PNIX_HY_PYTHON` | `pnix-hy-pnix` | `hy-meta` |
| **rs** | `pnix-rs-rs` (bare `cargo`/`rustc`) | `pnix-rs-library` (`libpnix_rs.*` + `pnix_rs.h`) | `PNIX_RS_LIB_DIR`, `PNIX_RS_INCLUDE_DIR`, `PNIX_RS_RUNTIME` | `pnix-rs-pnix` / `px-eval` | `rs-meta` |
| **clr** | `pnix-clr-clr` / `clojure-clr`; C# `pnix-clr-cs` | `export-pnix-clr-library` → `Pnix.Clr` + guest AOT DLLs + MSBuild props | `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT`, `PNIX_CLR_LIBRARY` | `pnix-clr-pnix` | `clr-meta` |

Flake apps (inside each host directory):

```text
nix run .#pnix-<host>          # runtime CLI
nix run .#pnix-<host>-pnix     # pnix-main REPL
nix run .#gate
nix run .#pnix-rs-library      # rs only: embeddable rlib/dylib package
nix run .#pnix-clr-library     # clr only: export Pnix.Clr + guest AOT
```

**Not every host has `.#pnix-<host>-library`.** See [HOST_IMPORT.md](HOST_IMPORT.md)
§ packaging tiers. clj/hy/cljs expose the library as the main product package
(plus HM `pnix-*-library` path printers).

Import cookbooks: **[HOST_IMPORT.md](HOST_IMPORT.md)**.

HM reimplements shell runners with **`writeShellScriptBin`** (never raw
`writeShellApplication` → ShellCheck/GHC on x86_64-darwin). See
`~/dot-nix/dev/PNIX-HOSTS.md`.

---

## Host-language import of `.px` (API cheat sheet)

Libraries are **host-bound**. Prefer these entry points:

| Host | From host language |
|------|--------------------|
| clj | `(pnix-clj.core/eval-file "x.px")` — public API: [docs/HOST_IMPORT.md](pnix-clj/pnix-clj/docs/HOST_IMPORT.md) |
| cljs | `require('@plumpmath/pnix-cljs')` → `evalFile` / `evalSource` ([HOST_IMPORT.md](pnix-cljs/HOST_IMPORT.md)) |
| hy | `import pnix_hy as ph; ph.eval_file("x.px")` (= `run_px`) |
| rs | `pnix_rs::eval_file("x.px")` / C ABI `pnix_rs_eval` |
| clr | `Pnix.Clr.Eval.File("x.px")` or `pnix-clr x.px` (JSON CLI result) |

### CLR library layout (product)

```bash
cd pnix-clr
./bin/export-pnix-clr-library   # → pnix-clr/target/pnix-clr-library/
# lib/net{8,10}.0/Pnix.Clr.dll
# lib/net10.0/runtime-artifact/pnix_clr.*.clj.dll + manifest.json
# build/Pnix.Clr.props|.targets
# share/pnix-clr/refs.env
```

C#:

```csharp
using Pnix.Clr;
var r = Eval.File("hello.px").EnsureDone();
```

Guest AOT `*.clj.dll` are **ClojureCLR assemblies**, not a general C# API.
Enable `PnixClrImportGuestDlls` only for CLR hosts that load them via
`CLOJURE_LOAD_PATH` / assembly load — not casual net8 app code.

Sources: `pnix-clr/csharp/Pnix.Clr/`, flake apps `pnix-clr-library`, `pnix-clr-refs`.

### RS library layout (product)

```text
packages.pnix-rs-library → $out/lib/libpnix_rs.* + $out/include/pnix_rs.h
```

### CLJS library layout (product)

```text
packages.pnix-cljs →
  $out/share/pnix-cljs/                         # flat: pnix-cljs-module.js
  $out/lib/node_modules/@plumpmath/pnix-cljs/   # scoped require (preferred)
NODE_PATH must include lib/node_modules and/or share/
# require('@plumpmath/pnix-cljs')  or  require('pnix-cljs-module.js')
```

Detail: [pnix-cljs/HOST_IMPORT.md](pnix-cljs/HOST_IMPORT.md).

### HY library layout (product)

```text
packages.pnix-hy → site-packages/pnix_hy  (PYTHONPATH)
```

### CLJ library layout (product)

```text
pnix-clj/ sources via -Sdeps {:deps {pnix/pnix-clj {:local/root …}}}
```

---

## Ecosystem checklist (must all exist per host)

For a host to be “set up” for developers:

1. **Host-main** wrappers that inject the product library  
2. **`pnix-<host>-library` and/or `*-refs`** (or equivalent flake package)  
3. **`pnix-<host>-pnix`** (pnix-main)  
4. Host-language REPL/toolchain variants where the flake defines them  
5. **`<host>-meta`**  
6. **Gate** when the host has one  

dot-nix implements (1)–(6) under `dev/{clj,cljs,py,rs,cs}/`. Product work that
cannot be done with PATH alone lives in the host tree (this monorepo).

---

## What agents must not do

- Claim **Stage15/N**, common-compiler, or five-host gate unless the host’s own
  gate/docs say so (esp. clr / cljs).
- Treat host `*.dll` / rlib / JS share as a **common** `.px` package.
- Pull `writeShellApplication` into home-manager on darwin (ShellCheck/GHC).
- Globally override `pkgs.python311` / full `python3Packages` to inject Hy
  (breaks nixpkgs builders). Use PATH joins only.
- Mix full `pnix-rs` + `pnix-rs-library` in one `buildEnv` (dylib clash).
- Confuse Rhino **sdk_8** with pnix-clr **sdk_10**.

---

## Per-host deep docs

| Host | Start here |
|------|------------|
| clj | `pnix-clj/CLAUDE.md`, `pnix-clj/README.md`, `pnix-clj/pnix-clj/todo.md` (host import) |
| cljs | `pnix-cljs/CLAUDE.md`, `pnix-cljs/README.md`, `pnix-cljs/cljs-meta/todo.md` |
| hy | `pnix-hy/CLAUDE.md`, `pnix-hy/README.md`, `pnix-hy/pnix-hy/todo.md` |
| rs | `pnix-rs/CLAUDE.md`, `pnix-rs/README.md`, `pnix-rs/pnix-rs/todo.md` |
| clr | `pnix-clr/CLAUDE.md`, `pnix-clr/README.md`, `pnix-clr/csharp/Pnix.Clr/README.md`, `pnix-clr/clr-meta/todo.md` |

HM packaging truth: `~/dot-nix/dev/PNIX-HOSTS.md`.

---

## Hy name clash (important)

| Name | Meaning |
|------|---------|
| flake `.#pnix-hy-hy` | `pnix-hy --repl hy` (source tree / proof Python) |
| HM bin `pnix-hy-hy` | bare **Hy interpreter** with `PYTHONPATH` for `pnix_hy` |

They share a name but are **not** the same program. Prefer docs that say
“bare `hy` via `pnix-hy-host`” vs “flake app `pnix-hy-hy` REPL mode”.

## Smoke (orientation)

```bash
# host-main library inject
clojure -e '(+ 1 2)'                    # clj
python -c 'import pnix_hy'              # hy (with PYTHONPATH)
# cargo/rustc have PNIX_RS_* when pnix-rs-rs is on PATH
# node has NODE_PATH when pnix-cljs-host is on PATH

# pnix-main
# nix run .#pnix-clj-pnix   (from pnix-clj/)
# nix run .#pnix-hy-pnix
# nix run .#pnix-rs-pnix
# nix run .#pnix-cljs-pnix
# nix run .#pnix-clr-pnix

# clr library
cd pnix-clr && ./bin/export-pnix-clr-library && cat pnix-clr/target/pnix-clr-library/share/pnix-clr/refs.env
```
