# pnix-rs

**rs-meta backed pnix runtime front-end** — the Rust host lane for the pnix
runtime path, sibling of `pnix-clj` (clj-meta backed) and `pnix-hy` (hy-meta
backed).

```text
pnix-rs    = Rust bootstrap/front-end for the pnix runtime path (this lane)
../rs-meta = Rust meta-circular stage15-N compiler/evaluator substrate (dependency)
runtime/   = repo-owned .px runtime artifacts
```

The dependency on rs-meta is falsifiable, not nominal: the px engine
(`src/px.rs`) is written inside the rs-meta evaluated Rust subset, and
`substrate-check` has the rs-meta bootstrap interpret that exact source, then
requires its output to match both the rustc-compiled run and this binary's own
native behavior on the same `.px` probes (3-way equality).

## Commands

```sh
export CARGO_TARGET_DIR=/tmp/pnix-rs-target
cargo build --release
P=/tmp/pnix-rs-target/release/pnix-rs

$P px-check          # seed .px corpus -> expected canonical output
$P substrate-check   # rs-meta interp == rs-meta rustc == pnix-rs native
$P px-eval -c 'let a = 1; b = a + 2; in a + b'
$P px-eval -f runtime/corpus/c05_recurse.px
```

## Seed .px surface

Integers, booleans, `+ - * /`, comparisons, `if/then/else`, lambdas
`param: body`, application by juxtaposition (`f x y`), **recursive** `let ... in`
(siblings and self-references resolve — pnix let semantics), attrset literals
`{ k = v; }` with sorted canonical printing, `#` comments. Everything outside
the seed (floats, strings, lists, `//` merge, selection, builtins) is refused
honestly and tracked in `todo.md`.

Corpus files under `runtime/corpus/` include two cases vendored from the
pnix-clj `rust_grounded` invariance corpus (`c05_recurse`, `c09_lambda`) for
later cross-host comparison, plus seed-owned cases (including the recursive-let
regression guard `seed_let_rec.px`).
