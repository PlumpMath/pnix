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
