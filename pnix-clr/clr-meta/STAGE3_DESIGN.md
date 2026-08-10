# clr-meta Compiler Stage3 design

Status: **implemented + gated** (2026-08-07). Live gate
`scripts/clr-meta-compiler-selfhost-stage3-gate` PASS; receipt under
`work/compiler-selfhost-stage3-gate.receipt.json`. Highest closed compiler
floor is now **Stage3** (C3 Stage2 remains the parent). Stage4+ still open
(`STAGE15_N_ROADMAP.md`).

## Goal

**Stage3** is the first **same-source recompile convergence** step after C3
Stage2:

```text
Stage1 (C2 host-seeded) → Stage2 (C3 same-source) → Stage3 (Stage2 recompiles
same frozen kernel → semantic + structural convergence under fresh load)
```

Stage3 is **not**:

- evaluator generation nesting (gen0–2 stay separate);
- Stage15/N open-world evidence;
- full ClojureCLR replacement;
- PE byte-identity fixed point (that is Stage8 policy);
- automatic host promotion into tri/five-host full corpus.

## Preconditions (already closed)

| Checkpoint | Claim |
|---|---|
| C0 | selfhost ABI/attack contract |
| C1 | recursive source admission of frozen kernel |
| C2 | host-seeded executable Compiler Stage1 PE |
| C3 | Stage1→Stage2 same-source compile + source-hidden fresh-target replay |

Inputs Stage3 must pin by hash:

- C3 Stage2 PE + support triplet + Stage2 manifest
- frozen `pnix.clr-meta.compiler-kernel.v1` source closure (byte-identical to C3)
- profile / plan / toolchain snapshot digests from C3 lineage

## Stage3 definition of done

A Stage3 gate receipt (`compiler_stage3=true`) only if **all** hold:

1. **Parent bind** — Stage2 artifact and its parent lineage hashes match the
   frozen C3 receipt (no silent parent rewrite).
2. **Same-source recompile** — Stage2 (not Stage1, not host ClojureCLR) compiles
   the **exact** frozen kernel source into a Stage3 PE bundle.
3. **Fresh load** — Stage3 PE loads in a directory containing only Stage3 +
   support; no Stage1/Stage2 PE, no kernel source, no ClojureCLR product load
   path.
4. **Semantic agreement** — Stage3 compiles and runs the post-Stage2 nonce
   target family and the arithmetic/equality/truthiness targets; observations
   match Stage2 on the same inputs.
5. **Structural description equality** — method/field inventory and normalized
   structural description equal Stage2 (same policy as C3: raw PE bytes may
   differ until Stage8).
6. **Source-hidden replay** — recompile + target execution succeed with kernel
   source absent from the compiler directory (same honesty as C3 replay).
7. **No auto-promotion** — receipt records
   `compiler_self_reproduction=false`, `compiler_stage15_n=false`,
   `il_fixed_point=false`, `promotion/allowed?=false`.

## Explicit non-goals for Stage3

```text
compiler_stage4_through_7_convergence   # later stages
compiler_self_reproduction              # Stage2 already emits Stage3; full
                                        # closed self-reproduction loop is
                                        # a later named gate
clr_il_fixed_point / raw PE equality    # Stage8
broad_clojureclr_compatibility
pnix_common_compiler_integration
cross_host_canonical_equivalence
```

## Work packages (implementation order)

### WP-A — Stage3 plan + receipt schema

- Extend artifact/plan schema with `compiler_stage3` boolean (default false).
- New receipt schema
  `pnix.clr-meta.compiler-stage3.receipt.v1` with fields:
  - parent Stage2 digests, source closure digests
  - Stage3 output digests, structural description digest
  - semantic target matrix (nonce + arithmetic family)
  - `source_hidden_replay: true|false`
  - `promotion/allowed?: false`

### WP-B — Stage3 builder

- Input: verified C3 Stage2 bundle + frozen kernel source.
- Action: invoke Stage2 as the compiler (not host AOT) on kernel source.
- Output: Stage3 PE + support copy + Stage3 manifest.
- Fail closed if Stage2 missing, source hash drift, or host fallback observed
  for admitted forms.

### WP-C — Stage3 gate script

- `scripts/selfhost-stage3-gate` (name fixed at implementation time):
  1. verify parent C3 receipt
  2. build Stage3 (or `--no-build` consume existing)
  3. structural compare Stage2↔Stage3
  4. source-hidden fresh-target replay with Stage3 only
  5. write receipt; exit 0 only when definition-of-done holds

### WP-D — Mutation / negative matrix

Carry C2/C3 style no-output failures:

- identity/metadata mutation on Stage3 publication
- arithmetic lowering mutation on Stage3 control path
- missing support / wrong parent hash / source drift

### WP-E — Documentation honesty

- Update `STATUS.md` only after live green gate.
- Keep `STAGE15_N_ROADMAP.md` Open claims until WP-C passes.
- Do not fold Stage3 into `pnix-clr` product artifact plan until a separate
  product-admission decision.

## Suggested acceptance command (future)

```sh
# From pnix-clr/clr-meta/  (design only — script not landed yet)
./scripts/selfhost-stage3-gate --build
# expects: receipt compiler_stage3=true, promotion/allowed?=false
```

## Relationship to product hosts

Stage3 advances **clr-meta** compiler self-hosting. It does **not** by itself:

- widen `pnix-clr` language surface;
- admit clr into full tri-host corpus (use five-host **common slice** first);
- claim Trusting-Trust defense.

## Exit criteria for “Stage3 closed”

Live machine evidence:

1. stage3 gate exit 0
2. receipt hash recorded in STATUS
3. Open claims flip only:
   - `compiler_stage3` (narrow) → true  
   - Stage4–7 / self-reproduction / fixed-point remain false until their gates
