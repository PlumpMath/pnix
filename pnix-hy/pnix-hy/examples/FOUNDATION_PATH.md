# pnix-hy foundation path

Use this order before the extended proof/research catalog.

| Step | Concern | Example | Owner |
|---|---|---|---|
| 1 | Basic PNIX evaluation | `00-foundation/basic.py` | `pnix-hy` execution |
| 2 | Python/PNIX values | `00-foundation/interop.py` | `pnix_hy` interop |
| 3 | Meta-circular execution | `00-foundation/meta_circular.py` | explicit `pnix_hy.meta` facade |

`import pnix_hy` loads the basic runtime. `pnix_hy.load_meta_api()` loads the
basic meta-circular compiler/evaluator surface. Proof, action, deployment, and
admission APIs are not imported by either operation; they must be requested
through their explicit verification surface.

## Type rule

Python strings are data, never PNIX type witnesses. The protocol boundary uses
closed structural ADT nodes from `pnix.boundary-type.v1`. A label such as
`"ProbeInput"` may name a table entry, but cannot substitute for its validated
record/variant/result graph.

## Extended catalog

| Role | Existing examples |
|---|---|
| basic evaluation and diagnostics | `01`, `10`, `13`, `28` |
| Python/Hy/PNIX interop | `04`, `07`, `08`, `14`, `15` |
| meta-circular execution | `03`, `11`, `19`, `20`, `24`, `33`, `35` |
| state and isolation mechanisms | `12`, `22`, `23`, `30`, `31` |
| independent proof/research | `02`, `05`, `09`, `16`-`18`, `25`-`27`, `29`, `32`, `34` |

Agreement and proof remain important merge evidence, but they do not own basic
language outcomes.
