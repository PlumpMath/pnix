# clr-meta Compiler Stage10 design

Status: **closed (live gate PASS)** 2026-08-12. Parent floor is Stage9.

## Goal

Isolated load-context, classpath, session, and sandbox replay (the roadmap's
own one-line Stage10 definition). Two parts: a policy table
(`proofs/session-sandbox.tsv`) declaring an explicit stance for every
relevant boundary, and a live replay of the two properties that are actually
locally checkable — not asserted, not assumed.

## What's live-replayed (grounded in real checks, not just declared)

1. **Load-context shadow rejection.** `bin/clr-meta` already rejects any
   `pnix.clr-meta` namespace shadow planted in the pinned ClojureCLR runtime
   root before running anything (`bin/clr-meta` lines ~119-126) — this
   existed before Stage10, just untested by any gate. The gate plants a real
   shadow DLL, runs `bin/clr-meta -e "(+ 1 1)"` under `env -i` twice,
   requires both runs to fail with exit 2 and byte-identical stderr, then
   removes the shadow and confirms the *same* command succeeds again (proving
   the rejection was actually caused by the planted file, not an unrelated
   breakage).
2. **Session replay.** A two-command session (`--gate` then
   `-e "(+ 40 2)"`) through `bin/clr-meta`, run twice under `env -i`,
   requires byte-identical combined stdout across both runs.

## What's policy-only (declared, not independently re-proven here)

- **Classpath scoping**: `CLOJURE_LOAD_PATH` is always set to `clr-meta/src`
  only for tool invocations (verified by reading `bin/clr-meta`'s source,
  not by a separate live probe in this gate — a static claim about the
  script's own text, which Stage9's clean-process matrix already exercises
  functionally).
- **Sandbox env**: every compiler-selfhost stage gate (1-9) already routes
  its `dotnet` subprocess calls through an explicit `env -i` allowlist —
  this is Stage1-9's own established practice, recorded here as a Stage10
  policy line rather than re-verified a second time.
- **Remote CI** (`HELD`): the monorepo-level GitHub Actions workflow
  (`.github/workflows/hosts.yml`, `clr-gate` job) exercises this host on
  every PR via `nix run .#gate`, but its outcome is external and not
  fetched or trusted by any local proof check.
- **Network / external sandbox** (`HELD`): out of local proof scope; no
  adapter receipts exist for either.

## Non-claims

Stage11-15/N, compiler self-reproduction, IL fixed-point, ClojureCLR
replacement, promotion.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage10-gate
```

## Live receipt

`work/compiler-selfhost-stage10-gate.receipt.json` (gitignored) with
`claims.stage10 = true`, `claims["promotion/allowed?"] = false`.
