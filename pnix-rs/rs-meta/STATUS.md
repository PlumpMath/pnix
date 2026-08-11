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

**Nothing closed yet on this axis** — `todo.md` §6 records it as a one-line
"separate track, held" with no concrete plan. Unlike the other four hosts,
rs-meta has a real advantage here: **`mrustc`** (github.com/thepowersgang/mrustc)
is an already-existing, independently-authored Rust compiler with no shared
codebase with `rustc`, and `docs/self-hosting.md` already leans on mrustc's
precedent to justify holding the borrow checker (mrustc bootstraps real rustc
releases while holding the same feature). That makes rs-meta the most tractable
of the five hosts for genuine DDC, because the independent second compiler
does not need to be built from scratch.

Concrete phased plan:

```text
Phase 1 — feasibility
    Pin an mrustc revision. Confirm it can compile rs-meta's evaluator core
    (lexer.rs/ast.rs/parser.rs/typeck.rs/interp.rs/sig.rs/hash.rs — the exact
    files source-bundle-check already proves interp==rustc on) at all.
    docs/self-hosting.md's audit already shows this core uses zero held
    features (no macro_rules!/async/unsafe/trait/dyn/const-generics), which is
    exactly the surface mrustc is known to bootstrap real rustc through, so
    feasibility risk here is low relative to the other four hosts.

Phase 2 — behavioral DDC (the realistic near-term bar)
    Compile rs-meta with rustc -> binary A, with mrustc -> binary B. Run both
    over the existing 407-case corpus that tv-check already uses; require
    stdout equivalence A==B==interp. This is TV-style DDC: it would catch a
    backdoor unique to either compiler's lineage, since rustc and mrustc share
    no code.

Phase 3 — stronger form (self-compile cross-check)
    Have rustc-built rs-meta and mrustc-built rs-meta each recompile rs-meta's
    own source; compare their outputs' *behavior* against each other and
    against interp. Bit-identical bytecode is not a realistic bar across two
    different compiler backends (different codegen strategies), so behavioral
    equivalence — not raw binary equality — is the honest target, same
    scoping clj-meta already settled on for its own DDC work.
```

**Honest caveat to record once this lands:** mrustc's own development history
was itself validated against rustc (it targets rustc-compatible output), so it
is independently *authored* but not fully independent in *lineage/history* the
way a from-scratch, never-cross-checked-against-rustc compiler would be. Still
a materially stronger result than same-repo dual-path checks — record it as
such, not as full Wheeler DDC.

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
