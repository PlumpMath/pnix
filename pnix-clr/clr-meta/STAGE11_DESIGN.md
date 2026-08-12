# clr-meta Compiler Stage11 design

Status: **closed (live gate PASS)** 2026-08-12. Parent floor is Stage10.

## Goal

One accepted/failed boundary across source, IR, compiler, runtime, and
compatibility surfaces (the roadmap's own one-line Stage11 definition), read
here as: every integration surface clr-meta actually has must be declared in
one adapter policy table (`proofs/adapter-schema.tsv`), and every `DONE`
adapter's closure must be live-replayable.

## Adapters (5 rows)

- `local-clojureclr` (`DONE`) — the `bin/clr-meta` CLI plus the pinned
  ClojureCLR NuGet package and dotnet toolchain. Replayed by running
  Stage9's gate **once** (not twice — see "why replay-once" below).
- `compiler-selfhost-native` (`DONE`) — the `PersistedAssemblyBuilder`-based
  compiler-selfhost artifact family. Replayed by reading Stage8's own latest
  checked receipt (`work/compiler-selfhost-stage8-gate.receipt.json`), not
  by re-running Stage8's gate.
- `github-actions` (`HELD`) — the monorepo-level `hosts.yml` workflow
  exercises this host remotely on every PR, but its outcome is external and
  not fetched or trusted here.
- `external-nuget-feed` (`HELD`) — fetching packages beyond the pinned/cached
  ClojureCLR package is outside the local proof boundary.
- `cross-implementation` (`HELD`) — deferred to Stage14.

## Why replay-once, not replay-twice (a correction made while building this)

The first draft of this gate re-ran Stage9's *entire* gate twice (mirroring
Stage8-10's own "run twice, require identical" pattern). That's wrong here:
Stage9 already proves its own clean-process replay property internally, so
re-running it a second time from Stage11 doesn't add evidence — it just
doubles the cost. Worse, if every later stage (`12`, `13`, `14`, `15`, `N`)
"replayed twice" a predecessor that itself "replayed twice" *its*
predecessor, cost would double at every hop — quadratic in stage depth, and
close to unworkable by StageN. Every stage from Stage11 onward instead calls
its referenced predecessor **once**: enough to confirm the referenced
property still holds against today's source, without re-proving a property
that stage already proved about itself.

## Non-claims

Stage12-15/N, compiler self-reproduction, IL fixed-point, ClojureCLR
replacement, promotion.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage11-gate
```

## Live receipt

`work/compiler-selfhost-stage11-gate.receipt.json` (gitignored) with
`claims.stage11 = true`, `claims["promotion/allowed?"] = false`.
