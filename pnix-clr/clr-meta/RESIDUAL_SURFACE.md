# Residual surface (principle map)

This note states **what remains**, in terms of language/runtime principles —
not library version tags, not case-count bragging, and not promotion claims.

`promotion/allowed?` stays **false**. Closing a residual here means live
five-host agreement on a pinned corpus observation, nothing more.

## What the common slice now owns (principles)

| Principle | Meaning on this host |
|-----------|----------------------|
| URI as string | Deprecated URI lexer form evaluates as a plain string value |
| JSON round-trip | `fromJSON` / `toJSON` over guest values; integers stay exact; finite floats keep a decimal form |
| Attribute names | Identifiers, quoted strings (incl. empty), keyword-shaped names (`true`/`false`/`null`), and **dynamic** keys from string interpolation |
| Exact integers | Pure int arithmetic and compare stay on signed 64-bit cells (no silent float collapse past the mantissa boundary) |
| Mixed numeric ops | Int with float promotes; signed zero is preserved; ceil/floor refuse lossy int→float seams |
| Non-finite observation | Inf/NaN string forms; NaN is never scalar-equal; **shared value cells** may be equal inside lists/attrsets |
| POSIX ERE classes | `[[:name:]]` inside a bracket is ASCII (C locale), not Unicode properties; unknown names fail closed |
| Failed thunk replay | A catchable throw stored in a thunk replays on every force; blackhole is only the in-progress state |
| Kernel / math guest modules | Portable `.px` parser, numbers, evaluator, and math surfaces that run as ordinary guest programs |

## Still open (principle gaps)

| Principle | Why it is still residual |
|-----------|---------------------------|
| Module compile graph | `compile-module` overflows the host stack under mutual recursion / deep force |
| Derivation host ABI | Source uses a surface form this parser still rejects (interp / binding shape) |
| Term-DAG guest payload | Guest path feeds `fromJSON` a non-JSON fragment (`?…`); host JSON reader correctly fails closed |
| Full self-host fixed point | Meta-circular bootstrap of a **subset** is live; full self-hosting and IL fixed-point are not claimed |
| Compiler stages beyond the product floor | Stage chain past the closed self-host recompile floor, Trusting-Trust, and host promotion remain open |

## How to grow the slice

1. Name the **principle** (not a version string).
2. Implement the minimum host surface that makes the principle observable.
3. Prove it with the five-host common-slice gate against pinned `expected.json`.
4. Keep `promotion/allowed? = false` until an explicit promotion receipt exists.

Do not renumber “library levels.” Prefer sentences like “dynamic attribute
keys” or “shared NaN identity” over “slice N” or “lib-foo v3.”
