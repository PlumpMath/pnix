# clr-meta Compiler Stage8 design

Status: **closed (live gate PASS)** 2026-08-12. Parent floor is Stage7.

## Goal

Reproducible assembly artifact closure for the `PersistedAssemblyBuilder`-based
Compiler Stage1-7 family: an explicit, empirically-grounded policy for the
non-deterministic PE fields this codegen path actually produces, plus a gate
proving two independent Stage7 builds from the same frozen Stage6 parent are
byte-identical under that policy.

## What was actually non-deterministic (found by measurement, not assumed)

Two builds of the same frozen source, a few seconds apart, produced
`CompilerStage7.dll` outputs differing at exactly 17 bytes, in exactly two
regions (confirmed via `cmp -l` byte-offset diffing before any fix was
written):

1. **PE COFF header `TimeDateStamp`** (4 bytes at
   `PEHeaders.CoffHeaderStartOffset + 4`) — the real build wall-clock time.
2. **Module `Mvid`** (16 bytes in the `#GUID` metadata heap) — a fresh random
   GUID `PersistedAssemblyBuilder` assigns on every `Save()`.

Nothing else varied. In particular: this codegen path never writes a PDB or
embeds a debug directory, so debug info and embedded source paths are not a
non-determinism source *for this artifact family* — that is a checked fact
about `PeSink.cs`, not an assumption that generalizes to any future codegen
path that does emit debug info.

## Policy

`PersistedAssemblyBuilder.Save()` exposes no public API to set either field.
`PeSink.Finish()` now canonicalizes both, after `Save()` and before the
artifact is moved to its final path, for every artifact this pipeline
produces (not gated behind a flag — neither field is semantically observable
by `compile`/`invoke`/`describe`/`prepare`/`publish-directory`):

- `TimeDateStamp` → `0`.
- `Mvid` → `00000000-0000-0000-0000-000000000000`.

The timestamp is located via `PEHeaders.CoffHeaderStartOffset` (a real PE
structural offset, not a hardcoded byte position). The MVID is located by
reading the *actual* GUID `MetadataReader.GetModuleDefinition().Mvid`
reports, then requiring that exact 16-byte sequence to occur exactly once in
the file before overwriting it — not a hardcoded heap offset, since heap
layout shifts with the compiled program's own content. See
`compiler-selfhost/stage8-contract.edn` for the full policy record.

A new `describe-determinism` verb on `Pnix.ClrMeta.CompilerSupport` reads both
fields back from a finished artifact independently of the writer, so the gate
does not just trust that `Finish()` ran — it re-derives the claim from the
artifact itself.

## Non-claims

Stage9 (clean-process replay), compiler self-reproduction, IL fixed-point,
Trusting-Trust defense, ClojureCLR replacement, promotion. Determinism for any
future codegen path that emits debug info or embeds source paths is
unaddressed — this contract only covers the fields `PeSink.cs` was measured
to actually vary.

## Commands

```sh
./clr-meta/scripts/clr-meta-compiler-selfhost-stage8-gate
dotnet runtime/Pnix.ClrMeta.CompilerSupport.dll describe-determinism ARTIFACT.dll
```

## Live receipt

`work/compiler-selfhost-stage8-gate.receipt.json` (gitignored) with
`claims.stage8 = true`, `claims.raw_artifact_reproducibility = true`
(scoped to `compiler_stage7_persisted_assembly_builder_output`),
`claims["promotion/allowed?"] = false`.
