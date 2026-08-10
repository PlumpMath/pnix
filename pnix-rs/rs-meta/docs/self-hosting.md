# Self-hosting — what actually blocks it (and what doesn't)

Status: **AUDITED (2026-07-04)**. Answers the deep-research open question: *which
currently-held Rust features does rs-meta's OWN compiler source actually use?*
Gate: `selfhost-audit-check`.

## The finding

rs-meta's **evaluator core** — `lexer.rs`, `ast.rs`, `parser.rs`, `typeck.rs`,
`interp.rs`, `sig.rs`, `hash.rs` — is what `source-bundle-check` proves
`interp == rustc` on. That core uses **zero held-feature blockers**:

| held feature   | uses in the core | verdict |
| -------------- | ---------------- | ------- |
| `macro_rules!` | 0 | not a blocker |
| procedural / derive macros | 0 | not a blocker |
| `async` / `await` | 0 | not a blocker |
| `unsafe` | 0 | not a blocker |
| `trait` definitions | 0 (rs-meta defines **no** traits) | not a blocker |
| associated types | 0 (only top-level `type` aliases, which are supported) | not a blocker |
| `dyn Trait` | 0 in the core | not a blocker |
| `const` generics | 0 | not a blocker |
| lifetimes (`<'a>`, `&'a`, `'static`) | present, but parse-and-ignored | not a blocker |
| full borrow checker | not implemented (held) | not a blocker (see below) |

The mentions of `macro_rules!` / `unsafe` / `dyn` that a naive grep finds live in
`check.rs` — inside boundary-report *test data* and doc comments, i.e. strings,
not code. They are not in the interpreted evaluator core.

## Why this matters: NO held feature blocks the core self-host

This mirrors **mrustc**, which bootstraps a real, recent rustc while
deliberately holding the borrow checker — the borrow checker is not required to
self-host. The audit extends that result: for rs-meta's core, *none* of the held
features are required either. rs-meta already self-hosts its evaluator core
(`source-bundle-check` is the proof), and lifting a held feature would NOT move
that bar.

The borrow checker stays held as a no-op/witness under the mrustc stance ("trust
that the input is valid; a miscompilation is our bug"). This is sound here
because rs-meta's own source was already borrow-validated by real rustc. The one
caveat (deep-research) is the negative corpus: rs-meta must REJECT the programs
rustc rejects to keep `typeck-check` green — but those are *type* rejections, not
borrow rejections, so no borrow checker is needed for that either.

## So what IS the remaining self-host work?

Not held language features. The real axes are:

1. **Full-chain cost.** `stage3-full-chain` (the interpreter running the whole
   all-source evaluator over the full corpus) is DONE but budget-gated at
   ~2100s; the cost is inherent to meta-level self-interpretation (the outer
   evaluator load dominates), so it stays budget-gated rather than default-run.
2. **Widening the self-hosted set beyond the core.** The proof/harness layer
   (`check.rs`, `main.rs`, `native.rs`) uses `std::fs`, `HashMap`, process
   spawning, and `dyn` in test strings — surfaces the *core* does not need. Self-
   hosting the *whole binary* (not just the evaluator core) would require the
   interpreter to cover those, but that is a coverage task, not a held-feature
   lift.
3. **Keeping the core held-feature-free** as it grows — enforced by
   `selfhost-audit-check`, which fails if a core file gains a `macro_rules!`,
   `async`, `unsafe`, or `trait` definition (a genuine new self-host blocker).

## Consequence for the roadmap

The differential-testing discipline (`docs/differential-testing.md`) is the right
next work precisely because the self-hosting question is *settled*: the core is
already self-hostable, no held feature blocks it, so effort belongs in growing
and deepening the `interp == rustc` proof (coverage), not in lifting held
features. If a held-feature lift is ever wanted for downstream reasons,
`macro_rules!` is the tractable one (deep-research (b)); a trait solver is not
needed to self-host and is a research frontier.
