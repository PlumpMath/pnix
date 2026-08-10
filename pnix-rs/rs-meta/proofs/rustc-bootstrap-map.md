# rs-meta rustc bootstrap map

Date: 2026-07-01

This note fixes how rs-meta maps the familiar rustc bootstrap vocabulary onto
the current Rust-in-Rust evaluator chain. It is a receipt/index document, not a
correctness proof.

## Mapping

| rustc term | rs-meta term | Current receipt |
|---|---|---|
| stage0 compiler | host `rustc` building `bootstrap` | `cargo build` |
| stage1 compiler | `bootstrap` interpreter evaluating Rust subset programs | `self-check`, `tv-check`, `typeck-check` |
| stage1 compiler source coverage | rs-meta source parses and bundle runs | `source-ast-check`, `source-bundle-check` |
| stage2 compiler | all-source bundled evaluator' produced/loaded by stage1 | `stage2-chain-check` replays positive corpus 275/275 |
| stage2 component probes | lexer/parser/typeck/interp source slices under rs-meta and rustc | `stage2-probe-check` |
| stage3 seed | slim evaluator stage2 loads/evaluates slim stage2' | `stage3-chain-check` |
| artifact reproducibility seed | native rustc artifact receipts for sample + all-source bundle | `stage8-repro-check` |

## Held Boundaries

- Full all-source `stage2 -> stage2'` chain is not claimed. The local probe hit
  the current 420s budget; slimming/cache/direct-AST work is required.
- B==C fixed point is not claimed.
- Trusting-Trust defense is not claimed. Reproducibility receipts are evidence
  about this implementation's replay, not a diverse-double-compilation proof.
- GitHub Actions are disabled. The source of truth is local verification:
  `cargo build` plus `bootstrap check`.

## Current Local Gate

`bootstrap check` currently covers:

- positive corpus: interpreter stdout equals expected and rustc stdout
- negative corpus: interpreter rejects iff rustc rejects
- source parse/bundle gates
- all-source evaluator' corpus replay
- source-slice probes
- slim stage3 chain
- stage8 artifact receipt seed
- machine-readable stage manifest validation
