# Self-hosting — 실제로 막는 것 (그리고 막지 않는 것)

Status: **AUDITED (2026-07-04)**. Deep-research open question 답변: *rs-meta의
OWN compiler source가 실제로 쓰는 currently-held Rust features는?*
Gate: `selfhost-audit-check`.

## The finding

rs-meta의 **evaluator core** — `lexer.rs`, `ast.rs`, `parser.rs`, `typeck.rs`,
`interp.rs`, `sig.rs`, `hash.rs` — 는 `source-bundle-check`가
`interp == rustc`를 증명하는 대상. 그 core는 **held-feature blocker zero**:

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

Naive grep이 찾는 `macro_rules!` / `unsafe` / `dyn` 언급은 `check.rs` —
boundary-report *test data*와 doc comments, 즉 문자열, 코드가 아님. Interpreted
evaluator core에 없음.

## Why this matters: NO held feature blocks the core self-host

**mrustc**와 미러 — 최근 real rustc를 bootstrap하면서 borrow checker를
의도적으로 hold. Borrow checker는 self-host에 필수 아님. Audit가 그 결과를
확장: rs-meta core에 대해 held feature *어느 것도* 필수가 아님. rs-meta는 이미
evaluator core를 self-host (`source-bundle-check`가 증명), held feature lift가
그 바를 움직이지 않음.

Borrow checker는 mrustc stance("trust that the input is valid; a miscompilation
is our bug") 아래 no-op/witness로 held 유지. 여기서 sound: rs-meta 자체
source가 이미 real rustc로 borrow-validated. One caveat (deep-research):
negative corpus — `typeck-check` green 유지를 위해 rs-meta가 rustc가 reject하는
프로그램을 REJECT해야 함 — 그러나 그것은 *type* rejections, borrow rejections가
아니므로 borrow checker도 필요 없음.

## So what IS the remaining self-host work?

Held language features가 아님. Real axes:

1. **Full-chain cost.** `stage3-full-chain` (인터프리터가 전체 all-source
   evaluator를 full corpus 위에 실행)는 DONE이지만 ~2100s budget-gated; cost는
   meta-level self-interpretation 고유 (outer evaluator load dominates)이므로
   default-run이 아니라 budget-gated 유지.
2. **Widening the self-hosted set beyond the core.** Proof/harness layer
   (`check.rs`, `main.rs`, `native.rs`)는 `std::fs`, `HashMap`, process
   spawning, test strings의 `dyn` 사용 — *core*가 필요 없는 표면. *Whole
   binary* self-host (evaluator core만이 아님)는 인터프리터가 그것들을
   cover해야 하지만 coverage task이지 held-feature lift가 아님.
3. **Keeping the core held-feature-free** as it grows — `selfhost-audit-check`가
   강제, core file이 `macro_rules!`, `async`, `unsafe`, 또는 `trait`
   definition을 얻으면 fail (genuine new self-host blocker).

## Consequence for the roadmap

Differential-testing discipline (`docs/differential-testing.md`)이 올바른
다음 작업인 이유: self-hosting question이 *settled* — core already
self-hostable, held feature blocks 없음, 노력은 held feature lift가 아니라
`interp == rustc` proof (coverage) 성장·심화. Downstream 이유로 held-feature
lift가 필요하면 `macro_rules!`가 tractable (deep-research (b)); trait solver는
self-host에 필요 없고 research frontier.
