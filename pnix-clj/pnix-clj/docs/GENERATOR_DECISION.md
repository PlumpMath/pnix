# Candidate GENERATOR decision — for the pnix-clj self-* loop

The one missing piece of the self-* loop was the candidate GENERATOR that feeds
`self-improve` (which already witnesses + gates + ranks). This is the DECISION,
synthesized from `/deep-research` (16 claims confirmed 3-0; the workflow's own
synthesis step was cut off by an API weekly limit, so the decision is completed
here from the verified evidence).

## The evidence converges (all 3-0 confirmed)

| technique | fit for a lazy pure functional Nix-like language | killing pitfall | proven-vs-heuristic |
|---|---|---|---|
| **Escher** — observational-equivalence reduction (CAV'13) | ✅ exact fit: keep one representative per behavioral class by a **value-vector over examples**; ablating it blows up | needs example inputs | observational-equiv on finite examples = **heuristic PROPOSE** |
| **Myth/λ²** — evaluate-during-enumeration (POPL'15) | ✅ pure functional recursive ADTs, higher-order | **trace-complete examples** required (hard in practice); search still blows up | example-driven |
| **Smyth** — live bidirectional eval (2020) | ✅ recursion WITHOUT trace-complete sets; propagates examples **backward** through sketches | partial specs still underdetermine | verifier-drives-generator (CEGIS-like) |
| **Burst** — bottom-up + angelic + FTA (2021) | ✅ recursive functional; 3 spec modes (examples / reference-impl / logical); **no trace-completeness**; incremental spec = **FTA intersection** | angelic assumptions must be discharged | CEGIS refinement loop |
| **Synquid** — refinement types (PLDI'16) | ✅ **provably correct** synthesis; spec decomposition tames blow-up | ★**writing the logical spec is a manual creative step** — cannot autonomously feed self-improve | **PROVEN** (decidable) |
| **Knuth-Bendix equivalence reduction** (VMCAI'19) | ✅ equational specs → confluent terminating TRS → canonical normal form; **~80% of candidates pruned** before verification (21M→20% at 11 nodes) | needs an equational theory | canonical dedup = sound pruning |

## THE DECISION

**Build FIRST — an observational-equivalence-reduced bottom-up enumerative
synthesizer** (Escher's mechanism, C11), because it is the single best fit for
pnix-clj's exact situation:

1. **The dedup oracle already exists.** Escher's value-vector reduction needs an
   evaluator to compute each candidate's behavior over example inputs — pnix-clj
   **already has** `core/eval-source` (and the whole `run-witnessed` verifier).
   The one expensive dependency of this technique is a free gift here.
2. **It fits a lazy pure functional language directly** (Escher/Burst are for
   exactly this class) — no host-interop, no mutation, values are pure EDN.
3. **It lands cleanly on the constitution's proven-vs-heuristic boundary.** A
   value-vector match on finite examples is *observational equivalence*, which is
   a **heuristic PROPOSE**, not a proof — precisely the constitution's rule. The
   generator therefore emits PROPOSED candidates; `run-witnessed` proves they are
   well-behaved pnix programs, and `arith-proof`/`bool-proof` can upgrade the
   equivalence to PROVEN where applicable. Everything stays HELD (self-mod-gate).
4. **It avoids the killing pitfalls.** It does NOT require Myth's
   trace-completeness (we enumerate whole expressions, not recursive traces), and
   it does NOT require Synquid's manually-written logical spec (the pitfall C10
   that makes refinement-type synthesis unable to feed self-improve autonomously).

**Plug-in point:** a new `pnix-clj.generate` emits candidate pnix expression
strings; `synthesize-and-propose` hands them to `self-improve/evaluate-round`,
which witnesses (`run-witnessed`) + gates (`self-mod-gate`) + ranks + persists
them as a HELD review queue. The value-vector dedup uses `core/eval-source`.

### Then, in order

2. **CEGIS refinement** (Smyth/Burst — C6, C12-C14): feed `run-witnessed` +
   `property-fuzzer` COUNTEREXAMPLES back to strengthen the example set and
   re-enumerate (angelic → analyze → strengthen → retry). Turns the strong
   verifier into the generator's driver.
3. **Canonical equivalence-reduction pruning** (Knuth-Bendix — C15-C16): prune
   redundant candidates *syntactically* before evaluation, using pnix-clj's
   existing canonical forms (§3 α-canonical + arith-proof polynomial + bool-proof
   truth-table) as the normal-form oracle (~80% pruning).

### Deliberately NOT first

- **Synquid refinement-type synthesis** — proven, but needs a hand-written spec
  (C10): cannot autonomously feed the loop. Revisit once we want *proven-by-
  construction* candidates from a spec source.
- **Library-learning / LLM (DreamCoder/babble/LILO)** — heuristic, needs a corpus
  or a model; a later multiplier, not the first honest brick.

## Prioritized TODO

1. `pnix-clj.generate` — bottom-up enumerator over a small pnix grammar (input
   vars + int/bool literals + `+ - *`, comparisons, `if`, safe builtins),
   value-vector evaluated via `core/eval-source`, **observational-equivalence
   dedup** (one representative per value-vector). Return matches (exprs whose
   value-vector equals the example outputs) as pnix source — labelled HEURISTIC.
2. `synthesize-and-propose` — wrap each match as a witnessable pnix program and
   feed `self-improve/evaluate-round` → ranked HELD proposals; report which
   matches are additionally `arith-proof`/`bool-proof` PROVEN.
3. CEGIS: on a match, use `property-fuzzer` to seek a counterexample; if found,
   add it to the example set and re-synthesize (Burst/Smyth loop).
4. Knuth-Bendix-style canonical pre-pruning using §3/arith/bool canonical forms.
5. (Later) refinement-type lane; (later) corpus/library-learning multiplier.
