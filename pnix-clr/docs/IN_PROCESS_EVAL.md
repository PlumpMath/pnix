# In-process C# evaluator (experimental spike)

**Status:** experimental spike (2026-08-14) — **not** the product default.  
**Supported default:** `Pnix.Clr.Eval.Source` / `Eval.File` — **process-spawn** `pnix-clr`, JSON CLI contract.  
**Opt-in:** `Eval.SourceInProcess` / `FileInProcess` on **net10.0+** only.

Related: `csharp/Pnix.Clr/InProcessEval.cs` · monorepo `HOST_ENV_P2_P3.md` · `clr-meta/todo.md`.

---

## Why process-spawn is the product default

| Concern | Process-spawn (today) | In-process (goal) |
|---------|----------------------|-------------------|
| Isolation | Child process; crash ≠ host crash | Shared AppDomain / ALC |
| TFM mix | Host C# net8 can call net10 CLI | Must align TFMs / load contexts |
| Guest AOT | Optional Reference via props | Load `*.clj.dll` + ClojureCLR runtime |
| Deploy size | Needs `pnix-clr` on PATH/env | Bundle runtime + artifact |
| Determinism | CLI JSON schema already gated | Same schema + no silent drift |

Process-spawn stays the **default** even after in-process lands (opt-in).

---

## Goal

Allow C# host-main code to evaluate `.px` / inline source **without** spawning a process, returning the **same** `EvalResult` shape (`schema`, `outcome-kind`, `value`/`error`) as the CLI path.

Honest scope:

- **In:** pure eval of host-bound pnix on the clr limb (same semantics as `pnix-clr -e` / file).
- **Out:** full ClojureCLR REPL, arbitrary multi-ns Clojure projects, “replace process spawn for every deploy”.

---

## Embedding options (ordered by honesty cost)

### A. Managed host API over existing CLI protocol (thin)

Keep evaluation in a long-lived helper process / named pipe, not `Process.Start` per call.

- **Pros:** reuses JSON contract; weaker than true in-proc.
- **Cons:** still a process; not “in-process”.
- **Verdict:** intermediate; only if spawn cost is the pain, not embedding.

### B. Load guest AOT + ClojureCLR in an AssemblyLoadContext (preferred research path)

1. Ship/export already provides `runtime-artifact/*.clj.dll` + `Pnix.Clr` multi-TFM.
2. Host loads ClojureCLR substrate (net10 product path — see `TFM_POLICY.md`) in an **isolated** ALC.
3. Invoke the same entry the CLI uses for `-e` / file (or a dedicated managed entrypoint that emits CLI-shaped JSON / EDN).
4. Map to `EvalResult` without shelling out.

**Blockers to solve before code:**

1. **Substrate package** — which assemblies must be next to the host (Clojure.Main, deps, version pin 1.12.3-alpha8).
2. **ALC isolation** — unload, duplicate type identities, no leak into default context.
3. **TFM** — product guest AOT is net10; net8-only hosts may stay process-spawn only.
4. **Thread / apartment / statics** — ClojureCLR init once per ALC; document reentrancy.
5. **Parity gate** — byte-identical or structured-equal JSON for a fixed corpus vs process path.

### C. Pure managed reimplementation of pnix on CLR without ClojureCLR

Rewrite evaluator in C# / F#. **Rejected for now** — second semantic source of truth; violates host-bound product doctrine.

---

## Acceptance sketch (when owner pulls implementation)

1. Design note (this file) stays accurate.
2. Opt-in API, e.g. `Eval.SourceInProcess` / `EvalOptions.Execution = InProcess`, **default remains Process**.
3. Gate script: N fixtures where process and in-process results agree on `outcome-kind` + value JSON (or documented held diffs only).
4. Negative: missing substrate fails closed with actionable message (not hang, not silent null).
5. Docs: README table marks Process = supported, InProcess = experimental until gate green.
6. **No** Stage15/N or Trusting-Trust claims from embedding alone.

---

## Non-goals

- nuget.org requirement for first spike (local export layout is enough).
- Replacing `clojure-clr` facade or bootstrap multi-ns story.
- Loading arbitrary user Clojure projects in-process.

---

## Spike landed (2026-08-14)

| Piece | Location |
|-------|----------|
| Implementation | `csharp/Pnix.Clr/InProcessEval.cs` (net10 `#if`) |
| API | `Eval.SourceInProcess` / `FileInProcess` |
| Parity example | `csharp/examples/InProcessParity/` |
| Gate | `bin/pnix-clr-inprocess-eval-gate` (opt-in; **not** in `pnix-clr-gate` yet) |

### How it works

1. Resolve **substrate** (`PNIX_CLR_SUBSTRATE` or checkout `clojure-clr-…/net10.0/publish`) and **artifact** (`PNIX_CLR_ARTIFACT`).
2. Hook `AssemblyLoadContext.Default.Resolving` so guest AOT DLLs find `Clojure.dll`.
3. Preload substrate assemblies; `require` `pnix-clr.evaluator` / `main` / `json`.
4. Invoke `eval-source` (or `eval-file`) + `projection` + `write-json` via reflection — **no** `Environment.Exit` from `-main`.
5. Parse into the same `EvalResult` shape as the process path.

### Env contract

| Variable | Role |
|----------|------|
| `PNIX_CLR_ARTIFACT` | Guest AOT dir (`manifest.json` + `*.clj.dll`) |
| `PNIX_CLR_SUBSTRATE` | ClojureCLR net10 publish dir (`Clojure.dll`) |
| `PNIX_CLR_ROOT` | Host root (import confinement) |
| `PNIX_CLR` | Process path still used by parity comparison |

### Verified corpus (gate)

- `1 + 2` → 3  
- `true && !false` → true  
- `if true then 40 + 2 else 0` → 42  
- `1 / 0` → failed / division-by-zero (parity)  
- Missing substrate → `NotSupportedException` (fail closed)

### Still open before “admitted”

- [x] Broader parity corpus (14 source cases + file + 2 negatives) — gate 2026-08-14
- [ ] Collectible isolated ALC — **blocked for now**: ClojureCLR guest AOT
  initializes via `Assembly.Load` into the **default** context; a collectible
  ALC cannot see substrate types already loaded there without dual Resolving
  that collapses to Default. Documented tradeoff; revisit only with a
  substrate that supports ALC-aware load.
- [x] Wire into `pnix-clr-gate` when substrate+artifact present (`PNIX_CLR_INPROCESS_GATE=0` skips)
- [ ] net8 host story (keep process-spawn)
- [ ] Unload / multi-thread reentrancy policy
- [ ] No Stage15/N claims from embedding

### Run

```bash
export PNIX_CLR_ROOT=$PWD
export PNIX_CLR_ARTIFACT=$PWD/pnix-clr/target/runtime-artifact
export PNIX_CLR_SUBSTRATE=$PWD/clojure-clr-clojure-1.12.3-alpha8/Clojure/Clojure.Main/bin/Release/net10.0/publish
export PNIX_CLR=$PWD/bin/pnix-clr
./bin/pnix-clr-inprocess-eval-gate

# Or as part of the product aggregate (opt-in; default off):
PNIX_CLR_INPROCESS_GATE=1 ./bin/pnix-clr-gate

# HelloPnix demo (net10, same env):
dotnet run --project csharp/examples/HelloPnix -c Release -- --inprocess '1 + 2'
```