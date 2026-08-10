# Differential testing — the self-growing interp==rustc discipline

Status: **ACTIVE (2026-07-04)**. Direction set by deep-research (95 agents, 3-0
verified). Gates: `fuzz-check` (DONE), `emi-check` (DONE). This is the answer to
"what do we build next": an automated, zero-dependency, no-proof-assistant
discipline that continuously deepens and GROWS the interp==rustc proof.

## Why this, and why now

rs-meta is mature: a broad Rust subset, interp==rustc translation validation over
a fixed 310/257 corpus, a stage ladder with a bounded self-host. The obvious
surface is built; the fixed corpus stops finding bugs. The highest payoff-to-risk
next step (deep-research #1) is **differential testing**: generate fresh,
well-defined Rust and cross-check the two independent implementations of one
semantics — the trusted interpreter floor and the rustc native tier. Any
divergence localizes a real bug. This discipline found 325+ bugs in GCC/LLVM
(Csmith, PLDI'11) and even wrong-code bugs in the *verified* compiler CompCert's
unverified trusted computing base — testing catches exactly what proof leaves
uncovered, so it comes BEFORE any proof-assistant investment (deep-research (e)).

## Findings
**FIVE real interpreter bugs found+fixed (see also IntLit below) via differential probing (2026-07-04).** (0) enum/Option/Result Debug: the interp printed `Option::Some(1)`/`E::A(5)` but rustc derive(Debug) prints `Some(1)`/`A(5)` (no type prefix) and Debug-renders the payload -- surfaced while adding iter min/max; fixed in Val debug_display. (1) Boundary
probing surfaced a genuine interpreter typeck gap: a function with a non-unit
return type whose body FALLS THROUGH to `()` (empty body `fn f() -> i64 { }`, or
a block ending in a `let`) was ACCEPTED by the interpreter (returning `()`) but
rustc REJECTS the type mismatch. Fixed in typeck.rs by a conservative
`block_falls_through` divergence analysis (control-flow constructs treated as
diverging so valid `{ return 5; }`-style functions are never wrongly rejected).
Zero regression: self-check 310, typeck 257, roundtrip 310, emit-tv 310, self-
host all still pass; the two bug cases were added to the NEGATIVE corpus (now
259) as a permanent regression guard. **Second real bug FIXED**: mixed-width integer arithmetic (e.g. `u32 + u64`,
`i32 + u32`) was accepted by the interpreter but rustc rejects the type mismatch
-- the interpreter used I64 as a flexible literal type and did not enforce that
two CONCRETE integer widths match. Fixed in type_binary for BOTH arithmetic AND comparisons (</<=/>/>=/==/!=) --
reject when both operands are integers of different types and neither is the
flexible I64 literal; zero regression, negative-corpus guards now 262.
Residual known gap: `i32 + i64` still passes because I64 is overloaded as both
the literal type AND the concrete i64 type, so the interpreter cannot tell an
i64 literal from an i64 value -- fixing that needs a distinct untyped-integer
type (documented, not yet done). This is the whole discipline working end to
end: find a divergence, fix the interpreter, prove no regression, lock it in.

**Overflow boundary (first real interp!=rustc divergence found).** Direct
probing (the payoff of the discipline) surfaced a genuine boundary: for a
compile-time-CONSTANT overflow like `let x = i64::MAX; x + 1`, the interpreter
WRAPS at runtime (release two-'s-complement) and prints i64::MIN, but rustc
REJECTS the program at compile time via its deny-by-default arithmetic-overflow
lint on const-evaluable expressions. This is interp-accepts / rustc-rejects =
'divergent'. It is a HELD const-overflow-lint feature -- the ONLY true accept/reject divergence found; other cases rustc rejects at compile time (div-by-zero, array-OOB, non-exhaustive match) the interpreter also rejects, at run time (verdict agrees, phase differs) -- it, not a wrap bug: the
interpreter has no compile-time const-overflow analysis. The generator therefore
does NOT emit plain overflow; it uses `wrapping_add`/`wrapping_mul` (well-defined,
interp==rustc) to cover the wrap surface. Documented boundary, tracked by the boundary-check gate (drift-detected: if the interp ever adds a const-overflow lint, the gate flags the boundary moved).

## Result so far
Across the 78-seed gate plus deep searches totalling 1650 programs (`fuzz-scale` 250+400+500+500),
the interpreter and rustc agree on EVERY generated program (0 divergences) over the full surface -- including the subtle integer /-and-% surface -- strong evidence the interpreter is faithful to rustc on
arithmetic/let/if/match/fn/struct/enum/tuple/Option/Vec/String/recursion/refs.
A divergence, if found, is the payoff: shrink it and mint it into the corpus.

## Interpreter-fidelity findings (2026-07-04 systematic boundary probing)

Direct probing of `interp accepts / rustc rejects` (real over-permissiveness)
and `interp rejects / rustc accepts` (subset gaps) found, this session:

FIXED (real over-permissiveness bugs, zero regression, corpus-guarded 257->262):
- fn return-type FALL-THROUGH: `fn f() -> i64 { }` accepted (returned ()) -- fixed.
- mixed-width integer ARITHMETIC: `u32 + u64` accepted -- fixed.
- mixed-width integer COMPARISON: `u32 < u64`, `u32 == u64` accepted -- fixed.

DOCUMENTED (held features / value-model limitations, NOT fixed):
- const overflow/underflow (`i64::MAX + 1`, `0u32 - 1`): interp wraps/computes,
  rustc rejects via the arithmetic-overflow lint -- a held const-lint boundary.
- use-after-move (`let t = s; s.len()`): interp accepts, rustc rejects -- held
  borrow checker.
- `i32 + i64`: still accepted because I64 is overloaded as literal AND concrete
  type; needs a distinct untyped-integer type.
- interp computes ALL integers as i64 (so `0u32 - 1` = -1, not u32 wrap): the
  value model is width-agnostic -- a deeper held limitation.
- bitwise `&`/`|`/shifts on integers: interp rejects (unsupported subset), rustc
  accepts -- a subset gap.


## Subset-completeness map (interp rejects / rustc accepts -- missing features)

Systematic probing of the OTHER direction found features rustc accepts but the
interpreter does not yet support (each a multi-layer addition, deliberately not
done -- no self-host demand, the core is already self-hostable):
- **unit struct `struct D;`**: DONE (worktree feat/unit-struct) -- parser accepts `struct D;`, typeck/interp construct the bare name `D` as a unit value (only for unit structs, not `struct D {}`), emit/sig preserve unit; corpus 310->311.
- **bitwise `&` / `|`**: DONE (worktree feat/bitops) -- added parse_bitor (looser than ^) + parse_bitand (tighter than ^) precedence levels; infix position disambiguates from prefix ref `&x`/closure `|x|`; signedness-safe (bitwise ops act on the stored i64 bits identically for signed/unsigned); mixed-width (u32 & u64) auto-rejected via the shared arithmetic arm.
- **shift `<<` / `>>`**: DONE (worktree feat/shifts) -- parse_shift level between cmp and add; `<<`=two Lt, `>>`=two Gt (lexer never fuses), and a CONSECUTIVE-pair check distinguishes a shift from a comparison; generics stay intact because `Vec<Vec<i64>>` closes in the TYPE parser (separate context). Shift amount may be a different integer type (`i64 << u32`), so a dedicated typeck arm bypasses the mixed-width check.
- OLD bitwise note (superseded): `&`/`|` / `<<` / `>>`** on integers: parse-level gap. Only `^`
  (BitXor) is supported. `&`/`|` need parser disambiguation (& = reference vs
  bit-and, | = closure vs bit-or); shifts need lexer+parser+ast+typeck+eval+
  emit+sig.
- **iterator adaptor chains**: DONE (worktree feat/ref-arith) -- the block was number-reference arithmetic (`&i64 * 2`), now auto-derefed in type_binary + int2.
- **trait DEFAULT methods** (`trait T { fn v(&self) -> i64 { 42 } }`): the parser
  SKIPS trait bodies entirely (parse-and-ignore), so a trait method with a
  default body is not modelled; dispatch works only through impl blocks.

SUPPORTED: iterator adaptors over .iter() (`v.iter().map(|x| x*2).collect()` / .sum()) via number-reference auto-deref in arithmetic -- rustc has impl Add for &T, so &i64 and vec-element refs now arithmeticate (added via worktree 2026-07-04; .into_iter()/range map-collect already worked), TRAIT DEFAULT methods (trait T { fn v(&self)->i64 { .. } } + impl T for A {}; added via worktree 2026-07-04 -- the parser now models trait bodies as TraitDef{methods(defaults), decls(signatures)}, impls track the trait name, and setup FLATTENS non-overridden defaults onto the implementing type so dispatch is unchanged; emit re-emits the trait + impl-for faithfully), stdlib methods str::repeat / Vec::contains / std::cmp::max,min (added via worktree 2026-07-04; Vec::sort DONE via copy-out + local insertion-sort + write-back), labeled break (`'outer: loop { break 'outer [v]; }`, added via worktree 2026-07-04 as a Labeled wrapper; loops re-propagate labelled breaks, the wrapper catches its own; continue 'label is NOT modelled), associated consts impl S { const N } + S::N access (added via worktree 2026-07-04; registered as globals under the qualified name, resolved through the EnumCtor path), where clauses + bare-call turbofish id::<T>() (added via worktree 2026-07-04; both dropped -- the interp is dynamically typed and does not resolve bounds), const fn + move closures (added via worktree 2026-07-04; both modifiers accepted, dropped on emit since the interpreter has no const-eval/borrow distinction), compound assignment <<= >>= &= |= ^= (added via worktree 2026-07-04, also fixed &= mapping to BitAnd + closed bool-bitwise gap), shifts << and >> (added via worktree 2026-07-04; all common Rust operators now covered), bitwise & and | (added via worktree 2026-07-04), integer .pow() (all int types, added via worktree 2026-07-04 -- saturating_add/abs deliberately NOT added because the interpreter stores all integers as i64 and cannot tell signed from unsigned at method dispatch, so their boundary semantics would diverge), fixed-size array type `[T; N]` (literal/repeat/indexing already worked; added the type annotation via worktree 2026-07-04), loop-as-expression (`let r = loop { break v; };`, added 2026-07-04 via worktree), tuple structs (`struct P(i64,i64);` + `P(a,b)` construction + `p.0` access, added 2026-07-04 via worktree), unit structs (`struct D;` + bare `D` construction, added 2026-07-04 via worktree), nested closures, String::push_str, tuple
indexing (3+), for-in-iter, iterator sum, and the whole fuzzer surface.

**Fuzzer surfaces extended to the 2026-07 subset expansions (2026-07-08):** 8 new
generated surfaces -- bitwise &|^ + shifts, compound assigns, labeled loop-as-
expression, integer pow/signum/rem_euclid, Vec sort/dedup/contains, .iter().map()
.sum() adaptor chains, [i64;N] arrays + single-key HashMap indexing (never
iterated: rustc HashMap order is random), and trait-default+override methods on
the generated struct. First 400-program run immediately caught a GENERATOR bug
that doubles as a boundary: `(7 - 4).signum()` -- the interp types untyped
integer literals as i64 so the method call evaluates, but rustc rejects with
E0689 (ambiguous numeric type; method resolution precedes literal fallback).
Generator now emits typed literals -- and the boundary itself was then CLOSED by the IntLit change (2026-07-08): unsuffixed integer literals (and literal-only arithmetic) type as an internal Type::IntLit that coerces to any concrete integer, collapses to i64 at bindings/containers, and REJECTS method calls exactly like rustc E0689; suffixed literals (5i64) now desugar at parse time to casts ((5 as i64)) so their receiver type is concrete. Residual accepted divergence: let x = 5; x.pow(2) (interp accepts via i64 collapse; rustc E0689 on the unconstrained var) -- var-level literalness needs real inference.

## The gates

### fuzz-check (differential testing — Csmith/PLDI'11)
A DETERMINISTIC generator (`check::fuzz_gen(seed)`, LCG-seeded) emits programs
from the evaluated subset, run through both tiers, cross-checked
interp-stdout == rustc-stdout. Reproducible: same seed → same program, so a
divergence is a stable, mintable corpus entry.

Current surface: bounded integer arithmetic incl. integer `/` and `%` by nonzero literals (truncation toward zero, sign-of-dividend remainder -- a classic divergence surface), `let` chains, `if/else` over all six comparison operators (==/!=/</>/<=/>=), plus a DEEP NESTED control-flow shape (if inside if/else) as a different distribution, `match` with literal arms + wildcard, `struct` definition +
literal + field access, `enum` definition + variant construction + match-on-
variant with binding, `tuple` literal + `.0`/`.1` indexing, `Option` Some/None + option-pattern match, `Vec` literal + indexing + `.len()`, `String` via format! + `.len()`, references `&v`/`*r`, bounded mutable `while`-loop accumulation (`let mut` + reassignment), closures (`|x| ...` capture + application), and helper `fn` definitions with parameters + calls. Runs
42 seeds per gate invocation.

### emi-check (metamorphic mutation — EMI/Orion PLDI'14)
A semantics-PRESERVING mutation must not change output. Observable code
(`println!`) is injected into a provably-DEAD `if false { … }` branch of a base
program; interp-stdout AND rustc-stdout must stay identical to the base. This
stresses lowering / dead-code handling, not just the front-end (EMI found 147
GCC/LLVM bugs, the majority silent miscompilations). Teeth: the same injection
under `if true` DOES change output, so the invariance is non-vacuous.

## Soundness by construction (deep-research #3)

For the differential oracle to be sound, every generated program must have ONE
well-defined meaning. The generator guarantees this BY CONSTRUCTION rather than
by post-hoc filtering:
- **No overflow**: values are additive combinations of single digits; `*` only
  multiplies two single-digit literals; helper calls pass single digits.
- **No division / no unsafe / no async**: outside the subset (held), never
  emitted.
- **No nondeterminism**: no HashMap iteration order, no time, no runtime
  randomness — the LCG runs only at GENERATION time; the emitted program is a
  fixed computation.

The real rs-meta soundness constraint is avoiding *nondeterministic /
unspecified* output, not the 191 C undefined behaviours (unsafe is held) — the
generator's job is to avoid nondeterminism, and it does so structurally.

## The self-host dividend

`check.rs` (which hosts the generator + gates) is part of the interp-able
all-source bundle: `source-bundle-check` runs rs-meta's OWN interpreter over it
and requires interp==rustc. Implementing these gates, that discipline caught
THREE real self-host bugs before they shipped:
1. `parse::<u64>()` — interp turbofish supports only `i64`/`f64`.
2. `&u64` method dispatch — `s.wrapping_add(..)` on a `&u64` loop variable.
3. `String::replace` / `replacen` — unsupported by the interpreter.

**Discipline: run `source-bundle-check` BEFORE committing any rs-meta change.**
check.rs must remain interpretable by rs-meta itself.

## Roadmap (ranked)

1. **Extend the generator surface** (coverage-driven): [struct + enum + match-
   on-variant DONE 2026-07-04]; [tuple + Option + Vec + String +
   bounded recursion + references DONE]; the generator now spans the major
   std reimplementations, the call stack, and reference semantics; scale search: fuzz-scale <n> DONE — each a new region where an interp-vs-rustc
   divergence can hide.
2. **Divergence shrinking** (delta debugging) [engine DONE 2026-07-04, shrink-
   check]: a conservative statement-level ddmin drops removable let bindings
   while both tiers keep the output; the predicate swaps to divergence-
   preservation to minimize a real divergence into a small corpus reproducer.
3. **Corpus auto-mint** [DONE 2026-07-04]: fuzz-mint writes verified generated
   programs to proofs/fuzz-corpus.tsv; fuzz-corpus-check re-runs the interpreter
   and requires the frozen output -- a monotone regression suite.
4. **Self-host blocker audit** (deep-research open question): determine which
   currently-held features rs-meta's OWN compiler source actually uses — this,
   not a feature wishlist, defines the true remaining self-hosting work.

## Deliberately NOT next (deep-research findings)

- **Borrow checker**: mrustc bootstraps real rustc while HOLDING the borrow
  checker — it is NOT required to self-host. Hold it as a no-op/witness unless
  the 257-negative corpus demands rs-meta REJECT ill-borrowed programs (open
  question).
- **Chalk-style trait solver**: a research frontier (experimental even for the
  Rust core team). If a held-feature lift is wanted, `macro_rules!`
  matcher/transcriber is more tractable (it reuses the existing parser via
  fragment specifiers).

## Primary sources
Csmith (Yang/Chen/Eide/Regehr, PLDI'11); EMI (Le/Afshari/Su, PLDI'14); YARPGen
(OOPSLA'20); mrustc (thepowersgang); rustc bootstrap (rustc-dev-guide); Diverse
Double-Compiling (Wheeler).
