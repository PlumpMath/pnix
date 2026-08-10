# rec / let Forward-Reference Fixture Taxonomy (audit, no behavior change)

Status: **audit + evidence only.** No evaluator/parser/runtime/fixture behavior was
changed to produce this document. It prepares the *supervised* multi-lane fix for
`rec` forward references that was attempted and reverted (see todo.md and the
`docs(pnix-clj): record rec-forward-ref as supervised multi-lane work` commit).

Goal per owner direction: classify the fixture space, capture **evidence/receipts**
for why the current `mirror-error/rec-forward-reference` fixture must be
reclassified, and do NOT silently flip a negative fixture into a success fixture.

---

## 1. The four lanes

`pnix-clj.core/run-source` runs every source through four lanes and records a
`:lane-summary`:

```
pnix-clj-evaluator            the direct semantic evaluator (this is the runtime)
pnix-clj-lowering-clj-meta    lower pnix AST -> clj-meta form -> eval on the host
clojure-stage15-mirror        the stage15 mirror over the clj-meta lowering
px-runtime-pnix-mirror        the internal .px runtime mirror
```

The `mirror-error` corpus (`resources/pnix_clj/mirror_error/cases.edn`) "accepts"
a row when the lanes **agree on an error boundary**. Its own lineage note says:

> "Accepted here means the Clojure evaluator and internal .px runtime mirror agree
> on the error boundary."

That is an *agreement* claim, not a *Nix-semantics* claim. This distinction is the
whole point below.

---

## 2. Evidence: 4-lane verdicts (captured via run-source, read-only)

Baseline — simple constructs are fully supported on every lane:

| source | evaluator | clj-meta | stage15-mirror | px-runtime |
|---|---|---|---|---|
| `1 + 1` | ok | ok | ok | ok |
| `{ a = 1; }` | ok | ok | ok | ok |
| `{ a = 1; }.a` | ok | ok | ok | ok |
| `let a = 1; in a` | ok | ok | ok | ok |
| `rec { a = 1; }.a` | ok | ok | ok | ok |
| `if true then 1 else 2` | ok | ok | ok | ok |

Forward references and cycles:

| source | evaluator | clj-meta | px-runtime | Nix-correct |
|---|---|---|---|---|
| `let a = b + 1; b = 10; in a` | **ok = 11** | held (clj-meta-eval-failed) | held (px-runtime-run-error) | **11** |
| `let a = a + 1; in a` | held :infinite-recursion | held | held | infinite-recursion |
| `let a = z + 1; in a` | held :unbound-var | held | held | unbound-var |
| `rec { x = y; y = 1; }.x` | **held :unbound-var** | held | held | **1 (ok)** |
| `rec { a = a + 1; }.a` | held :unbound-var | held | held | infinite-recursion |
| `rec { a = z + 1; }.a` | held :unbound-var | held | held | unbound-var |

(`clojure-stage15-mirror` tracks `clj-meta` for these rows and is omitted for width.)

---

## 3. Findings

### F1. In Nix, `let` and `rec` are the SAME recursion; our evaluator only did `let`.
`let ... in` and `rec { ... }` both introduce one mutually-recursive scope. Our
evaluator's `eval-let` uses knot-tied memoized thunks and is fully correct
(forward=11, cycle=infinite-recursion, unbound=unbound-var). `eval-attrs` (rec)
builds its environment *incrementally*, so a forward name is simply never in
scope — every rec forward/cycle case collapses to `:unbound-var`. So:

- `rec { x = y; y = 1; }.x` → should be **1**, is `:unbound-var` (missing forward ref).
- `rec { a = a + 1; }.a` → should be **infinite-recursion**, is `:unbound-var`
  (the cycle can't even be detected because the name isn't bound).

The fix is to give `eval-attrs` the same knot-tied-thunk scope as `eval-let`
(the reverted patch did exactly this and passed direct unit tests).

### F2. The clj-meta / px-runtime lanes are a FRONTIER for forward references, not a semantic judge of them.
Those lanes return `ok` for `let a = 1; in a` and `rec { a = 1; }.a`, but `held`
for `let a = b + 1; b = 10; in a` — a perfectly valid expression the evaluator
computes as `11`. They lower/execute let/rec assuming **sequential** bindings, so
any forward reference fails there regardless of whether it is semantically valid.
Their `held` on `rec { x = y; y = 1; }.x` therefore tells us **nothing** about
whether that expression is an error — they would `held` on the valid
`rec { x = 1; y = x; }.y`-style forward case too.

### F3. The mirror-error "agreement" on rec-forward-reference is SPURIOUS.
For `rec { x = y; y = 1; }.x` all lanes are currently `held`, so the row is
"accepted (agree)". But the agreement is coincidental:
- evaluator held = the **rec forward-ref bug** (`:unbound-var`),
- clj-meta/px held = the **forward-ref frontier** (F2).
They agree on the *word* held, not on any error semantics. Freeze that agreement
and you have frozen the evaluator bug as "expected".

### F4. The evaluator-ahead divergence is ALREADY an accepted state — for `let`.
`let a = b + 1; b = 10; in a` today is `evaluator=ok 11` while clj-meta/px are
`held`. That divergence exists right now and **no fixture flags it**. Fixing
`rec { x = y; y = 1; }.x` to `evaluator=ok 1` produces the *exact same shape* of
divergence — evaluator ahead of a known frontier — which is already tolerated for
`let`. The only thing that makes rec special is the mirror-error fixture that
captured the pre-fix buggy agreement.

### F5. Owner-proposed `let-forward => unbound-var/error` does NOT match Nix or our code.
In the taxonomy sketch, `let a = b + 1; b = 10; in a` was listed as
`unbound-var/error`. Evidence: Nix evaluates it to `11`, and our evaluator already
returns `ok 11`. `let` is recursive in Nix exactly like `rec`. Recommend the
taxonomy treat `let-forward` and `rec-forward` as the SAME class (`*-forward-ok`),
which is what the evidence supports. Flagged here rather than silently implemented.

---

## 4. Proposed taxonomy (target verdicts, all four lanes)

| class | example | evaluator | frontier lanes (clj-meta/mirror/px) |
|---|---|---|---|
| `forward-ok` | `rec { x = y; y = 1; }.x`, `let a = b+1; b=10; in a` | **ok** | held @ frontier (until lanes support forward refs) |
| `cycle-error` | `rec { a = a + 1; }.a`, `let a = a+1; in a` | held **:infinite-recursion** | held @ frontier |
| `unbound-error` | `rec { a = z + 1; }.a`, `let a = z+1; in a` | held **:unbound-var** | held @ frontier |

Note: `forward-ok` and `cycle-error` are NOT "all lanes agree on an error". They are
"evaluator gives the Nix verdict; the other lanes are a declared forward-ref
frontier". They therefore belong in a **forward-reference corpus with an explicit
frontier marker**, NOT in the error-agreement `mirror-error` corpus.

---

## 5. Receipts: what must change and why (do NOT do silently)

1. **`mirror-error/rec-forward-reference` is mis-filed.** Its `:source`
   `rec { x = y; y = 1; }.x` is a `forward-ok` case (Nix = 1), captured as an
   error only because of F1 (evaluator bug) + F3 (spurious agreement). It must
   leave the mirror-error corpus. Receipt = F1–F4 above. Removing/moving it is a
   fixture reclassification with a written reason, not a "flip negative to green".

2. **A `rec-cycle` error fixture is missing / wrong-reasoned.** There is no fixture
   asserting `rec { a = a + 1; }.a` → infinite-recursion; today it is
   `:unbound-var` (F1). After the fix it should join a cycle-error corpus with
   reason `:infinite-recursion` (mirroring the existing `let a = a + 1; in a`
   behavior).

3. **A `rec-unbound` fixture is legitimate.** `rec { a = z + 1; }.a` → `:unbound-var`
   is correct on the evaluator and can stay as a genuine error case (with the
   frontier lanes marked, per F2).

4. **Frontier lanes need an explicit marker.** Because clj-meta/px-runtime `held`
   on all forward references (valid or not), forward-reference fixtures should
   record those lanes as `:frontier` (known-unsupported), so a future lane upgrade
   is a deliberate, evidenced change rather than a silent green.

---

## 6. Next (supervised) steps — not done here

1. Reclassify fixtures per §5 (with the receipts above) — owner review.
2. Apply the knot-tied `eval-attrs` fix (the reverted patch) so the evaluator lane
   gives `forward-ok = ok`, `cycle = infinite-recursion`.
3. Decide the frontier policy: either mark clj-meta/px-runtime as a declared
   forward-ref frontier, or extend those lanes to support recursive bindings so all
   four lanes reach `ok` (larger, separate work).
4. Only then flip the gate expectations, keeping the receipt trail.

---

## 7. Operator strictness (separate track) — audit-only, unchanged behavior

Per owner direction, `if <non-bool>`, `!<non-bool>`, `assert <non-bool>`, and
`+` string coercion (`1 + "a"`) are **left lenient**. The plan is a
`--strict-audit` mode that records what *would* fail under strict semantics as
evidence/warnings, with no behavior change, before any phased switch. Not started
here; noted so it is not conflated with the rec work.
