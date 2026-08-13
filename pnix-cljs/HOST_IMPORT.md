# Host-language import — pnix-cljs (Node / CLJS)

**Canonical dual-axis doctrine:** [`../HOST_DEV_ENV.md`](../HOST_DEV_ENV.md)

Product package ships a **host-bound** JS library (not portable multi-host `.px`).

---

## Layout (after `nix build .#pnix-cljs` / HM install)

```text
$out/share/pnix-cljs/
  package.json              # name @plumpmath/pnix-cljs, main: pnix-cljs-module.js
  pnix-cljs-module.js       # require target (eval API)
  pnix-cljs.js              # CLI entry (wrapped as bin/pnix-cljs)

$out/lib/node_modules/@plumpmath/pnix-cljs/   # same files (scoped require)
```

Env (HM `node` / `pnix-cljs-node` / shadow wrapper):

| Variable | Meaning |
|----------|---------|
| `PNIX_CLJS_SHARE` / `PNIX_CLJS_LIBRARY` | `$out/share/pnix-cljs` |
| `PNIX_CLJS` | path to `pnix-cljs` CLI |
| `NODE_PATH` | `$out/lib/node_modules:$out/share/pnix-cljs:…` |

---

## Require + eval API

```js
// Preferred (scoped package — needs lib/node_modules on NODE_PATH)
const pnix = require('@plumpmath/pnix-cljs');

// Flat fallback (share/ on NODE_PATH alone is enough)
// const pnix = require('pnix-cljs-module.js');

// Inline
pnix.evalSource('1 + 2');           // JS projection object
pnix.evalSourceJson('1 + 2');       // JSON string
pnix.evalValueJson('1 + 2');        // value-only JSON (e.g. "3")

// File (.px)
pnix.evalFile('prog.px');
pnix.evalFileJson('prog.px');
pnix.evalFileValueJson('prog.px');  // often what you want for smoke: "3"
pnix.evalFileValue('prog.px');
```

### Smoke (HM profile with `pnix-cljs-host`)

```bash
echo '1 + 2' > /tmp/t.px
node -e "const p=require('@plumpmath/pnix-cljs'); console.log(p.evalFileValueJson('/tmp/t.px'))"
# => 3   (after flake install that ships lib/node_modules)

# Always works with current share/ on NODE_PATH:
node -e "const p=require('pnix-cljs-module.js'); console.log(p.evalFileValueJson('/tmp/t.px'))"

pnix-cljs-library   # print env + paths
clojurescript -e '20 + 22'   # → pnix-cljs CLI
pnix-cljs-pnix               # pnix-main REPL
```

---

## Naming

| Name | Role |
|------|------|
| `pnix-cljs` | runtime CLI (eval / `--repl`) |
| `pnix-cljs-pnix` | pnix-main interactive REPL |
| `clojurescript` | bare host-main alias → `pnix-cljs` |
| `pnix-cljs-cljs` / `cljs-meta` | host-meta fixed-point surface |
| `shadow-cljs` | **build orchestrator** only; injects `PNIX_CLJS` / `NODE_PATH` |

---

## Not claimed

- Portable multi-host `.px` package  
- Full replacement of the shadow-cljs build graph  
- npm registry publish (optional later; use nix share / NODE_PATH today)
