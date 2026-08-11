# rs-meta status (peer host-meta floor)

Last verified: 2026-08-07.

## Peer-floor statement

**rs-meta** is a standalone Rust-in-Rust meta-circular compiler/evaluator
(zero crates.io deps; `rustc` is an external toolchain only). Practical peer
floor:

| Peer | Peer floor | rs-meta counterpart |
|---|---|---|
| hy-meta | stage ladder through ~15 | stage-manifest DONE through stageN seeds |
| cljs-meta | fixed-point (B==C) | stage3-fixedpoint-check (evaluator B==C) |
| clj-meta | selfhost + stock rebuild | source-bundle + stage2/3 chain + emit self-host |
| clr-meta | eval gens + C0–C3 | interp + native TV + Stage1/2-style chain |

**Core honesty:** translation validation (`interp` stdout == `rustc` stdout) is
the permanent green bar. Stage15/N rows in the manifest are **local seed/replay
closures**, not full rustc replacement or Trusting-Trust defense.
Borrow checker / full trait solver / user `macro_rules!` remain held; core
self-host is **not** blocked by those held features (see `docs/self-hosting.md`).

## Closed claims

Live-verified this session (2026-08-07):

```text
cargo build                           OK (~13s debug)
bootstrap self-check                  PASS 407/407
bootstrap tv-check                    PASS 407/407 (interp == rustc)
```

Documented DONE in `proofs/stage-manifest.tsv` (not all re-run this session):

```text
typeck-check, source-ast/bundle, emit-tv/self-host
stage2-chain/probe, stage3 shards + fixedpoint + full-chain (budget)
stage8..stageN seed + local replay closures
fuzz/emi differential discipline, selfhost-audit, constitution
```

## Open claims (do not claim)

```text
full_rustc_replacement = false
trusting-trust_defense = false
borrow_checker = held
full_trait_solver / dyn Trait / where / blanket = held
user_macro_rules_and_proc_macros = held
whole_binary_self_interpretation_default_gate = false
  (full-chain is budget-gated, not default primary)
```

**Correction (verified this session, 2026-08-11):** the "source-ast/bundle" row
above claimed DONE in `proofs/stage-manifest.tsv` is stale for
`source-bundle-check` specifically. `stage9-aggregate-replay-check` (and the
full `check` aggregate) fail on a completely unmodified checkout — confirmed
via a `git worktree` at the pre-session commit, not a regression from any
recent change. Root-caused two layered pre-existing bugs, one fixed:

```text
FIXED: parser.rs's format!-arg-usage validator didn't understand `{:.*}`
  (dynamic precision from an extra positional arg) -- it counted `{:.*}` as
  consuming one positional arg (the value) when real Rust's format! macro
  consumes two (precision, then value). Caused src/interp.rs's own
  `format!("{:.*}", precision, f)` (line ~2463, real, valid, rustc-accepted
  Rust) to fail rs-meta's OWN self-parse with "positional arg 1 is never
  used". Fixed.

FIXED: parser.rs's struct-pattern-field parser didn't recognize the `ref`
  shorthand (`Struct { ref x, .. }`) -- it read `ref` itself as if it were
  the field name. src/io.rs:97 uses exactly this
  (`Err(MetaIoError { ref error_class, .. }) if ...`), a real, valid Rust
  pattern. Fixed by special-casing `ref [mut] ident` in
  `parse_pattern_fields`, reusing the existing `Pattern::BindRef` AST node
  (already used for top-level `ref` patterns). io.rs now parses cleanly:
  source-ast-check is 16/16 (io.rs added to `source_files()` in check.rs;
  was 14/14 without it, would have been 15/15-with-a-failure without the
  ref fix).

ADDED (additive, no risk to existing passes): fs::symlink_metadata,
  fs::metadata, fs::read_dir type signatures, and Metadata/FileType method
  modeling (`.file_type()`, `.is_symlink()`/`.is_dir()`/`.is_file()`) in
  typeck.rs -- io.rs's `classify()` needs these to type-check.

FIXED: `Path::new(s)` was typed as returning owned `PathBuf` in typeck.rs;
  real Rust's `Path::new` returns borrowed `&Path`. io.rs's
  `classify(Path::new(path))` (classify takes `&Path`) surfaced this as a
  type mismatch. Changed the rule to return
  `Type::Ref { mutable: false, inner: Named("Path") }`. Verified safe: this
  is method-call-transparent (`method_target_name` already derefs `Ref` to
  find the target type, so `.exists()`/`.display()`/`.join()` etc. still
  resolve correctly), and re-confirmed self-check 407/407, tv-check 407/407,
  typeck-check 272/272 -- no regressions.

STILL OPEN (next layer found, not fixed): getting past Path::new reveals
  io.rs's `read_utf8`/`read_dir` call `e.kind() == std::io::ErrorKind::X`
  on the error from `fs::read`/`fs::read_dir` -- but those are modeled
  (matching the three pre-existing `fs::` functions) as returning
  `Result<_, String>`, a plain String error with no `.kind()` method or
  `ErrorKind` enum. Unlike Path::new, `fs::read`'s `String`-typed error is
  PRE-EXISTING (not added this session) and likely load-bearing elsewhere
  (e.g. anything that formats or `?`-propagates it as a String), so it
  should NOT be blanket-changed the way Path::new was -- the safer shape is
  probably an additive `.kind()` special-case (e.g. in `type_string_method`)
  returning a new small `IoErrorKind`-like type comparable via `==` against
  `std::io::ErrorKind::NotFound`/`NotADirectory` path expressions, without
  touching what `fs::read`'s error type actually is. Not attempted this
  session -- this makes four fixed layers (`.* `, `ref` patterns, Metadata/
  FileType, Path::new) and a fifth found; there is no guarantee this is the
  last one io.rs will reveal, since it was never in the self-hosting bundle
  before this session.

Net effect: `source-bundle-check`, `stage9-proof-matrix-check`,
  `stage9-aggregate-replay-check`, and the full `check` aggregate remain red
  -- but now for this one, precisely-diagnosed, next reason, not
  the "env_clear() strips something macOS needs" hypothesis this
  investigation started from (that hypothesis is ruled out: every failure
  found was a 100% deterministic Rust-subset-parser/typeck feature gap, not
  environment- or platform-dependent). Verified no regressions: self-check
  407/407, tv-check 407/407, typeck-check 272/272,
  independent-mini-backend-check 9/9, source-ast-check 16/16.
```

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

**`mrustc` turned out not to be usable here.** It's packaged in nixpkgs, but
marked `platforms = [ "x86_64-linux" ]` only — this dev machine is
`x86_64-darwin`, and forcing an unsupported-platform build of a *trust*
witness through `NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM` would be actively
counterproductive (a shakily cross-built mrustc could silently miscompile
and give false DDC confidence, which is worse than not having the check).
The mrustc phased plan below remains a valid option on Linux; the concrete
progress this session instead follows the same in-house pattern the other
four hosts already used.

**Independent mini backend added this session (2026-08-11):**
`independent_mini_backend.rs` is a new, from-scratch tokenizer/parser/
tree-walking interpreter for a small `i64` Rust subset (`fn`, `if`/`else`,
`+`/`-`/`*`, `<`/`>`/`<=`/`>=`/`==`, recursive calls), sharing zero code with
`lexer.rs`/`parser.rs`/`ast.rs`/`typeck.rs`/`interp.rs` — the evaluator core
`tv-check` already proves `== rustc` on. `rustc` itself remains the trusted
oracle, the same honest role real upstream Hy plays for the Python host's
`independent_mini_backend.py` and the self-hosted compiler plays for the
ClojureScript host's `independent_mini_backend.js`.

Covers 9 fixtures, cross-validated against real `rustc` (via `native::native_run`,
the same mechanism `tv-check` uses) — both agree on all 9, including a
recursive factorial. Wired into `independent-mini-backend-check` (both as its
own CLI subcommand and folded into the `check` aggregate). Verified live this
session: 9/9 accepted, `self-check` 407/407 and `tv-check` 407/407 re-run
unaffected (no regressions), full `check` aggregate green.

**What this closes and what it still doesn't:** a genuine 2-way behavioral
comparison (real `rustc` ≡ from-scratch interpreter) now exists and passes,
not just a documented plan. It is still only 9 fixtures against a *fresh*
independent implementation — not the 407-case corpus, and (same honest bar
every host settled on this session) an *interpreter*, not a second
*compiler*, so it does not by itself clear the full Wheeler bar the way a
genuine mrustc-vs-rustc compiler comparison would. **Next concrete step:**
widen the fixture set (loops, more arg arities, string/bool handling) toward
the 407-case corpus, and keep the mrustc phased plan on file for whenever
this runs on Linux.

## Primary gate

```sh
# From pnix-rs/rs-meta/
./bin/rs-meta-gate                 # cargo build + self-check + tv-check
./bin/rs-meta-gate self-check
./bin/rs-meta-gate check           # full local check aggregate (long)
```

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `self-check` | **PASS** 407/407 | cargo 1.97.1 / rustc 1.97.1 |
| `tv-check` | **PASS** 407/407 | |
| full `bootstrap check` | not default primary | longer; includes stage matrix |
