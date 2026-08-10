# Meta-Circular Evidence-Store SPINE — research-verified roadmap

The detailed, proven-technique plan for building the checklist's evidence-store
spine (§3/5/8/9/10/13.1/6.6-6.7/15) on the feat branch. Verified by
`/deep-research` (95 agents, adversarial verification, 2026-07-04) against
peer-reviewed sources. See `docs/META_CIRCULAR_AUDIT.md` for what exists vs what
this plan builds; gaps are also machine-registered in `resources/pnix_clj/roadmap.edn`.

## 0. THE load-bearing principle (get this right or nothing is sound)

**A content hash is a PROPOSE filter, never a proof of equivalence.** This is
not a stylistic note — it is proven:

- Hashing modulo α-equivalence (Maziarz, Ellis, Lawrence, Fitzgibbon & Peyton
  Jones, **PLDI 2021**, arXiv:2105.02856) GUARANTEES α-equivalent subtrees hash
  identically (one direction, deterministic), but the converse (same-hash ⇒
  α-equivalent) holds only with LOW collision probability — Lemma 6.6 bounds it
  at `(|a|+|b|)/2^b`, and even that only under the random-oracle model; the
  extension to a real seeded hash is asserted *without proof* (Khuong,
  pvk.ca/Blog/2022/12/29). 
- Empirically: Nix over 709,816 packages / 17 revisions (Malka, Zacchiroli &
  Zimmermann 2025, arXiv:2501.15919) reaches only **69–91%** bitwise
  reproducibility despite input-addressing — input-hash equality does NOT
  guarantee identical output; ~15% of failures are embedded build dates.

**⇒ Rule for every spine capability:** a hash-hit LICENSES a fast path (skip
eval / dedup / cache) but must be CONFIRMED by an exact structural / α check
(and, for determinism, by an actual re-run) before it is treated as truth. This
is the pnix-clj constitution's "proven-vs-heuristic boundary" made concrete.

## 1. Build ORDER (dependency-respecting — do not reorder)

```
§3 term store  →  §5 event log  →  §10 + §13.1 reflection snapshots
     →  §8 snapshot determinism  →  §9 purity-as-events
     →  §17 search  →  §6.6-6.7 mirror drift  →  §15 witness (capstone)
```
Rationale: everything keys off §3 term hashes; §8 resolve-term needs §3 + the
§10/§13.1 snapshots it pins; §9 needs §3(hash)+§5(events)+§8(snapshot); §17
needs §3 open-term summaries; §6.6-6.7 needs §5; §15 integrates all.

## 2. Per-capability plan (technique · reference · pitfall · sketch)

### §3 — Canonicalization + content-addressed TERM store  [FOUNDATION]
- **Technique**: canonicalize FIRST, hash SECOND, on a term GRAPH.
  1. order-independent attrset/let bindings; letrec via first-order term graph
     → **bisimulation collapse** → read back (Grabmayer & Rochel, *Maximal
     Sharing in the Lambda Calculus with letrec*, **ICFP 2014**,
     arXiv:1401.1460). Decides UNFOLDING-equivalence (structural sharing),
     canonical up to isomorphism — NOT β/η. Hash-cons over the collapsed graph.
  2. positional / de-Bruijn binders + dependencies-by-hash (**Unison** recipe:
     "each definition identified by a hash of its syntax tree"; named args →
     positional refs; deps → their hashes; ASTs stored keyed by hash, not text).
  3. α-aware content hash (Maziarz PLDI'21) → `term-key`.
- **Pitfall**: the shortcut "de Bruijn + sha256 the Merkle tree" is UNSOUND for
  OPEN subterms — false negatives (same subterm at different binder depths) AND
  false positives (distinct open subterms identical in isolation) — Blaauwbroek,
  Olšák & Geuvers, *Hashing Modulo Context-Sensitive α-Equivalence* (2024,
  arXiv:2401.02948). Closed WHOLE-term α-equivalence via de Bruijn IS sound; the
  unsoundness is specific to open subterms / similarity.
- **Sketch (pnix-clj)**: `pnix-clj.cas` — `canonical-form` (parser AST →
  collapsed graph), `term-hash`/`term-key` (α-aware), `put-term!`/`get-term`;
  on a hash-hit run `alpha-equivalent?` (graph isomorphism on the collapsed
  form) before dedup. Reuse `hash.clj`. Reject mutable/identity-bearing host
  objects up front (shared guard with §5).
- **§3c open-term summary**: anonymous skeleton + free-variable summary +
  structural distance via context-sensitive hashing (Blaauwbroek 2024), for §17.

### §5 — Append-only EVENT log (verifying traces)
- **Technique**: this is a *verifying trace* (Mokhov, Mitchell & Peyton Jones,
  *Build Systems à la Carte*, **ICFP 2018 / JFP 2020**): store ONLY hashes
  (compact; early cutoff; dynamic deps). Scheduler (topological/restarting/
  suspending) is ORTHOGONAL to the rebuilder — keep them separable. VALUES stay
  in cached-eval/§3 keyed by the same hashes (do NOT duplicate values into the
  log). `open-store`, `append!`, event hash, event seq, index-by-kind/hash/field,
  pointer-movement-as-event.
- **Pitfall (hermeticity contamination — reject at `append!`)**: Bazel
  hermeticity enumerates exactly what must never enter the log — build IDs &
  timestamps (`java.util.Date`, `System.currentTimeMillis`), host-varying
  binaries / absolute paths / system compilers, and writes into the source tree.
  Pure-EDN payload discipline; a `contamination?` predicate at the `append!`
  boundary rejects identity-bearing/runtime objects.
- **Sketch**: `pnix-clj.store` (append-only, EDN-only) + `pnix-clj.evidence`
  (event schema); reuse receipt shapes.

### §10 + §13.1 — Reflection snapshots
- all-ns / ns-publics / var-root / var-meta / dynamic-binding SNAPSHOTS + diff +
  witness; classpath + JVM-version snapshot/hash. These are the host-varying
  inputs §8 must PIN (Bazel: pin host-varying binaries). Deterministic
  serialization (sorted, no identity). `pnix-clj.meta.namespace` / `.var` /
  `.classpath` / `.jvm`.

### §8 — Snapshot determinism
- **Technique**: content-addressed skip is sound ONLY under a determinism
  assumption keyed on terminal inputs (deep constructive traces, Nix/Buck's
  class; *Build Systems à la Carte* §4.2.4 — the **Frankenbuild** example proves
  n≥2 nondeterminism can violate correctness). Bazel integration pattern: a CAS
  (values by content hash) + command history annotated with observed dep hashes
  lets the engine predict the result hash and bypass eval.
- **Sketch**: `:snapshot/id` = hash of (evaluator-version ⊕ symbol-version ⊕ the
  §10/§13.1 reflection snapshots); `assert-snapshot-runtime-match!` FAILS CLOSED
  on mismatch; `resolve-term` under a snapshot. Futamura residual bytecode is
  content-addressed by (source term-hash + snapshot-id).

### §9 — Purity / determinism as EVENTS
- **Technique**: do NOT enforce determinism statically. Enforce it as EVENTS
  caught at replay/fork — a violation runs fine once and is caught later as a
  divergence PINNED TO THE FIRST event that fails to reproduce (Nakajima 2026,
  *The Log is the Agent*; record-replay divergence-detection is independently
  established). ★Verified caveats: a deterministic fold over the log is NOT
  automatically byte-identical, and content-addressed caching of a
  nondeterministic effect does NOT make replay deterministic — effects must be
  RECORDED and determinism WITNESSED by actual re-run, never assumed.
- **Sketch**: repeated-eval determinism (same source+term-hash+snapshot+
  runtime-version → one result hash by ACTUAL re-run + diff); mutation isolation
  (old snapshot result immutable after later commits); threaded stress;
  nondeterminism → violation evidence pinned to the first-divergent event (= the
  §15 witness anchor) + fail closed; scan result payloads for date/timestamp/
  build-ID patterns as detectors.

### §17 — Search
- content-address lookup + event index + structural-similarity (open-term
  skeleton distance from §3c). `pnix-clj.search`.

### §6.6-6.7 — Mirror drift + chain convergence
- mirror DRIFT events + repeated-run chain-convergence stability, recorded as §5
  events. (Cross-mirror per-run verdict already exists in `mirror.clj`.)

### §15 — Witness schema + admission lattice  [CAPSTONE]
- explicit witness (`:witness/id`, input/output/term/result hash, runtime/
  compiler/evaluator version, snapshot id, stage, status, evidence events) +
  admission/status lattice (held/candidate/admitted/rejected/evidence/failed/ok),
  in-toto / SLSA-shaped. Integrates all: cross-mirror tower verdicts recorded as
  §5 events feed §15; the first-divergent-event from §9 is the witness anchor.

## 3. Paste-ready TODO sequence (research-ordered)

1. **§3a canonicalizer** — AST → order-independent attrset/let; letrec via
   term-graph + bisimulation collapse (Grabmayer-Rochel); reject mutable/
   identity host objects; positional binders + dependency-by-hash (Unison).
2. **§3b α-aware content hash** (Maziarz) → term-key → put/get; hash-hit =
   PROPOSE only → confirm with graph-isomorphism / α-check on the collapsed form.
3. **§3c open-term summary** (skeleton + free-var summary + structural distance)
   via context-sensitive hashing (Blaauwbroek), NOT raw de-Bruijn Merkle.
4. **§5 event log** — open-store/append!/event-hash/seq/index/pointer-as-event;
   pure-EDN discipline; CAS guard rejecting build-IDs/timestamps/host-varying/
   mutable (hermeticity classes).
5. **§10 + §13.1** — ns/var/meta/dynamic-binding snapshots + diff + witness;
   classpath + JVM-version snapshot/hash.
6. **§8 snapshot** — :snapshot/id + evaluator/symbol-version binding the §10/
   §13.1 snapshots; runtime-match gate FAIL-CLOSED; resolve-term under snapshot.
7. **§9 purity-as-events** — repeated-eval determinism by actual re-run+diff;
   mutation isolation; threaded stress; nondeterminism → evidence pinned to
   first-divergent event + fail closed; payload date/timestamp/build-ID scan.
8. **§17 search** — content-address + event index + structural similarity.
9. **§6.6-6.7** — mirror drift events + chain-convergence stability.
10. **§15 witness + admission lattice** (capstone) — full schema, in-toto/SLSA
    shaped; residual bytecode content-addressed by (term-hash + snapshot-id);
    cross-mirror verdicts as §5 events feeding §15.

## 4. Prerequisite owner decision (unchanged)

`origin/main` already implements much of §3/5/8/9/17 (`cas.clj`/`store.clj`/
`term.clj`/`stage.clj`/`purity.clj`/`resolve.clj`/`evidence.clj`/`search.clj`).
Choose before building: **(A)** port main's spine onto feat, **(B)** keep the
split, **(C)** rebuild minimal on feat in the rewrite's style. If (A)/(C), THIS
plan is the correctness spec the ported/rebuilt code must meet (esp. §0: hash =
propose-filter, confirm exactly).
