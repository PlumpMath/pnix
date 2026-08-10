# rs-meta

**rs-meta is a Rust meta-circular compiler/evaluator** — written in Rust, it
evaluates Rust — built as a staged self-bootstrap toward stage15-N. Standalone:
it is not tied to any other language or project.

It evaluates Rust two ways and keeps them equal:

- **in-Rust interpreter** (`src/interp.rs`) — a tree-walking evaluator for a Rust
  subset; the meta-circular oracle.
- **native tier** (`src/native.rs`) — the *same Rust source* handed to `rustc`
  and run. This is the Evcxr mechanism (Rust evaluated via rustc) done in-house.

**Translation validation** requires interpreter stdout == rustc stdout for every
program. The long-term goal is to grow the evaluated Rust subset until the
evaluator's own source falls inside it, so the evaluator can evaluate itself
(true meta-circularity), then push the rustc-style stage chain to stage15-N.

## Status / primary gate

See [STATUS.md](STATUS.md). Primary gate: `./bin/rs-meta-gate` (self-check + tv-check).

## Constitution

1. **Zero crates.io dependencies.** `std` only. `rustc` is invoked as a
   *toolchain* via `std::process`, never depended on as a library. `evcxr-0.21.1`
   is studied for technique, not linked.
2. **Honesty.** `DONE` = runnable and checked, `TODO` = next slice, `HELD` = not
   yet claimed. No overclaiming (a B==C fixed point is reproducibility evidence,
   not a Trusting-Trust defense).
3. **Translation validation stays green.** Every execution path agrees on every
   program; forms the native tier can't handle are refused, never faked.

## Substrate snapshots

- `../rust-1.96.0` — the rustc source snapshot (trusted toolchain).
- `../evcxr-0.21.1` — the Rust-evaluates-Rust reference for the native tier.

## Commands

```sh
export CARGO_TARGET_DIR=/tmp/rs-meta-target
cargo build
BIN=/tmp/rs-meta-target/debug/bootstrap

$BIN self-check       # interpreter runs the corpus; output matches expected
$BIN tv-check         # interpreter stdout == rustc stdout
$BIN typeck-check     # interp rejects iff rustc rejects
$BIN roundtrip-check  # parse -> emit -> reparse AST identity + interp(emit) parity
$BIN emit-tv-check    # rustc(emit(parse(src))) == expected (310/310)
$BIN emit-self-host-check # emitted all-source bundle rustc == original + corpus replay
$BIN witness-check    # facet witness table determinism + proof/witness-report.tsv
$BIN witness -c 'fn main() { }'   # facet witness records for one program
$BIN cap-check        # capability gate: zero-cap floor + fail-closed native tier
$BIN trace-check      # eval trace facets: default-off, deterministic, covering
$BIN trace-run -c 'fn main() { }'  # run under the eval trace (RSMETA_CAPS gates native)
$BIN diag-check       # positional diagnostics: line/col + caret from token indices
$BIN ast-canonical-check # sig faithful on generics (`<T>` kept, injective)
$BIN ast-diff-check   # canonical-AST semantic diff (localized; analogue of pnix-rs ir-diff)
$BIN rust-ir      -f samples/factorial.rs   # content-addressed canonical Rust IR + stable ir_hash
$BIN rust-ir-check    # ir_hash stable/format-invariant/faithful/evaluable
$BIN borrow-boundary-check # ownership boundary: rustc reason codes preserved (interp != borrow checker)
$BIN trait-boundary-check  # trait surface: supported vs held (assoc-type/dyn/where/blanket)
$BIN macro-boundary-check  # macro surface: fixed vs macro_rules!/proc held
$BIN rust-artifact -f samples/factorial.rs   # per-program native artifact receipt (reproducible)
$BIN fuzz-gen 7     # emit one deterministic well-defined generated Rust program
$BIN fuzz-check     # DIFFERENTIAL TESTING: generated Rust, interp-stdout == rustc-stdout
$BIN fuzz-scale 250 # deep differential search over 250 generated programs
$BIN emi-check      # METAMORPHIC: dead-branch mutation preserves interp & rustc stdout
$BIN rust-surface  -c 'macro_rules! m {}'    # per-program trait+macro surface (supported vs held)
$BIN source-ast-check # rs-meta src/*.rs parses under the rs-meta front-end
$BIN source-bundle-check # all-source bundle stdout matches rustc
$BIN stage2-chain-check  # all-source evaluator' replays positive corpus and matches rustc
$BIN stage2-probe-check  # lexer/parser/typeck/interp source slices agree with rustc
$BIN stage3-chain-check  # slim evaluator stage2 -> stage2' chain agrees with rustc
$BIN stage3-all-source-smoke-check # slimmed evaluator-core stage2 -> stage2' smoke
$BIN stage3-core-mini-check # evaluator-core stage2' mini-corpus replay
$BIN stage3-core-prefix-check # evaluator-core stage2' corpus prefix replay
$BIN stage3-core-middle-check # evaluator-core stage2' corpus middle replay
$BIN stage3-core-suffix-check # evaluator-core stage2' corpus suffix replay
$BIN stage3-core-feature-check # evaluator-core stage2' named feature corpus replay
$BIN stage3-core-negative-check # evaluator-core stage2' named negative corpus rejection
$BIN stage3-core-negative-middle-check # evaluator-core stage2' negative corpus middle rejection
$BIN stage3-core-negative-suffix-check # evaluator-core stage2' negative corpus suffix rejection
$BIN stage3-mirror-check # stage1/stage2/stage2' canonical AST + output mirror
$BIN stage3-fixedpoint-check # stage2 (B) == stage2' (C) evaluator transcript fixed point
$BIN stage3-full-chain-check # all-source evaluator stage2' chain replay (budgeted)
$BIN stage3-full-held-check # full all-source stage3 boundary matches manifest
$BIN stage8-repro-check # same Rust source yields same native artifact receipt
$BIN stage8-selfhost-repro-check # stage2 evaluator artifact reproducibility
$BIN manifest-check   # validate proofs/stage-manifest.tsv
$BIN isolation-check  # fresh interpreter runs do not leak state
$BIN constitution-check # zero-dep/local-only/determinism guard
$BIN actions-disabled-check # GitHub Actions disabled; local verification only
$BIN native-cache-check # native rustc compile cache probe
$BIN stage9-replay-check # clean-process product entrypoint matrix seed
$BIN stage9-proof-matrix-check # clean-process proof command matrix
$BIN stage9-aggregate-replay-check # bounded proof aggregate replay
$BIN stage10-session-check # deterministic clean-process session replay seed
$BIN stage10-sandbox-check # client/server/session/sandbox closure
$BIN stage11-adapter-check # adapter schema/held/conflict seed
$BIN stage11-adapter-replay-check # multi-domain adapter replay closure
$BIN stage12-quarantine-check # self-improvement quarantine seed
$BIN stage12-quarantine-replay-check # quarantine replay closure
$BIN stage13-horizon-check # long-horizon stale/boundary seed
$BIN stage13-horizon-replay-check # long-horizon organism replay closure
$BIN stage14-cross-impl-check # cross-implementation export seed
$BIN stage14-cross-impl-replay-check # cross-implementation replay closure
$BIN stage15-evidence-check # open-world evidence federation seed
$BIN stage15-evidence-replay-check # open-world evidence replay closure
$BIN stageN-extension-check # versioned constitutional extension seed
$BIN stageN-extension-replay-check # versioned extension replay closure
$BIN check            # self/tv/typeck/source/stage2/stage3/stage8/manifest/isolation/constitution/actions/cache/stage9/stage10/stage11/stage12/stage13/stage14/stage15/stageN checks
$BIN stage-status   # the stage0..stageN ladder + honest status

$BIN run        -c 'fn main() { println!("{}", 1 + 2 * 3); }'
$BIN run        -f samples/factorial.rs    # interpreter
$BIN ast        -f src/ast.rs -f src/parser.rs  # ordered multi-file load
$BIN native-run -f samples/factorial.rs    # via rustc
$BIN ast        -f samples/factorial.rs    # parsed AST (derive Debug; unstable)
$BIN ast-canonical -f samples/factorial.rs  # stable canonical AST serialization
$BIN typecheck  -f samples/factorial.rs      # certify well-typedness (floor typeck)
```

## Evaluated subset

`fn` / `struct` / `enum` (unit/tuple/struct-like variants) / inherent `impl` items; `i64` / `i32` / `u32` / `u64` / `u8` /
`usize` / `f64` / `char` / `bool` / `()` / named / tuple / reference / `Vec<T>` types;
`impl Trait` plus shallow generic `fn`/`struct`/`enum` unification, trait item
surface and `impl Trait for Type` value-type method dispatch (full trait solving
held), plus narrow `impl Into<String>` `.into()` support;
top-level `type Name = ...;` aliases are preserved and resolved by typeck;
integer literal suffixes for known integer types plus decimal/hex integer literals;
in-range unsuffixed integer literal inference in expected integer contexts;
`[T]` / `&[T]` slice type surface with `.len` / `.get` / `.iter` / indexing;
lifetime tokens/params (`'a`, `&'a T`) parsed and erased;
`let` (+`mut`) and destructuring `let pat = expr;`, assignment expressions and compound assignment expressions (including bool `&=`), tuple enum variant constructor values,
statement/expression `return`, expression and block statements;
integer/char/bool/string literals (including Rust line-continuation escapes), variables, `as` casts,
`&expr` reference creation for rvalues, `&mut place` reference creation for mutable places,
`*` deref, free function / associated / method calls,
receiver forms `self` / `&self` / `&mut self`, unary `- !` (including ref bool/int autoderef), binary `+ - * / % ^` and
`== != < <= > >=` and short-circuit `&& ||`; `if`/`else`, blocks, `match`
(wildcard / binding / `ref` / `ref mut` / `@` binding / int / int range / char / char range / string / bool / tuple / reference / or-pattern / struct / enum tuple+struct-like patterns including field `..` rest / prelude Option/Result patterns plus guards, with one-layer auto-deref for literal/destructuring patterns), tuple compatibility, tuple / struct / enum
literals including `E::V { field: value }`, struct field shorthand, field `.x` and tuple `.0` access, range expressions `a..b` / `a..=b` as `Iter`, `while` / `loop` / `for pat in iter` / `for a..b` (`..=`)
with `break` / `continue`; mutable struct field assignment; char predicate methods and `.to_string`;
`break` / `continue` typed as diverging expressions for match/loop absorption;
blocks whose last statement diverges type as `!`;
`if let pat = expr { ... } else { ... }`, `while let pat = expr { ... }`, and `let pat = expr else { diverge };`;
`Vec::new`, `Vec::with_capacity`, `vec![...]`, array literals `[a, b]` and repeat
forms `[x; n]` / `vec![x; n]` (Vec-backed), `.push`, `.pop`, `.remove`, `.get`, `.first`, `.last`, `.last_mut`, `.iter`, `.len`,
`.is_empty`, `.clear`, `v[i]` reads/writes, `.iter`, `.iter_mut` (read-observation surface), `.into_iter`, stateful `Iter::next` / `Iter::nth`, `Iter::last`, `Iter::map`, `Iter::zip`, `Iter::all`, `Iter::any`, `Iter::rev`, `Iter::enumerate`, `Iter::find`, `Iter::position`, `Iter::count`, `Iter::sum`, `Iter::fold`, `Iter::take`, `Iter::skip`, `Iter::copied`, `Iter::cloned`, and `for x in v` / `for x in &v` / `for x in &[T]`;
string-like `Vec::join`;
Vec range slicing `v[a..b]` / `v[a..=b]` / `v[..b]` / `v[a..]` and `.to_vec()`;
`String::new/from`, `.to_string`, `.push_str`, `.push(char)`, `.len`, `.trim()`, `.split()`, `.chars()` as `Iter<char>`, `.bytes()` as `Iter<u8>`, `.as_bytes()` as `&[u8]`, `.chars().map(...).collect::<String>()`, `Vec<char>::iter().filter(...).collect::<String>()`,
`.is_empty`, `.as_str`, `.contains`, `.starts_with`, `String + &str`, `String == &str`, string-like `&String`, and `&String` to `&str` call coercion;
typed/zero-arg closure literals with value capture, parameter patterns, return annotations, closure variable calls, immediate expression calls, and top-level function items as callables;
selected built-in `.clone()` surface for current value types, including deep
String / Vec / struct / enum / iterator-state value clones;
method turbofish parsing and `str/String.parse::<i64>()`;
`Some` / `None` / `Ok` / `Err` plus core `Option` / `Result` methods (`as_ref` / `map` / `and_then` / `copied` / `cloned` / `unwrap_or_else` / `ok_or_else` / `map_err` included), `None` placeholder assignment/comparison joining, and `?`
early-return;
`Box::new/as_ref`, `Rc::new/clone/ptr_eq/as_ref`, narrow `Rc<Vec<T>>.len/is_empty/iter/get/index`, narrow `Rc<String>.as_str/chars`, `&Box<T>` / `&Rc<T>` to `&T` and `&&T` to `&T` call coercions, and `RefCell::new/borrow/borrow_mut`;
`HashMap::new`, `.insert`, `.contains_key`, `.get`, `.get_mut` (including `String` key lookup by `&str`), `.remove`, `.len`, `.is_empty`, `.iter`, `.entry(...).or_insert/_with`, `.and_modify`;
ignored self-host surface items (`#[...]`, doc comments, `pub`, top-level `use`, top-level `mod`);
top-level immutable `const` / `static` globals; known fully-qualified `std::...`
path canonicalization for the supported std surfaces;
`std::fs::create_dir_all/write/read/read_to_string`, `Path::new`, `PathBuf::from`,
`Path/PathBuf::join/display/exists`, `std::env::args/var`, and
`Command::new/arg/env/env_clear/output` with `Output.status/stdout/stderr`
narrow surfaces for the native/check tiers;
recursion and mutual recursion; a minimal
`print!` / `println!`, `eprintln!` (stderr ignored for stdout TV), `format!`
with `{}` / `{0}` / `{name}` / `{:?}` / `{:#?}` / `{:016x}` / `{:<N}` /
`{:>N}` / `{:.N}` (numeric fixed precision, Rust's `N <= 65535`) placeholders and `name = expr`
arguments, `matches!` with pattern
guards; `write!` / `writeln!` for `String` / `&mut String`; `panic!` / `unreachable!` / `todo!` parsed as diverging macros;
`assert!` / `assert_eq!` fixed macros; narrow
`cfg!(name)` platform checks;
`usize::saturating_sub` / `i64::saturating_sub` / `i64::max` / `i64::min`.
Format args are type-checked across Display / Debug / LowerHex / fixed-numeric surfaces,
including Box/Rc Display delegation when the inner value is displayable.
Match type-checking includes a light bool/custom-enum/Option/Result exhaustiveness
check; guarded arms do not count as exhaustive, matching rustc's acceptance
boundary. A light type-checker rejects what `rustc` rejects.
Everything in the corpus is real Rust that also compiles with `rustc`; anything
outside the subset is refused, never silently mistranslated.

## Status

The interpreter, the rustc native tier, the type-checker, and the `check` gate
are all green over a 407-program positive corpus and a 272-program negative corpus:
`self-check` 407/407, `tv-check` (interp == rustc) 407/407, `typeck-check`
(interp rejects iff rustc rejects) 272/272, `source-ast-check` parses all 8
`src/*.rs` files, `source-bundle-check` proves the all-source print_help bundle
matches rustc,
`stage2-chain-check` replays the full positive corpus through the all-source
bundled evaluator', `stage2-probe-check` verifies lexer/parser/typeck/interp source-slice harnesses,
and `stage3-chain-check` verifies a slim evaluator stage2 -> stage2' chain
against rustc. `stage3-all-source-smoke-check` now uses a slimmed evaluator-core
source bundle so the source-loader smoke proof stays inside the local cost
budget, `stage3-core-mini-check` has that stage2' replay a small
arith/recursion/enum/struct/Vec-String/iterator-turbofish corpus, and
`stage3-core-prefix-check` has it replay the first 8 positive corpus cases.
`stage3-core-middle-check` has it replay the middle 8 positive corpus cases.
`stage3-core-suffix-check` has it replay the last 8 positive corpus cases,
tracking the current corpus head/middle/tail.
`stage3-core-feature-check` separately replays 10 later feature-heavy corpus
cases under that same stage2' evaluator.
`stage3-core-negative-check` has stage2' reject 10 representative negative
corpus cases, while `stage3-core-negative-middle-check` and
`stage3-core-negative-suffix-check` have it reject the middle and last 8
negative corpus cases, extending stage3 evidence to the acceptance boundary.
`stage3-mirror-check` appends a hand-written canonical AST serializer
(`proofs/mirror-sig.rs`) to the evaluator-core bundle and requires stage1
(rustc-native), stage2, and stage2' to emit byte-identical canonical AST +
probe output for `samples/mirror_probe.rs` (derived `Debug` is deliberately not
used: interpreter debug rendering is not byte-faithful to rustc's derive).
`stage3-fixedpoint-check` compares the stage2-materialized evaluator (B) and the
stage2'-materialized evaluator (C) on the same transcript and requires them to
be identical — a normalized, bounded B==C fixed point; the corresponding binary
artifact reproducibility receipt is `stage8-selfhost-repro-check`.
`stage3-full-chain-check` (budget-gated) replays the full positive corpus
through the all-source stage2' evaluator and matches rustc; `stage3-full-held-check`
guards its DONE/budget/cost manifest row against drift. `stage8-repro-check` seeds native artifact reproducibility by
building a sample Rust source and the all-source evaluator bundle in two
workdirs and comparing canonical receipts (source hash, rustc version,
deterministic flags, artifact hash).
`manifest-check` validates `proofs/stage-manifest.tsv`, a machine-readable index
of stage status, local check commands, timeout budgets, and cost notes.
`isolation-check` verifies fresh interpreter runs do not leak stdout or function
namespace state between programs.
`constitution-check` enforces the local project constitution: no crates.io
dependencies, GitHub Actions disabled, and content-hash native artifact names.
`native-cache-check` verifies repeated native execution reuses the content-hash
rustc artifact cache; stage8 receipt paths still force fresh compiles.
`stage9-replay-check` starts clean subprocesses for a lightweight product
entrypoint matrix (`help`, `stage-status`, `run`, `native-run`, `ast`,
`manifest-check`) with hard-fixed `SOURCE_DATE_EPOCH` and soft-observed `PATH`
for rustc lookup, then emits canonical JSON receipts.
`stage10-session-check` replays a fixed clean-process command session twice and
requires identical canonical transcripts.
`proofs/rustc-bootstrap-map.md` pins the stage0/1/2/3/stage8 vocabulary mapping
and the current held boundaries.
`run` / `ast` / `native-run` accept repeated `-f file.rs` inputs for ordered
multi-file loading. GitHub Actions are intentionally disabled; local
`cargo build` plus `bootstrap check` is the verification source of truth.
See `todo.md` for the stage15-N ladder: stage1/stage2, the stage3 closure
(core shards, canonical AST mirror, bounded B==C fixed point, and the full
all-source stage2→stage2' corpus replay — 2103s with a release build,
budget-gated via `RS_META_STAGE3_FULL_CHAIN_BUDGET_SECS`), and the
stage8-stageN local replay receipts are all DONE. The full-chain pass required
fixing a real self-interpretation fidelity bug (`value_eq` lacked a ref-aware
`Vec` arm, so cloned `Vec<enum>` slice equality broke only at nesting depth 3);
`stage3-full-held-check` now guards the DONE boundary row against drift.

## Differential testing
The self-growing interp==rustc discipline (fuzz-check + emi-check) and its
roadmap are documented in [docs/differential-testing.md](docs/differential-testing.md).
