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
