# Host-language import — pnix-clj (JVM)

**Canonical dual-axis doctrine:** [`../../../HOST_DEV_ENV.md`](../../../HOST_DEV_ENV.md)

This document is the **public API surface** for caller projects that load
`pnix-clj` as a host library after `clojure` / `pnix-clj-clj` injects:

```clojure
{:deps {pnix/pnix-clj {:local/root "…/pnix-clj"}}}
```

Env (HM / wrappers): `PNIX_CLJ_ROOT`, `PNIX_CLJ_LIBRARY` (same tree root).

---

## Supported (stable for host-main)

Prefer these for application code. Other namespaces exist for tower/gates and
may change without a deprecation cycle.

| Namespace | Entry points | Role |
|-----------|--------------|------|
| **`pnix-clj.core`** | `parse-source`, `eval-source`, **`eval-file`**, `eval-source-with-imports`, `eval-source-strict`, `eval-source-strict-audit`, `lower-source` | Parse / evaluate `.px` (primary surface) |
| **`pnix-clj.machine-outcome`** | `eval-source-outcome` | Structured Done/Failed/Suspended projection |
| **`pnix-clj.convenience`** | helpers used by examples | Thin sugar over core (prefer core in new code) |

### Minimal examples

```clojure
(require '[pnix-clj.core :as c])

;; Inline source
(c/eval-source "1 + 2")
;; => {:status :ok, :value 3, …}

;; File (host-language import of a .px program)
(c/eval-file "path/to/prog.px")

;; In-memory imports only (no FS): map of target-string -> source
(c/eval-source-with-imports "import ./lib.px" {"lib.px" "1 + 1"})
```

Result shape: runtime map with `:status` (`:ok` / `:failed` / `:suspended` …),
`:value` or `:error`, and parse metadata. Treat non-`:ok` as failure.

---

## Available but secondary

Use only if you already know the tower/mirror lanes:

| Namespace | Notes |
|-----------|--------|
| `pnix-clj.mirror`, `pnix-clj.mirror-pair` | Cross-substrate mirror reports |
| `pnix-clj.interop` | Host interop / opaque refs |
| `pnix-clj.capabilities` | Capability index |
| `pnix-clj.lowering` | Lowering lane |
| `pnix-clj.parser` / `pnix-clj.evaluator` | Internals — prefer `core` |

Proof / generator / fuzzer namespaces (`generate`, `grammar-fuzzer`,
`arith-proof`, …) are **not** host-library API for applications.

---

## What this is not

- Not a portable multi-host `.px` bytecode package.
- Not a substitute for stock `clojure` without the inject wrapper when you need
  jars / `libexec` — use `clojure-stock` in nix for those.
- Optional later: Maven coordinate so projects need not `local/root`.

---

## Smoke

```bash
# With HM clojure (= pnix-clj-clj)
echo '1 + 2' > /tmp/t.px
clojure -M -e "(require '[pnix-clj.core :as c]) (println (:value (c/eval-file \"/tmp/t.px\")))"
# => 3

pnix-clj-library   # print PNIX_CLJ_ROOT / local/root hint
pnix-clj-pnix      # pnix-main REPL
```
