# pnix-clj foundation path

Use this order before the historical verification/research catalog.

| Step | Concern | Example | Owner |
|---|---|---|---|
| 1 | PNIX values and basic evaluation | `00-foundation/basic.clj` | `pnix-clj` execution |
| 2 | PNIX values crossing into Clojure | `00-foundation/interop.clj` | `pnix-clj.interop` |
| 3 | Meta-circular host execution | `00-foundation/meta_circular.clj` | `clj-meta` mechanism, no proof prerequisite |

## Basic means

```text
PNIX source -> parse/evaluate or lower/execute -> language outcome
```

Meta-circular compilation/evaluation is also basic. A compile receipt, mirror,
repeat-compilation witness, deployment decision, or owner approval can verify
an implementation, but cannot be required to obtain a basic result.

## Types are structures, not names

`"I64"`, `"ProbeInput"`, and `"~type"` are text values. They never grant type
authority. Host interop receives a complete validated `pnix.boundary-type.v1`
node, including every record field, variant case, and child type.

## Extended catalog

After this path, use the numbered examples by role:

| Role | Existing examples |
|---|---|
| lazy evaluation and machine behavior | `01`, `61`, `78`, `79`, `81`, `90` |
| Clojure/PNIX interop | `04`, `75`, `80` |
| meta-circular execution | `11`, `19`, `33`, `35`, `70`, `71` |
| imports and modules | `51`, `89` |
| structured failures | `40`, `63`, `76`, `77`, `82` |
| independent proof/research | `05`, `16`, `20`-`30`, `34`, `46`-`49` |

Proof/research examples are not children of the basic runtime. They are sibling
verification lanes.
