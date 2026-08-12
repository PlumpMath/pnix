# clr-meta compiler self-reproduction / B==C fixed point

Status: **closed (live gate PASS)** 2026-08-12. Depends on Stage8's PE
canonicalization.

## Goal

The `todo.md`-tracked open item: prove the "kernelB compiles kernelC, B==C"
pattern hy-meta and rs-meta both already close — not just Stage3-7's
existing structural-description equality to the immediate parent, but a
genuine self-hosting fixed point.

## What was found (not built from scratch)

Stage8's own gate output already logged, as an unplanned bonus observation,
that Stage3 through Stage7's compiled `CompilerStageN.dll` shared one
sha256. This check formalizes that finding: it builds Stage1 through Stage7
fresh and confirms **all seven** stages — not just an adjacent pair, and
including Stage1 itself — share the exact same sha256
(`19872f28ed3576cbdf50001649fb9fd773023778fa0bb5c3d7aee45a61baecb7` in the
verifying run). This holds *because of* Stage8's canonicalization: every
stage compiles the same frozen `compiler_kernel.clj` source, producing the
same IL, through the same `PersistedAssemblyBuilder`-based codegen path —
once the only two non-deterministic PE fields (`TimeDateStamp`, `Mvid`) are
canonicalized away, nothing is left to differ between stages, including
Stage1's host-seeded build (it goes through the same `PeSink.Finish()` path
as every later stage).

A live compile+execute of an unseen target (`unseen_add.clj`) through the
shared Stage7 artifact confirms the shared bytes are not vacuously
identical-but-broken.

## Non-claims

This is a fixed point for the `PersistedAssemblyBuilder` PE-artifact output
of the Compiler Stage1-7 family specifically — not a claim about the CLR IL
format in general, not ClojureCLR replacement, not promotion.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-self-reproduction-check
```

## Live receipt

`work/compiler-self-reproduction-check.receipt.json` (gitignored) with
`claims.compiler_self_reproduction = true`, `claims.fixed_point = true`,
`claims["promotion/allowed?"] = false`.
