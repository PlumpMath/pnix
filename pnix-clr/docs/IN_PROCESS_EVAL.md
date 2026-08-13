# In-process C# evaluator (design — not admitted)

**Status:** design only (2026-08-14).  
**Current supported surface:** `Pnix.Clr.Eval.Source` / `Eval.File` — **process-spawn** `pnix-clr`, JSON CLI contract.  
**Do not claim** in-process eval closed until the acceptance gate below is green.

Related: `csharp/Pnix.Clr/Eval.cs` · monorepo `HOST_ENV_P2_P3.md` · `clr-meta/todo.md` § Host-import hard.

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

## Suggested first implementation slice (later)

1. Spike: ALC load of published Clojure.Main + one `pnix-clr` AOT DLL; call a tiny known entry; capture stdout/JSON.
2. Map to `EvalResult`; compare to `Eval.Source` on `"1 + 2"`.
3. Named gate `bin/pnix-clr-inprocess-eval-gate` (opt-in; not in default aggregate until green for weeks).
4. Expand corpus only with the gate green.

Until then, callers must use process-spawn `Eval.Source` / `Eval.File`.
