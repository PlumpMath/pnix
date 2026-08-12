# rs-meta TODO / 연속성 노트 (continuation note)

기준일: 2026-06-30 KST

재개 시 여기부터 읽고, 검증 명령으로 현재 상태를 확인한 뒤 다음 슬라이스를 잇는다.

---

## 0. 정체성 (한 줄)

**rs-meta = 독립적인 Rust meta-circular stage15~N 컴파일러/평가기.**

- **meta-circular over Rust**: Rust로 작성되어 **Rust를 평가/컴파일**한다 (자기가
  쓰인 언어를 평가). rustc self-hosting을 staged self-rebuild로 stage15~N까지
  밀어붙이는 것이 목표.
- **대상 언어 = Rust** (Lisp/커널/IR 아님). 평가 대상은 Rust subset이고, 그 subset을
  stage마다 넓혀 결국 **평가기 자신의 소스가 subset 안에 들도록** 한다 = 진짜 meta-circular.
- **evcxr-0.21.1** (`../evcxr-0.21.1`) = 그 *평가기* 형태의 레퍼런스. "meta-circular
  컴파일러가 만들어진 이후 rust mirror일 때의 interpreter." 기법만 차용, **미링크**.
- **rust-1.96.0** (`../rust-1.96.0`) = rustc 소스 스냅샷 / 툴체인 기판.

**무관한 것 (정체성에서 제외):**
- clojure/hy: `clj-meta`/`hy-meta`는 staged meta-circular bootstrap을 *어떻게
  구조화하는지*의 구조 레퍼런스일 뿐. 형제도 타워도 아님.
- pnix: 의존·지식·레이어 없음. 완전히 별개/다운스트림. rs-meta는 pnix를 모른다.

## 1. 헌법

1. **zero crates.io dependency.** std만. `[dependencies]` 금지. 해싱/파싱 전부
   in-house. **rustc는 툴체인**(std::process로 호출)이지 라이브러리 의존성이 아님.
   evcxr 크레이트는 레퍼런스, 미링크.
2. **정직(honesty).** DONE=실제로 돌고 검증됨, TODO=다음 슬라이스, HELD=아직
   주장 안 함. 과대주장 금지(Trusting-Trust/형식증명은 정직히 held; B==C 고정점은
   reproducibility 증거지 backdoor 방어 아님).
3. **TV(translation validation) green 유지.** 모든 실행 경로(in-Rust 인터프리터,
   rustc native tier, 추후 self-host)는 동일 Rust 프로그램에 동일 출력. native tier가
   못 다루는 형태는 명시 거부(silently wrong 금지).

## Current Remaining Work (verified 2026-08-11)

이 섹션은 2026-08-11 세션(commits `272ccef`, `24d3326`, `ae170e0`, 그리고 이후
같은 날 진행된 `interp.rs` Vec/chars 수정 커밋)이 끝난 시점의 정확한 "실제로 남은
일" 요약이다. 아래 §2 이후의 상세 로드맵/진행 로그는 그대로 보존하되(historical),
최신 상태를 빠르게 파악하려면 이 섹션부터 읽는다. 전체 서사는 `STATUS.md`(특히
"Correction (verified this session, 2026-08-11)"과 "Trusting-Trust defense
roadmap" 절)를 참고. 아래 항목들은 이 세션에서 라이브로 재검증됨 (`cargo build`,
`self-check` 407/407, `tv-check` 407/407, `typeck-check` 272/272,
`independent-mini-backend-check` 9/9, `source-ast-check` 16/16,
`source-bundle-check` PASS, `stage2-chain-check` PASS,
`stage9-aggregate-replay-check` PASS — all re-run live during this audit, not just
read from docs). Item 1 below (previously the highest-priority open item) is now
DONE — see its entry for the fix. A full `bin/rs-meta-gate check` run after that
fix found exactly one unrelated pre-existing failure (`trait-boundary-check`,
item 7 below, confirmed via `git worktree` to predate item 1's fix) — everything
else in the full aggregate is green.

**2026-08-12/13 update:** items 2, 6, and 7 are now ALL done too — see each
item's own entry. Only item 3 (mrustc, environment-gated) and item 5 (HELD
non-goal) remain open, and neither is actionable from this machine.

### 1. `interp.rs` runtime dispatch bug — Vec/chars — FIXED (2026-08-11, later pass)

- **State: DONE.** `stage2-chain-check` now passes, and so does
  `stage9-aggregate-replay-check` (the check this whole investigation was
  originally trying to get past — `aggregate-proof-matrix -> success`). Do not
  re-flag this as open.
- **Root cause:** `interp.rs`'s untyped `.collect()` (no turbofish, no static
  types available at runtime) guesses String vs `Vec<char>` from the collected
  items, and by explicit design an *empty* char iterator guesses `Vec`. A
  symmetric coercion already existed for the opposite mismatch
  (`coerce_string_to_char_vec`, used by `coerce_let_value` when a `let
  x: Vec<char> = ...` landed as a String) but not for this direction. This
  bit `parser.rs`'s `normalize_format_args`, whose `let inner: String =
  chars[i + 1..j].iter().collect();` extracts the (often-empty, e.g. every
  bare `{}`) text between a format placeholder's braces — landing as an empty
  `Val::Vec` instead of `Val::String`, so a later `.chars()` call on it failed
  with "interp: unsupported Vec method chars".
- **Fix:** added `coerce_char_vec_to_string` (mirrors
  `coerce_string_to_char_vec`) plus a matching `(Type::Named("String"),
  Val::Vec)` arm in `coerce_let_value`, in `src/interp.rs`.
- **Second gap found once unblocked:** `interp.rs`'s own `format_println`
  (the *runtime* renderer for interpreted `println!`/`format!` calls) had no
  case for `{:.*}` (dynamic precision) — only the parse-time arg-counting
  (`normalize_format_args`) and typeck (`format_placeholder_kinds`) sides had
  been taught about it earlier this session. This mattered because
  `interp.rs`'s *own source* calls `format!("{:.*}", precision, f)` in its
  fixed-precision branch, and self-interpreting `interp.rs` runs that exact
  line as interpreted code. Fixed by adding a `{:.*}` branch to
  `format_println` that consumes two args (precision, then value), matching
  the parse/typeck-time behavior.
- **Verified:** self-check 407/407, tv-check 407/407, typeck-check 272/272,
  independent-mini-backend-check 9/9, source-ast-check 16/16,
  source-bundle-check PASS, stage2-chain-check PASS, stage9-aggregate-replay-check
  PASS, ast-diff-check 4/4, rust-ir-check 4/4, emit-tv-check 407/407 — no
  regressions.
- **Side effect worth knowing:** `proofs/stage-manifest.tsv`'s `source-bundle`
  and `stage9-aggregate-replay`/`stage9` `DONE` rows are now genuinely
  reproducible via a live `check` run again (they were accurate for
  `source-bundle-check` alone but not for the `stage9-*` rows while this bug
  was open); no manifest edit needed now that reality matches the label.

### 2. Widen `independent-mini-backend-check` fixtures (in-house DDC track) — DONE (2026-08-12/13, later pass)

- **State: DONE-so-far, widened further.** Now 13/13 fixtures in
  `src/independent_mini_backend.rs`: the original 9 (`mini-const-arithmetic`,
  `mini-one-arg`, `mini-branch-two-arg`, `mini-mul`, `mini-sub`,
  `mini-unary-negate-branch`, `mini-equality-branch`, `mini-ge-branch`,
  `mini-recursive-factorial`) plus 4 new ones added this pass using
  capabilities the backend already had (no new backend features needed):
  `mini-nested-if`, `mini-recursive-fibonacci` (double recursive call),
  `mini-three-arg`, `mini-four-arg`. All cross-validated live against real
  `rustc`.
- **What "done" looks like (still aspirational, not a hard bar):** widen
  toward (not necessarily matching 1:1) the 407-case main corpus — this
  pass didn't add new backend LANGUAGE FEATURES (no `!=`, no strings), just
  more fixtures on top of existing arithmetic/comparison/`if`/recursion/
  multi-arg support. A future pass adding new syntax (e.g. `!=`) would need
  backend changes too.
- **Size:** small-to-medium, incremental, purely additive, no known blockers —
  each new fixture is its own self-contained slice.

### 3. mrustc-based Trusting-Trust DDC (Wheeler bar) — deferred, Linux-only

- **State:** deliberately not pursued this session. `mrustc` is nixpkgs-packaged
  but marked `platforms = [ "x86_64-linux" ]` only; this dev machine is
  `x86_64-darwin`. Forcing a cross-build via `NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM`
  was judged actively counterproductive for a *trust* witness (a shakily
  cross-built mrustc could silently miscompile, giving false DDC confidence).
- **What "done" looks like:** on a Linux box, build real `mrustc`, compile
  rs-meta's own source through it, and compare against `rustc`'s output —  a
  genuine compiler-vs-compiler (not compiler-vs-interpreter) comparison, clearing
  the full Wheeler DDC bar that item 2 above does not by itself clear.
- **Size:** large, but environment-gated, not effort-gated — the phased plan is
  already written up in `STATUS.md`; this is "pick it up on Linux," not "design it
  from scratch."

### 4. stage8–stage15/N seed + replay closures

- **State: ALL DONE.** Verified by reading every checkbox in §5.3 (Milestone C —
  stage8) and §5.4 (Milestone D — stage9-15 + stageN) below: every single line is
  `[x]`. Across the entire 1500+-line file there is exactly **one** unchecked
  `- [ ]` line, and it is unrelated to stage8-15/N (see item 6 below).
- **What this means:** no open work on this axis. These are local seed/replay
  closures — DONE here explicitly means "local proof closure," not "full rustc
  replacement" or "Trusting-Trust defense" (see item 5). That scope boundary is
  intentional and already correctly labeled throughout; nothing to re-flag.

### 5. Full rustc replacement / borrow checker / full trait solver / user `macro_rules!` — HELD (recorded non-goal, not pending work)

- **State:** explicitly HELD, and — per `docs/self-hosting.md`'s audit
  (2026-07-04, gated by `selfhost-audit-check`) — **confirmed not a blocker**:
  rs-meta's evaluator core (`lexer.rs`/`ast.rs`/`parser.rs`/`typeck.rs`/
  `interp.rs`/`sig.rs`/`hash.rs`) uses **zero** instances of `macro_rules!`,
  proc/derive macros, `async`/`await`, `unsafe`, `trait` definitions, associated
  types, `dyn Trait`, or const generics. Lifting any of these would not move the
  self-host bar. Borrow checker is held as a no-op/witness under the mrustc
  stance ("trust the input is valid; a miscompile is our bug"), sound here because
  rs-meta's own source is already borrow-validated by real `rustc`.
- **Do not re-flag this as pending work.** It is a recorded, audited scope
  decision, not a gap.
- **If ever revisited (not currently planned):** `macro_rules!` is called out in
  `docs/self-hosting.md` as "the tractable one" if a lift is ever wanted for
  downstream (`pnix-rs`) reasons. A full trait solver is explicitly called "a
  research frontier," not required to self-host.

### 6. Minor open item: A4 generic-inference tail (§5.1, line ~380) — DONE (2026-08-12/13, later pass)

- **State: DONE.** Preserved nested unsuffixed integer provenance so
  `Rc::ptr_eq(&Rc::new(vec![1]), &Rc::new(vec![1u64]))` now infers `u64`
  like `rustc` does, while explicit `Rc<i64>` vs `Rc<u64>` (both concrete,
  conflicting) still gets rejected.
- **Root cause:** `vec![...]`'s own typeck (`Expr::VecLit` in `typeck.rs`)
  eagerly collapses an unsuffixed integer literal element to `i64`
  (`collapse_lit`) before returning the `Vec<T>` type — so by the time
  `Rc::ptr_eq` compares its two `&Rc<Vec<T>>` arguments' inner types, the
  first side's `1` has already lost its flexibility and become a concrete
  `Vec<I64>`, which no longer unifies with the second side's `Vec<U64>`.
  A blanket fix (removing the eager collapse everywhere) was tried first and
  reverted: it broke a genuinely different case
  (`vec![3, 1, 2]; v.sort_by(|a, b| b.cmp(a))`, a positive self-check
  fixture) where the literal really does need to default to `i64` in
  isolation, since nothing else constrains it — this needs real
  bidirectional inference to do generally, which is out of scope for a
  "very small" item.
- **Fix (narrowly scoped to `Rc::ptr_eq` specifically):** added
  `rc_ptr_eq_uncollapsed_vec_type`, a fallback that only runs when the
  normal (collapsed) comparison fails: if an argument is exactly
  `&Rc::new(vec![...])`, re-derive that `vec!`'s element type *without* the
  eager default, then retry the comparison. Purely additive on the failure
  path — does not change behavior for anything that already passed.
- **Fixtures added:** `rc-ptr-eq-unsuffixed-literal-provenance` (positive,
  self-check/tv-check) and `rc-ptr-eq-explicit-conflicting-types` (negative,
  typeck-check).
- **Self-hosting gotcha hit while building this (same pattern as earlier
  this session):** the first draft used `.collect::<Result<Vec<_>, _>>()` to
  gather both sides' results — real rustc accepts this, but rs-meta's own
  `interp.rs` `Iter::collect` turbofish only supports `String`/`Vec<T>`
  targets, not `Result<Vec<T>, E>`, so `source-bundle-check` failed once
  this became part of the self-hosted bundle. Rewritten as two plain
  sequential calls instead of a `.collect()`.
- **Verified:** self-check 408/408, tv-check 408/408, typeck-check 273/273,
  source-ast-check 16/16, source-bundle-check PASS — no regressions.

### 7. `trait-boundary-check` regression: `held-assoc-type` now parses — DONE (2026-08-12/13, later pass)

- **State: DONE.** Confirmed live (not assumed) that associated types
  genuinely now parse, typecheck, *and execute correctly* — not just parse
  and fail later. `impl Two for D { type Out = i64; fn two(&self) -> i64 {
  self.n * 2 } }` (struct target) runs and returns the right value. The
  original fixture (`impl Two for i64`) combined TWO separate boundaries:
  associated types (now genuinely supported) and implementing any trait for
  a primitive type (still separately unsupported, confirmed by testing a
  trait with NO associated type against the same `i64` target — same "impl
  target I64 is not supported yet" error either way).
- **Also confirmed the "support" is shallow, not real associated-type
  modeling:** `Self::Out` in the trait's own method signature is never
  checked against the impl's `type Out = ...` binding. An impl declaring
  `type Out = i64` with a method that actually returns `bool` is silently
  accepted (real rustc would reject the mismatch). This is intentionally
  preserved as a documented, honest gap, not silently claimed as full
  support.
- **Fix:** renamed the classification from `held-assoc-type` (false — it
  does parse now) to `assoc-type-accepted-unenforced` in
  `trait_boundary_report`, with a doc comment explaining exactly why and
  what's still not modeled. Updated `trait_boundary_check`'s test case to
  use a supported (struct) impl target and assert real execution, plus
  added a second case asserting the mismatch-is-silently-accepted gap
  explicitly (so it stays documented, not just true-by-accident).
- **Second caller found and fixed:** `rust_surface_check` (the
  `rust_surface_report` consumer used by `pnix-rs`'s peer-engine `verdict`
  field) had its OWN separate hardcoded `"held-assoc-type"` string
  assertion, missed on the first pass and caught by re-running the full
  `bin/rs-meta-gate check` aggregate before considering this closed. Fixed
  to match the renamed classification.
- **Self-hosting gotcha hit while building this:** an early draft used
  `Option::as_deref()` in the new test code, which `interp.rs` doesn't
  model either (`unsupported Option method as_deref`) — rewritten as a
  plain `match`.
- **Verified:** `trait-boundary-check` 4/4, `rust-surface-check` 4/4, plus
  the full regression bar (self-check/tv-check/typeck-check/mini-backend/
  source-ast/source-bundle/macro-boundary/borrow-boundary) all green, no
  regressions.

## 2. Stage 사다리 (rustc bootstrap 모델 → stage15~N)

rustc의 stage0/1/2 부트스트랩을 stage15~N까지 확장한다.

```text
stage0  = rustc가 평가기를 빌드 (cargo build) — 신뢰된 seed             [DONE]
interp  = Rust subset의 in-Rust 트리워킹 평가기 (오라클)                 [DONE]
native  = 같은 Rust 소스 -> rustc -> run (evcxr 메커니즘, zero-dep)      [DONE]
tv      = interp 출력 == rustc 출력 (corpus 전체)                        [DONE]
subset  = rs-meta src/*.rs source-cover + positive/negative corpus        [DONE]
stage1  = 평가기 자신의 소스가 subset 안에 들어감                        [DONE]
stage2  = all-source evaluator'가 corpus를 replay                         [DONE]
stage3~7= slim/core chain + corpus shards + mirror + bounded B==C +
          full all-source chain replay(release, budget-gated) 전부 통과   [DONE]
stage8  = native artifact reproducibility incl. stage2 evaluator receipt [DONE]
stage9  = clean-process product/proof/aggregate replay                   [DONE]
stage10 = client/server/session/sandbox replay closure                   [DONE]
stage11 = multi-domain adapter replay closure                            [DONE]
stage12 = self-improvement quarantine replay closure                     [DONE]
stage13 = long-horizon organism replay closure                           [DONE]
stage14 = cross-implementation replay closure                            [DONE]
stage15 = open-world evidence federation replay closure                  [DONE]
stageN  = versioned constitutional extension replay closure              [DONE]
```

Cross-stage 불변(모든 stage8+ 증명): canonical record/hash 비교, hard manifest
binding vs soft env observation 분리, drift는 debug manifest로, machine-readable
stage manifest index, per-stage timeout/cost note.

## 3. 현재 위치 — slice 1 (2026-06-30, DONE)

처음부터 새로 작성. **전부 실제로 돌고 검증됨.**

```text
rs-meta/
  Cargo.toml         # zero-dep, bin = bootstrap
  src/
    ast.rs           # Rust subset AST
    lexer.rs         # 텍스트 -> 토큰 (손수 작성)
    parser.rs        # 재귀하강 파서 -> Program
    typeck.rs        # 가벼운 타입검사 (acceptance TV: interp 거부 ⟺ rustc 거부)
    interp.rs        # 트리워킹 평가기 = meta-circular 오라클 (println! 캡처)
    native.rs        # 같은 Rust 소스 -> rustc -> run (evcxr 메커니즘)
    check.rs         # corpus + self/tv/typeck/source/stage2-probe checks
    main.rs          # bootstrap CLI
  samples/*.rs       # 실제 Rust (rustc로도 컴파일됨)
```

subset (현재):
- 아이템: `fn`, `struct { f: T, ... }`, `enum { Unit, Tuple(T, ...), Struct { f: T } }`
- 타입: `i64`, `i32`, `u32`, `u8`, `usize`, `char`, `bool`, `()`,
  `Named`(struct/enum), `(T, ...)` 튜플, `[T]` slice type (`len/get/iter/index`) — type-check 사용
- 문: `let (mut)? x (: T)? = e;`, assignment expression `lhs = rhs` 및
  compound assignment expression (`+= -= *= /= %=`), statement/expression `return e?`,
  expr/block stmt
- 식: 정수/char/bool 리터럴, 변수, `as` 캐스트, `&`/`&mut` 참조, `*` 역참조,
  free function/associated/method 호출, 단항(`- !`),
  이항(`+ - * / %`, `== != < <= > >=`, `&& ||` 단락), tuple enum variant
  constructor values, `if/else`, block,
  `match`(패턴 wild/bind/`@`/int/range/char range/bool/tuple/enum + prelude `Some/None/Ok/Err`
  + guard `pat if cond`),
  튜플/struct/enum 리터럴, struct field shorthand `S { x }`, 필드 `.x`/튜플인덱스 `.0`,
  `while`/`loop`/`for a..b`(`..=`),
  `break`/`continue`(Never typing), trailing diverging statement block typing,
  `println!("{}", ...)`
- impl: inherent `impl Type { ... }`, associated fn `Type::new(...)`, method
  `x.f(...)`, receiver `self`/`&self`/`&mut self`(간단한 mut receiver 검사)
- refs: `&T`/`&mut T` 타입, `&place`/`&mut place`, `*r`, `*r = value`,
  참조 receiver field/method auto-deref 근사. full borrow checker/lifetime은 held.
- Vec: `Vec<T>` 타입 표면, `Vec::new()`, `Vec::with_capacity`, `vec![...]`, `.push`, `.pop`, `.remove`, `.reverse`,
  `.get`, `.first`, `.last`, `.last_mut`, `.len`, `.is_empty`, `.clear`, `v[i]` 읽기/쓰기 인덱싱,
  `.iter`, `.iter_mut`(read-observation surface), `.into_iter`, stateful `Iter::next`,
  `Iter::nth`, `Iter::last`, `Iter::map`, `Iter::enumerate`, `Iter::find`,
  `Iter::position`, `Iter::count`, `Iter::sum`, `Iter::fold`, `Iter::take`,
  `Iter::skip`, `Iter::copied`, `Iter::cloned`, string-like `.join`,
  range slicing `v[a..b]`/`v[a..=b]`/`v[..b]`/`v[a..]`, `.to_vec()`, `for x in v`,
  `for x in &v`.
- String/&str: 문자열 리터럴(`&str`), `String::new/from`, `.to_string`,
  `.push_str`, `.push(char)`, `.chars()` as `Iter<char>`, `.bytes()` as `Iter<u8>`, `.trim()`, `.split()`, `.len`, `.is_empty`, `.as_str`, `.contains`,
  `.starts_with`, `String + &str`, `String == &str`, string-like `&String`,
  `&String` → `&str` call coercion.
- Option/Result: prelude-style `Some`/`None`/`Ok`/`Err`, `.unwrap`, `.unwrap_or`,
  `.is_some`, `.is_none`, `.is_ok`, `.is_err`, `Result::ok`, `Result::map`, `Result::map_err`,
  `Option::as_ref`, `Option::map`, `Option::and_then`, `Option::copied`, `Option::cloned`, `Option::unwrap_or_else`, `Option::ok_or_else`, `None` placeholder
  assignment/comparison joining, `?` early-return.
- Box/Rc/RefCell: `Box::new/as_ref`, `Rc::new/clone/as_ref`, narrow `Rc<Vec<T>>.len/is_empty/iter/get/index`,
  narrow `Rc<String>.as_str/chars`, `RefCell::new`,
  `.borrow`, `.borrow_mut`, `.into_inner`, deref on `Box<T>`/`Rc<T>`, and
  `&Box<T>`/`&Rc<T>` to `&T`, `&&T` to `&T` call coercions.
- HashMap: `HashMap::new`, `.insert`, `.contains_key`, `.get`, `.get_mut`, `.remove`,
  `.len`, `.is_empty`, `.iter`, `.entry(...).or_insert/_with`, `.and_modify`
  (Vec-backed model, `.get` returns `Option<&V>`, `String` key lookup accepts `&str`).
- Surface: `#[...]` attributes ignored, `pub` visibility ignored, top-level `use`
  ignored, top-level `mod` items accepted/ignored, top-level `type` aliases preserved/resolved,
  selected built-in `.clone()` with deep String/Vec/struct/enum/Iter value clones.
- char/casts: char literal/pattern/display, char methods(`is_whitespace`,
  `is_ascii_digit`, `is_ascii_hexdigit`, `is_ascii_alphabetic`,
  `is_ascii_alphanumeric`, `.to_string`),
  `usize`/`i32`/`u32`/`u64`/`u8`, decimal/hex integer literals, integer `^`/`^=`,
  `as` casts, narrow `usize::saturating_sub` /
  `i64::saturating_sub`, `i64::max`, `i64::min`.
- 재귀·상호재귀, block 스코프 섀도잉, tuple structural compatibility

검증(2026-07-10 current receipt, 전부 PASS, corpus 407 positive / 272 negative —
전부 valid/invalid Rust):
- `self-check` 407/407 — 인터프리터가 corpus를 기대 출력으로 평가.
- `tv-check` 407/407 — **interp 출력 == rustc 출력** (translation validation).
- `typeck-check` 272/272 — **interp 거부 ⟺ rustc 거부** (acceptance TV).
- `source-ast-check` 8/8 — `src/*.rs` 전부 rs-meta front-end AST parse OK.
- `source-bundle-check` 1/1 — `src/*.rs` concat bundle `print_help` path interp == rustc.
- `stage2-chain-check` 1/1 — all-source evaluator' current positive corpus replay interp == rustc.
- `stage2-probe-check` 4/4 — lexer/parser/typeck/interp source slices interp == rustc.
- `stage3-chain-check` 1/1 — slim evaluator stage2 → stage2' chain interp == rustc.
- `stage3-all-source-smoke-check` 1/1 — slimmed evaluator-core source bundle
  stage2 → stage2' smoke chain interp == rustc.
- `stage3-core-mini-check` 1/1 — evaluator-core stage2'가 산술/재귀/enum/struct/
  Vec-String/iterator-turbofish mini-corpus를 replay하고 interp == rustc.
- `stage3-core-prefix-check` 1/1 — evaluator-core stage2'가 positive corpus
  prefix 8개를 replay하고 interp == rustc.
- `stage3-core-middle-check` 1/1 — evaluator-core stage2'가 positive corpus
  middle 8개를 replay하고 interp == rustc.
- `stage3-core-suffix-check` 1/1 — evaluator-core stage2'가 positive corpus
  suffix 8개를 replay하고 interp == rustc.
- `stage3-core-feature-check` 1/1 — evaluator-core stage2'가 후반 feature-heavy
  corpus 10개를 replay하고 interp == rustc.
- `stage3-core-negative-check` 1/1 — evaluator-core stage2'가 대표 negative
  corpus 10개를 거부하고 interp == rustc.
- `stage3-core-negative-middle-check` 1/1 — evaluator-core stage2'가 negative
  corpus middle 8개를 거부하고 interp == rustc.
- `stage3-core-negative-suffix-check` 1/1 — evaluator-core stage2'가 negative
  corpus suffix 8개를 거부하고 interp == rustc.
- `stage3-mirror-check` 1/1 — canonical AST serializer(`proofs/mirror-sig.rs`)가
  stage1(native)/stage2/stage2'에서 probe의 canonical AST + 출력을 byte-identical
  하게 산출 (nested-under-rustc 교차확인 포함).
- `stage3-fixedpoint-check` 1/1 — stage2(B)와 stage2'(C) 평가기 transcript가
  동일 (정규화 bounded B==C; artifact 재현성은 stage8-selfhost receipt).
- `stage8-repro-check` 2/2 — sample Rust source + all-source bundle을 두 workdir에서 rustc 빌드 → canonical native artifact receipt 동일.
- `stage8-selfhost-repro-check` 1/1 — stage2 evaluator' artifact를 두 workdir에서 fresh rustc build → canonical receipt 동일.
- `manifest-check` 1/1 — `proofs/stage-manifest.tsv` machine-readable stage index 구조/필수 row 검증.
- `isolation-check` 1/1 — fresh interpreter run 사이 stdout/function namespace state 누수 없음.
- `constitution-check` 1/1 — zero-dep, Actions disabled receipt, content-hash native artifact naming 검증.
- `native-cache-check` 1/1 — 같은 native source 두 번째 compile이 content-hash rustc artifact cache hit.
- `stage9-replay-check` 1/1 — clean-process lightweight entrypoint matrix replay + canonical JSON receipts.
- `stage9-proof-matrix-check` 1/1 — non-recursive proof command clean-process replay + canonical JSON receipts.
- `stage9-aggregate-replay-check` 1/1 — proof-command aggregate clean-process replay without recursive `check`.
- `stage10-session-check` 1/1 — clean-process command session transcript 두 번 replay 결과 동일.
- `stage10-sandbox-check` 1/1 — client/server/session/sandbox proof receipt + fail-closed boundary 검증.
- `stage11-adapter-check` 1/1 — adapter schema/held/conflict TSV receipt 검증.
- `stage11-adapter-replay-check` 1/1 — local/rustc adapter replay + DISABLED/HELD fail-closed closure 검증.
- `stage12-quarantine-check` 1/1 — self-improvement quarantine/no-auto-promotion/fail-closed TSV receipt 검증.
- `stage12-quarantine-replay-check` 1/1 — local gate replay + candidate intake/no-auto/DISABLED/HELD closure 검증.
- `stage13-horizon-check` 1/1 — long-horizon stale degradation/no-boundary-leak TSV receipt 검증.
- `stage13-horizon-replay-check` 1/1 — manifest/session replay + stale/external HELD degrade-to-held closure 검증.
- `stage14-cross-impl-check` 1/1 — cross-implementation export schema/conflict/held TSV receipt 검증.
- `stage14-cross-impl-replay-check` 1/1 — rs-meta/rustc local exports replay + alternate implementations HELD closure 검증.
- `stage15-evidence-check` 1/1 — open-world external evidence offline approval/fail-closed TSV receipt 검증.
- `stage15-evidence-replay-check` 1/1 — local proof/manifest replay + external evidence HELD/offline approval closure 검증.
- `stageN-extension-check` 1/1 — versioned constitutional extension/migration/budget TSV receipt 검증.
- `stageN-extension-replay-check` 1/1 — manifest index/timeout-cost/stageN seed replay + future extension HELD closure 검증.
- 교차확인: plain `rustc -O samples/factorial.rs` → `3628800` (동일).

## 4. 검증 명령

```sh
cd ~/pnix-rs/rs-meta
export CARGO_TARGET_DIR=/tmp/rs-meta-target   # target/ 오염 방지
cargo build
BIN=/tmp/rs-meta-target/debug/bootstrap

$BIN self-check
$BIN tv-check
$BIN typeck-check
$BIN source-ast-check
$BIN source-bundle-check
$BIN stage2-chain-check
$BIN stage2-probe-check
$BIN stage3-chain-check
$BIN stage3-all-source-smoke-check
$BIN stage3-core-mini-check
$BIN stage3-core-prefix-check
$BIN stage3-core-middle-check
$BIN stage3-core-suffix-check
$BIN stage3-core-feature-check
$BIN stage3-core-negative-check
$BIN stage3-core-negative-middle-check
$BIN stage3-core-negative-suffix-check
$BIN stage3-mirror-check
$BIN stage3-fixedpoint-check
$BIN stage3-full-held-check
$BIN stage8-repro-check
$BIN stage8-selfhost-repro-check
$BIN manifest-check
$BIN isolation-check
$BIN constitution-check
$BIN actions-disabled-check
$BIN native-cache-check
$BIN stage9-replay-check
$BIN stage9-proof-matrix-check
$BIN stage9-aggregate-replay-check
$BIN stage10-session-check
$BIN stage10-sandbox-check
$BIN stage11-adapter-check
$BIN stage11-adapter-replay-check
$BIN stage12-quarantine-check
$BIN stage12-quarantine-replay-check
$BIN stage13-horizon-check
$BIN stage13-horizon-replay-check
$BIN stage14-cross-impl-check
$BIN stage14-cross-impl-replay-check
$BIN stage15-evidence-check
$BIN stage15-evidence-replay-check
$BIN stageN-extension-check
$BIN stageN-extension-replay-check
$BIN check                 # self + tv + typeck + source + stage2/stage3/stage8 + manifest/isolation/constitution/actions/cache/stage9/stage10/stage11/stage12/stage13/stage14/stage15/stageN checks
$BIN stage-status

$BIN run        -c 'fn main() { println!("{}", 1 + 2 * 3); }'
$BIN run        -f samples/factorial.rs
$BIN native-run -f samples/factorial.rs
$BIN ast        -f samples/factorial.rs
```

## 5. 상세 로드맵 / 체크리스트 (stage1 → stage15-N)

**원칙(매 항목 공통):** 인터프리터 + native(rustc) + TV 동시(lockstep). 양성은
`tv-check`(interp==rustc), 음성은 `typeck-check`(둘 다 거부). corpus 항목은 전부
valid/invalid **실제 Rust**. zero-dep·honesty 유지. 각 항목 끝나면 `[x]` + 날짜.

**완료 요약(✅):**
- [x] **slice1**: Rust subset 인터프리터 + rustc native tier + TV. (2026-06-30)
- [x] **typeck**: 가벼운 타입검사 + acceptance TV(negative). (2026-06-30)
- [x] **데이터 타입**: struct/enum(unit·tuple 변형)/튜플/match(wild·bind·int·bool·
  tuple·enum 패턴)/필드·튜플인덱스 접근. (2026-06-30)
- [x] **루프+가변성**: while/loop/for(`a..b`,`..=`)/break/continue + `let mut`/대입 +
  loop_depth 검사. (2026-06-30)
- [x] **A1 impl/메서드**: inherent impl, associated fn, `self`/`&self`/`&mut self`
  receiver, method call 디스패치, receiver typeck. (2026-06-30)
- [x] **A2 참조 표면**: `&expr`/`&mut expr` temporaries/`*`/`&T`/`&mut T`, mutable ref assignment,
  참조 receiver auto-deref 근사, negative acceptance TV. (2026-06-30)
- [x] **A3-1 Vec core**: generic type surface, `Vec::new`, `Vec::with_capacity`, `vec![]`, `.push`,
  `.pop`, `.remove`, `.get`, `.first`, `.last`, `.last_mut`, `.len`, `.is_empty`, `.clear`, indexing read/write,
  `.iter`, `.iter_mut`(read-observation surface), `.into_iter`, `Iter::next`,
  `&mut Vec<T>` param, `for x in v`, `for x in &v`. (2026-06-30)
- [x] **A3-2 String/&str core**: string literal, `String::new/from`, `.push_str`,
  `.len`, `.is_empty`, `.as_str`, `.contains`, `.starts_with`, `String + &str`,
  `String == &str`, string-like `&String`.
  (2026-06-30)
- [x] **A3-3 Option/Result core**: `Some`/`None`/`Ok`/`Err`, `.unwrap`,
  `.unwrap_or`, `.is_*`, `Option::map`, `Option::and_then`, `Option::copied`,
  `Option::cloned`, `Option::ok_or_else`,
  `None` placeholder assignment joining, `Result::ok`, `Result::map_err`, `?` early-return.
  `.map`은 closure 필요로 A5에 연결.
  (2026-06-30)
- [x] **A3-4 Box/Rc/RefCell core**: `Box::new/as_ref`, `Rc::new/clone/as_ref`,
  narrow `Rc<Vec<T>>.len/is_empty/iter/get/index`, narrow `Rc<String>.as_str/chars`,
  `RefCell::new/borrow/borrow_mut/into_inner`, Box/Rc deref, `&Box<T>`/`&Rc<T>` to
  `&T` call coercion. (2026-06-30)
- [x] **A3-5 HashMap core**: `HashMap::new`, `.insert`, `.contains_key`,
  `.get`, `.get_mut`, `.remove`, `.len`, `.is_empty`, acceptance TV. (2026-06-30)
- [x] **A8/A10 surface**: `#[...]` attributes, `pub`, top-level `use`, top-level
  `mod` accepted/ignored, top-level `type` alias parse + typeck resolution. (2026-06-30)
- [x] **A10 char/casts**: `char` literal/pattern/display, char predicate methods,
  `String::push(char)`, `usize`/`i32`/`u32`/`u8`, `as` casts,
  `usize::saturating_sub` / `i64::saturating_sub` / `i64::max` / `i64::min`,
  negative acceptance TV.
  (2026-06-30)
- [x] **A5 closure expected-type inference**: `impl Fn(...) -> ...` 파라미터처럼
  기대 타입이 있는 위치에서 untyped closure params(`|a, b|`)를 타입 주입.
  (2026-06-30)

---

### 5.1 Milestone A — stage2 self-host (평가기가 평가기 자신을 평가)

목표: `src/*.rs`가 평가 subset 안에 들어가, 인터프리터가 **자기 소스**를 로드·평가해
같은 결과를 내는 것. 아래는 평가기 소스가 실제로 쓰는 기능 기준 체크리스트.

#### A1. impl / 메서드 / 연관함수
- [x] `impl Type { ... }` 블록 파싱
- [x] 메서드 정의 `fn m(&self, ...) -> T { ... }` (self receiver)
- [x] `&self` / `&mut self` / `self` (by value) receiver 구분
- [x] 연관함수 `Type::new(...)` (self 없는 정적, 경로 호출)
- [x] 메서드 호출 `x.m(args)` 파싱(postfix 확장: `.ident(...)`)
- [x] interp: 값의 런타임 타입 → impl 메서드 테이블 디스패치
- [x] typeck: receiver 타입 → 메서드 시그니처 해석
- [x] corpus: `Point::new(3,4)` + `p.dist_sq()` 등 (rustc 등가)
- 주: inherent impl + narrow trait impl method surface. UFCS/general borrow는 held.

#### A2. 참조 & 소유권 표면 (인터프리터는 값의미로 근사)
- [x] `&expr` / `&mut expr` 파싱
- [x] `*expr` 역참조
- [x] 메서드 호출 시 자동 참조/역참조 근사
- [x] interp: 참조를 값 핸들로 근사(Rc 공유) — 가변 참조 통한 변경 관측
- [x] TV로 관측 동등성 확인; 앨리어싱 의존 케이스는 정직 held
- 주: **full borrow checker/lifetime은 범위 밖(held)** — rustc가 빌림검사를 맡고
  interp는 동작만 근사. 빌림 위반 프로그램은 rustc가 거부(=acceptance 경계).

#### A3. 제네릭 & 표준 컬렉션 (Vec/String/Map/Option/Result/Box/Rc/RefCell)
- [x] 제네릭 파라미터 표면 + shallow monomorphic unification:
  `fn f<T>(...)`, `struct S<T>`, `enum E<T>`에서 `T`/`E`/`K`/`V`류 타입 변수를
  호출·리터럴·패턴 경계에서 실제 타입으로 치환. full generic solver는 held.
- [x] 타입 인자 `Vec<i64>` 및 `Wrap<T>`/`Opt<T>`류 generic type surface 파싱
- [x] **Vec\<T\>** 모델 (interp: `Rc<RefCell<Vec<Val>>>`)
  - [x] `Vec::new()` / `vec![...]`
  - [x] `.push` `.len` `.is_empty` `.clear`
  - [x] `.pop` `.remove` `.get` `.first` `.last`
  - [x] 인덱싱 `v[i]` 읽기
  - [x] 인덱싱 `v[i]` 쓰기
  - [x] `for x in &v` / `for x in v`
  - [x] `.iter()` / `.into_iter()` / `Iter::next` (narrow `Iter`, collect는 아래 A5에 연결)
- [x] **String / &str** 모델
  - [x] 문자열 리터럴 → `&str`, `String::new()` / `String::from`
  - [x] `.to_string()`
  - [x] `.push_str` `.len` `.is_empty` `.as_str`
  - [x] `.push`(char 필요) `.chars()` as `Iter<char>`
  - [x] `.bytes()` as `Iter<u8>` / `.as_bytes()` as `&[u8]`
  - [x] `+` 연결, `.contains` `.starts_with`
  - [x] `.split`
- [x] **Option\<T\> / Result\<T,E\>** (내장 enum 으로 승격 + 메서드)
  - [x] `Some`/`None`/`Ok`/`Err` 내장 변형
  - [x] `.unwrap` `.unwrap_or` `.is_some` `.is_none` `.is_ok` `.is_err` `.ok`
    `.as_ref` `.unwrap_or_else`
    `.copied` `.cloned` `.ok_or_else`
  - [x] `let mut x = None; x = Some(T)` placeholder assignment joining
  - [x] `.map` (closure 기반)
- [x] **Box\<T\>** (`Box::new`, `.as_ref`, deref)
- [x] **Rc\<T\> / RefCell\<T\>** (`Rc::new/clone`, `RefCell::new/borrow/borrow_mut/into_inner`)
- [x] **HashMap\<K,V\>** 모델 (`.insert` `.get` `.contains_key` `.remove`
  `.len` `.is_empty` `.iter`)
- [x] HashMap `.entry` API: `.or_insert`, `.or_insert_with`, `.and_modify`
- [x] char / `usize` / 기타 정수 타입 + 캐스트 `as` (기본: `i32/u32/u8/usize`)
- [x] narrow `usize::saturating_sub` / `i64::saturating_sub` / `i64::max` / `i64::min` (parser/interp source surface)

#### A4. 트레잇
- [x] `trait T { fn ...; }` 정의 surface (body parsed/skipped)
- [x] `impl T for Type { ... }`
- [x] 트레잇 메서드 디스패치(정적; interp는 값타입 기반)
- [x] narrow `impl Into<String>` parameter + `.into()` surface (trait solver는 held)
- [x] `#[derive(Clone, PartialEq, Debug)]` 근사 처리 일부: derive attr 파싱/무시 +
  selected built-in `.clone()` surface (deep String/Vec/struct/enum/Iter value clone)
- [x] `Display` / `Debug` trait 표면(typeck Display 가능성 체크 포함; `{}`/align vs `{:?}` vs `{:016x}`)
- [x] fixed macro placeholder subset: `print!/println!/format!` `{}` / `{:?}` / `{:#?}` / `{:016x}` / `"{:<N}"` / `"{:.N}"` (numeric fixed precision, N <= 65535)
- [ ] generic inference tail: preserve nested unsuffixed integer provenance so
  `Rc::ptr_eq(&Rc::new(vec![1]), &Rc::new(vec![1u64]))` can infer `u64` like
  rustc, while explicit `Rc<i64>` versus `Rc<u64>` remains rejected.
- [x] `Iterator` 최소(`.next()`, for 통합)
- 주: **trait coherence/오버랩/orphan 검사는 범위 밖(held).**

#### A5. 클로저 & 반복자 어댑터
- [x] 클로저 `|x| expr` / `|x| { ... }` / `|x: T| ...` 파싱
- [x] typed closure 값(환경 값 캡처) + closure variable 호출 `f(x)`
- [x] zero-arg closure `|| expr` / `|| -> T { ... }` + return annotation typeck
- [x] expected closure parameter inference (`|x| ...`) — iterator adapter와
  `impl Fn(...) -> ...` 함수 인자 위치
- [x] 핵심 어댑터 `.iter()` `.map()` `.filter()` `.collect()` `.enumerate()`
  - [x] narrow `Vec::iter()` + `Iter::map()` + `Iter::filter()` + `Iter::zip()` +
    `Iter::all()` + `Iter::any()` + `Iter::collect()` (`&char` → `String`, 그 외 Vec)
- [x] `.find()` / `.position()` / `.nth()` / `.last()`
- [x] `.sum()` `.fold()` `.count()`
- [x] `.any()` / `.all()`
- [x] `.zip()` (Iter x Iter -> Iter<(A, B)>)
- [x] `.rev()`
- [x] `.take`/`.skip`
- [x] range `.map`

#### A6. 패턴 매칭 확장
- [x] struct/struct-like enum field 패턴 `Point { x, y }` / `E::V { x }` +
  rest `Point { x, .. }` / `E::V { x, .. }`
- [x] match guard `pat if cond =>`
- [x] prelude enum pattern `Some(x)` / `None` / `Ok(x)` / `Err(e)`
- [x] or-pattern `a | b`
- [x] binding `@` (`n @ 1..=5`)
- [x] 참조 패턴 `&pat` / `&mut pat`
- [x] one-layer ref match ergonomics: literal/tuple/struct/enum destructuring over `&T`
- [x] binding modifier `ref x` / `ref mut x`
- [x] 리터럴 범위 패턴 `1..=5` / `'a'..='z'`
- [x] 문자열 리터럴 패턴 `"..."` (`&str`/`String::as_str` match)
- [x] `if let`
- [x] `while let`
- [x] `let ... else`
- [x] match 망라성(exhaustiveness) 가벼운 검사(누락 변형 거부).
  bool/custom enum/Option/Result 대상. guard arm은 rustc처럼 망라성에 기여하지 않음.

#### A7. 에러 전파
- [x] `?` 연산자 (Result/Option)
- [x] interp: `?` = early-return match 로 lowering
- [x] narrow `From<&str> for String` error 변환 (`Result<T, &'static str>?`
  in `Result<T, String>`). full `From<E>` trait solving은 held.

#### A8. 모듈 & 가시성
- [x] `mod name { ... }` (인라인, body ignored) + `mod name;` accepted
- [x] `use path::{...}` / `use path as alias` accepted as ignored item
- [x] `pub` (파싱; interp 무시)
- [x] 경로 해석 `a::b::c` (known std path canonicalization: `std::path::PathBuf`,
  `std::collections::HashMap`, `std::process::ExitCode`, `std::rc::Rc`, `std::fs`, `std::env`)

#### A9. 매크로 (고정 집합만)
- [x] `format!` (`{}` / `{0}` / `{name}` / `{:?}` / `{:#?}` / `{:016x}` /
  `"{:<N}"` / `"{:>N}"` placeholders + escaped braces, interp: println 포맷 재사용)
- [x] `vec![...]`
- [x] `matches!(e, pat)` + guard `matches!(e, pat if cond)`
- [x] `cfg!(name)` fixed macro (`native.rs` platform branch self-host blocker)
- [x] `write!` / `writeln!` (String / &mut String target)
- [x] `panic!` / `unreachable!` / `todo!` (Never)
- [x] `assert!` / `assert_eq!` (bool/equality typeck + panic-on-fail interp behavior)
- 주: **사용자 `macro_rules!` 정의는 범위 밖(held)** — 고정 집합만 인식.

#### A10. 잡다 표면
- [x] 정수 타입 다양화(`u8`/`u32`/`u64`/`usize`/`i32`) + 캐스트 `as`
- [x] integer literal suffix `1usize`, `2i64`, `3u8` 등 known suffix 소비
- [x] in-range unsuffixed integer literal inference in expected integer contexts
- [x] hex integer literal `0x...` (`native.rs` FNV 상수 self-host blocker)
- [x] `u64::from_str_radix` (`lexer.rs` hex literal parser self-host blocker)
- [x] integer bitxor `^` / `^=` (`native.rs` FNV hash self-host blocker)
- [x] char 리터럴 `'a'`, 이스케이프, char 패턴/display
- [x] 기본 char 메서드: `is_whitespace`, `is_ascii_digit`,
  `is_ascii_hexdigit`, `is_ascii_alphabetic`, `is_ascii_alphanumeric`, `.to_string`
- [x] assignment expression `lhs = rhs`
- [x] compound assignment expression `+= -= *= /= %=`
- [x] return expression `return e` (diverging `Never` 타입 흡수)
- [x] Vec range slicing `v[a..b]` / `v[a..=b]` / `v[..b]` / `v[a..]`
- [x] slice type/reference surface `[T]` / `&[T]`, `&Vec<T>` → `&[T]` call coercion
- [x] 배열 `[T; N]` + 배열 인덱싱 (`[x; n]` / `vec![x; n]` repeat 포함,
  Vec-backed 값 모델)
- [x] `const` / `static` (top-level immutable globals)
- [x] 포맷 확장: named/positional 인자 (`{0}`, `{name}`, `name = expr`)
- [x] 포맷 확장: `{:>N}` right-align placeholder
- [x] doc 주석 `///` / `//!` (line comment path로 무시)
- [x] 속성 `#[...]`(파싱·무시)
- [x] `as` 캐스트 타입검사 정합 (numeric/bool/char/ref boundary + negative corpus)
- [x] method turbofish `m::<T>()` 파싱 + `str/String.parse::<i64>()`
- [x] lifetime token/parameter/type surface (`'a`, `<'a>`, `&'a T`) — erased in model
- [x] struct literal field shorthand `S { x }`

#### A11. self-host probe 메커니즘 (점진)
- [x] 인터프리터가 멀티파일 `.rs`를 로드·평가하는 경로 정비 (`run/ast/native-run -f a.rs -f b.rs ...`)
- [x] `src/ast.rs + src/lexer.rs + src/parser.rs + src/typeck.rs` concat bundle:
  rs-meta로 parse/typeck/eval-init 통과, expected stop = `interp: no fn main`. (2026-06-30)
- [x] `src/ast.rs + src/interp.rs` concat bundle:
  rs-meta로 parse/typeck/eval-init 통과, expected stop = `interp: no fn main`. (2026-06-30)
- [x] `src/ast.rs + src/lexer.rs + src/parser.rs + src/typeck.rs + src/interp.rs`
  core concat bundle: rs-meta로 parse/typeck/eval-init 통과, expected stop =
  `interp: no fn main`. (2026-06-30)
- [x] `src/*.rs` all-source concat bundle:
  rs-meta와 rustc 양쪽에서 같은 `print_help` stdout. 원래 CLI `main`은
  flatten bundle 안에서 `bootstrap_main`으로 보존. (2026-06-30)
- [x] `src/lexer.rs` 를 인터프리터로 평가 → 작은 입력의 토큰열이 native 와 일치
- [x] `src/parser.rs` 부분 → parse count가 native 와 일치
- [x] `src/typeck.rs` 부분 → parse+typeck harness 결과가 native 와 일치
- [x] `src/interp.rs` 부분 → `Interp::run_main()` harness 결과가 native 와 일치
- [x] **stage2 mini = 인터프리터가 `src/*.rs` 전체 로드 → 평가기' 생성**:
  all-source evaluator'가 산술/재귀/struct-field/enum-match/Vec-String/`&mut self` field mutation mini-corpus를
  평가하고 rustc 경로와 동일 stdout. (2026-06-30)
- [x] stage2(평가기')가 positive corpus 전체 평가 → stage1 결과와 일치 (**chain-check**):
  all-source evaluator'가 positive corpus 전체를 replay하고 rustc 경로와 동일 stdout.
  host stack sizing(build.rs), Vec element refs, unit pattern, Rc debug, hidden String `into_iter`
  runtime support로 blocker 제거. (2026-07-01)

---

### 5.2 Milestone B — stage3-7 (eval mirror 폐포 + 고정점)
- [x] **stage3 slim chain-check**: stage1→stage2(slim evaluator)→stage2' 로드 후
  `42` harness 평가 동일. (2026-07-01)
- [x] **stage3 evaluator-core source smoke chain-check**: stage1→stage2(slimmed
  evaluator-core source bundle)→stage2' smoke harness(`42`) 평가 동일. full
  all-source corpus replay 전 비용 경계를 한 단계 축소. (2026-07-01)
- [x] **stage3 evaluator-core mini-corpus chain-check**: stage2'가 산술/재귀/
  enum/struct/Vec-String/iterator-turbofish mini-corpus를 직접 `interp_run`으로 replay하고 rustc와
  동일 stdout. full corpus replay 전 비용 경계를 한 단계 더 축소. (2026-07-01)
- [x] **stage3 evaluator-core corpus-prefix chain-check**: stage2'가 현재 positive
  corpus 앞 8개를 자동 embed 받아 직접 `interp_run`으로 replay하고 rustc와 동일
  stdout. full corpus replay 전 비용 경계를 수동 mini-corpus보다 넓게 축소. (2026-07-02)
- [x] **stage3 evaluator-core corpus-middle chain-check**: stage2'가 현재 positive
  corpus 가운데 8개를 자동 embed 받아 직접 `interp_run`으로 replay하고 rustc와
  동일 stdout. prefix/suffix 사이의 moving middle shard까지 로컬 stage3 증거로
  고정. (2026-07-02)
- [x] **stage3 evaluator-core corpus-suffix chain-check**: stage2'가 현재 positive
  corpus 뒤 8개를 자동 embed 받아 직접 `interp_run`으로 replay하고 rustc와 동일
  stdout. corpus tail이 이동해도 최신 tail shard를 계속 stage3로 검증. (2026-07-02)
- [x] **stage3 evaluator-core feature-corpus chain-check**: stage2'가 후반
  feature-heavy corpus 10개(radix/alias Rc RefCell/trait method/write macro/
  struct-like enum rest/slice-array/clone/generic/let-else/array repeat)를
  named set으로 replay하고 rustc와 동일 stdout. full corpus replay 전 비용
  경계를 prefix 외 방향으로도 축소. (2026-07-02)
- [x] **stage3 evaluator-core negative-corpus chain-check**: stage2'가 대표
  negative corpus 10개(type/borrow/Vec/closure-ish Result/map/radix/write/
  pattern/let-else/generic mismatch)를 거부하고, 그 거부 동작 자체가 rs-meta와
  rustc 실행에서 동일 stdout. acceptance 경계를 stage3 쪽으로 확장. (2026-07-02)
- [x] **stage3 evaluator-core negative-middle chain-check**: stage2'가 현재
  negative corpus 가운데 8개를 자동 embed 받아 직접 `interp_run(...).is_err()`로
  거부를 확인하고 rustc와 동일 stdout. named set과 tail 사이의 negative middle
  shard도 stage3 acceptance 증거에 포함. (2026-07-02)
- [x] **stage3 evaluator-core negative-suffix chain-check**: stage2'가 현재
  negative corpus 뒤 8개를 자동 embed 받아 직접 `interp_run(...).is_err()`로
  거부를 확인하고 rustc와 동일 stdout. 최신 negative tail도 stage3 acceptance
  증거에 포함. (2026-07-02)
- [x] **stage3 full held-check**: `stage3-full-held-check`가 manifest의
  `stage3-full-chain` HELD/cost-boundary row를 검증해 상태 drift를 방지. (2026-07-02)
- [x] **full all-source chain (DONE 승격, 2026-07-02)**: stage1→stage2(all-source
  evaluator)→stage2' **전체 corpus replay가 release 빌드에서 2103s로 PASS** (interp
  == rustc). 경로: dev 420s timeout(비용) → release 2451s 완주하나 FAIL(fidelity)
  → diag로 2케이스 특정 → `value_eq` Vec arm 수정 → **PASS**. `stage3-full-chain`
  manifest row DONE(budget-gated, `RS_META_STAGE3_FULL_CHAIN_BUDGET_SECS=3600`),
  `stage3-full-held-check`는 이제 DONE row의 budget/release note를 가드.
  기본 `check` aggregate에서는 비용상 budget 미설정 시 skip(명시 메시지).
  - [x] 2026-07-02: `RS_META_STAGE3_FULL_CHAIN_BUDGET_SECS=120`로 `stage3-full-chain-check`
    실행 시 360초 기준 종료(타임아웃) 확인, held 경계 유지.
  - [x] 현재 blocker 기록: all-source stage2가 all-source/lexer-slice stage2'를 로드하는
    경로가 420s timeout. Slim evaluator chain은 통과.
  - [x] 2026-07-02 release 실측: release(-O3) 빌드로 full chain이 **2451s에 완주했으나
    FAIL** — outer harness의 `interp_run(inner).unwrap()`이 "called unwrap on Err".
    즉 full 경계는 비용만이 아니라 **실제 Err를 내는 블로커**. 진단을 위해 outer
    harness를 unwrap 대신 Err 페이로드 출력(`ERR {e}`)으로 바꾸고,
    `RS_META_STAGE3_FULL_CHAIN_SMOKE=1` smoke 모드(번들 로드만) 추가.
    manifest note 갱신(2500s).
  - [x] 2026-07-02 smoke 실측: **smoke PASS, 28.6s (release)** — all-source stage2가
    all-source stage2'를 로드(lex/parse/typeck)하고 smoke main을 평가, interp==rustc.
    이전 기록("load 경로 자체가 420s timeout")은 dev 빌드 비용이었고, release에선
    번들 로드는 문제없음. **블로커는 depth-3 corpus replay 안의 특정 케이스(들)** —
    2451s 런은 ~40분을 replay에서 소모 후 실패.
  - [x] `RS_META_STAGE3_FULL_CHAIN_DIAG=1` 진단 모드 추가: stage2'가 corpus 전 케이스를
    unwrap 없이 돌며 per-case `MISMATCH {name}` / `ERRCASE {name} :: {err}` 출력 →
    실패 케이스 특정. (diag 실행은 ~40min; 결과로 fidelity 버그 수정 후 full DONE 승격
    시도.)
  - [x] 2026-07-02 diag 결과: **297 중 2 실패, 단일 근본 원인** —
    `result-map-err-question`, `parse-turbofish-i64` 둘 다
    "parse only supports turbofish i64, got [Type::I64]" (= i64를 받고도 비교 실패).
  - [x] 2026-07-02 근본 원인 (6.4s 최소재현으로 격리): `interp.rs value_eq`에
    `(Val::Vec, Val::Vec)` arm이 없어 Vec 컨테이너 등가가 `_ => l == r`(derived
    PartialEq)로 추락. derived 경로는 ref-aware/string-content-aware가 아니라서,
    **평가기 자신이 해석될 때**(depth-3: 비교 코드가 interpreted 머신리 위에서 실행)
    clone된 `Vec<enum>`의 slice-vs-array 비교가 깨짐. 원소별 비교(value_eq 재귀,
    ref-aware)는 통과하고 컨테이너 비교만 실패하는 것과 정확히 일치.
    격리 사다리: 직접 생성 값 OK → parser 산출 값 OK → **`.clone()` 경유 값 FAIL**
    (L1에선 전부 OK, L2에서만 분화).
  - [x] 2026-07-02 수정: `value_eq`에 ref-aware `(Val::Vec, Val::Vec)` arm 추가
    (borrow 후 len + 원소별 value_eq 재귀). depth-3 재현 D3OK, L1/L2 진단 완전 일치.
    회귀 가드 corpus `clone-vec-enum-slice-eq` 추가 (corpus 298).
  - [x] source bundle slimming 1차 적용: smoke gate는 proof corpus/check/main을
    제외한 evaluator-core source bundle로 축소해 로컬 budget 안에 둠. full corpus
    chain은 parse-cache / typeck-cache / direct AST handoff 중 하나로 추가 비용 제거
    후 DONE 승격.
- [x] **stage3-full-held-check 재정의**: HELD 가드에서 **DONE boundary 가드**로 —
  manifest의 stage3-full-chain row가 DONE + budget-gated command + release cost
  note(≥2100s)를 유지하는지 검증. drift 시 FAIL. (2026-07-02)
- [x] **mirror-check (DONE 승격)**: `stage3-mirror-check` — 손수 작성한 canonical AST
  serializer(`proofs/mirror-sig.rs`, derive Debug 미사용: interp debug 렌더링은
  rustc derive와 byte-faithful하지 않음을 실측 확인)를 evaluator-core 번들에 붙여
  같은 serializer 소스가 stage1(native)/stage2/stage2'에서 probe
  (`samples/mirror_probe.rs`)의 canonical AST + 실행 출력을 **byte-identical**하게
  산출함을 검증. nested-under-rustc 교차확인 포함(4중 transcript 동등). (2026-07-02)
- [x] **B==C 고정점 (bounded DONE 승격)**: `stage3-fixedpoint-check` — stage2가
  물질화한 평가기(B)와 stage2'가 물질화한 평가기(C)의 transcript(canonical AST +
  probe 출력)가 동일함을 정규화 비교로 검증. artifact 재현성은
  `stage8-selfhost-repro-check` receipt와 결합. (2026-07-02)
  full all-source chain replay도 이후 같은 날 DONE 승격(위 항목)되어, stage2'의
  전체 corpus 행동 동등까지 실증. 남은 것은 canonical-AST 축의 all-source-스케일
  mirror(현재 evaluator-core 스케일) 정도로, 별도 확장 항목.
- [x] rustc bootstrap 모델(stage0/1/2) 매핑 문서 + receipt 산출:
  `proofs/rustc-bootstrap-map.md` + manifest row. (2026-07-01)
- [x] stage 별 모듈 격리(상태 누수 없음) 검증:
  `isolation-check` fresh interpreter runs stdout/function namespace isolation. (2026-07-01)

### 5.3 Milestone C — stage8 (재현 빌드)
- [x] **seed artifact 결정성**: 같은 sample Rust source를 서로 다른 두 workdir에서
  rustc 빌드 → native artifact FNV hash 동일. (2026-07-01)
- [x] **결정성 플래그 seed**: native tier에 `SOURCE_DATE_EPOCH=0`,
  `-C debuginfo=0`, `-C metadata=rsmeta`, `-C codegen-units=1`,
  `--remap-path-prefix <workdir>=.` 적용. (2026-07-01)
- [x] **canonical seed receipt**: source hash, rustc version, deterministic flags,
  artifact hash를 canonical record로 묶고 두 workdir receipt 비교. (2026-07-01)
- [x] **drift debug manifest(seed)**: receipt mismatch 시 두 canonical receipt를
  그대로 출력. (2026-07-01)
- [x] **local gate**: `stage8-repro-check`를 CLI와 기본 `check`에 포함. (2026-07-01)
- [x] full-source bundle native artifact reproducibility: all-source evaluator bundle을
  두 workdir에서 빌드하고 canonical receipt 비교. (2026-07-01)
- [x] 추가 결정성 플래그 seed: `SOURCE_DATE_EPOCH`, `-C metadata`, codegen-units.
  (2026-07-01)
- [x] full-source stage8 receipt: all-source evaluator bundle source hash,
  rustc version, deterministic env/flags, artifact hash를 canonical record로 비교.
  (2026-07-01)
- [x] self-hosted stage8 receipt: stage2 evaluator' artifact를 fresh workdir 두 곳에서
  rustc build하고 canonical record로 비교. (2026-07-01)

### 5.4 Milestone D — stage9-15 + stageN (federation 사다리)
rs-meta 고유의 federation 사다리. Local replay closure는 DONE이고, 외부/ambient
증거·대체 구현·미래 확장은 HELD/fail-closed로 고정한다.
- [x] **stage9 seed**: lightweight product entrypoint matrix(`help`, `stage-status`,
  `run`, `native-run`, `ast`, `manifest-check`) clean-process replay
  (hard-fixed `SOURCE_DATE_EPOCH`, soft-observed `PATH` for rustc lookup,
  canonical JSON receipts). (2026-07-01)
- [x] **stage9 proof matrix**: non-recursive proof commands(`self-check`, `tv-check`,
  `typeck-check`, source/stage/proof policy checks)를 clean subprocess에서 replay하고
  canonical receipt 비교. (2026-07-01)
- [x] **stage9 bounded aggregate replay**: 비재귀 proof-command matrix를 clean
  subprocess에서 replay하고 canonical receipt 비교. full aggregate `check` 재호출은
  로컬 비용 경계상 제외. (2026-07-01)
- [x] **stage10 session seed**: clean-process command session transcript
  (`run`, `native-run`, `ast`, `stage-status`) 두 번 replay 결과 동일. (2026-07-01)
- [x] **stage10 full**: `proofs/session-sandbox.tsv`로 client/server/session/sandbox
  replay boundary, local-only sandbox env, Actions disabled, HELD external sandbox,
  fail-closed conflict policy를 고정하고 `stage10-sandbox-check`로 검증. (2026-07-01)
- [x] **stage11 adapter schema seed**: `proofs/adapter-schema.tsv`로
  local/DISABLED/HELD adapter, held policy, fail-closed conflict policy를
  machine-readable receipt로 고정하고 `stage11-adapter-check`로 검증. (2026-07-01)
- [x] **stage11 full**: `proofs/adapter-replay.tsv`로 local-rust/stage10,
  rustc-native/TV adapter replay를 clean subprocess로 실행하고, GitHub Actions
  DISABLED 및 external adapter HELD rows를 fail-closed로 고정.
  `stage11-adapter-replay-check`로 검증. (2026-07-01)
- [x] **stage12 quarantine seed**: `proofs/quarantine-policy.tsv`로 local verification,
  GitHub Actions disabled, no-auto-promotion, fail-closed, held rows를 receipt로 고정하고
  `stage12-quarantine-check`로 검증. (2026-07-01)
- [x] **stage12 full**: `proofs/quarantine-replay.tsv`로 local-verification과
  candidate-intake gate를 clean subprocess로 replay하고, Actions DISABLED,
  manual/self/external promotion HELD, no-auto-promotion, fail-closed 정책을
  `stage12-quarantine-replay-check`로 검증. (2026-07-01)
- [x] **stage13 horizon seed**: `proofs/horizon-policy.tsv`로 stale evidence는 HELD로
  강등, boundary leak 금지, manifest/replay receipt 기반 long-horizon policy를 고정하고
  `stage13-horizon-check`로 검증. (2026-07-01)
- [x] **stage13 full**: `proofs/horizon-replay.tsv`로 manifest/session replay를
  clean subprocess로 확인하고, stale/external-memory/organism-state/ambient-network를
  HELD + degrade-to-held + no-boundary-leak로 고정. `stage13-horizon-replay-check`로
  검증. (2026-07-01)
- [x] **stage14 cross-implementation seed**: `proofs/cross-impl-schema.tsv`로
  local rs-meta/rustc native export schema, alternate toolchain/evaluator HELD rows,
  fail-closed conflict policy를 receipt로 고정하고 `stage14-cross-impl-check`로 검증.
  (2026-07-01)
- [x] **stage14 full**: `proofs/cross-impl-replay.tsv`로 rs-meta-local(stage13
  horizon replay)과 rustc-native(TV)를 clean subprocess로 비교 receipt화하고,
  GitHub Actions DISABLED 및 alternate-rustc/other-evaluator/DDC tracks HELD
  fail-closed 정책을 `stage14-cross-impl-replay-check`로 검증. (2026-07-01)
- [x] **stage15 evidence federation seed**: `proofs/evidence-federation.tsv`로
  local proof/stage-manifest/DISABLED Actions 및 external evidence offline approval,
  fail-closed conflict policy를 receipt로 고정하고 `stage15-evidence-check`로 검증.
  (2026-07-01)
- [x] **stage15 full**: `proofs/evidence-replay.tsv`로 local-proof(stage14 cross-impl
  replay)와 stage-manifest를 clean subprocess로 replay하고, GitHub Actions
  DISABLED 및 external-web/external-tool/human-note HELD offline/review approval
  정책을 `stage15-evidence-replay-check`로 검증. (2026-07-01)
- [x] **stageN extension seed**: `proofs/extension-policy.tsv`로 versioned manifest,
  timeout/cost budget row, explicit migration, fail-closed 정책을 receipt로 고정하고
  `stageN-extension-check`로 검증. (2026-07-01)
- [x] **stageN full**: `proofs/extension-replay.tsv`로 manifest-index, timeout-cost,
  stageN-seed를 clean subprocess로 replay하고, breaking-change/external-law/future-stage를
  HELD + explicit migration/review + fail-closed 정책으로 고정.
  `stageN-extension-replay-check`로 검증. (2026-07-01)

### 5.5 Cross-cutting (상시 불변 — 매 슬라이스 확인)
- [x] 신규 기능마다 interp + native + TV 동시(lockstep) 추가:
  현재 `self/tv` positive 297와 stage proof matrix가 로컬 green. (2026-07-01)
- [x] negative corpus로 acceptance TV(interp 거부 ⟺ rustc 거부) 유지:
  현재 `typeck-check` negative 254 로컬 green. (2026-07-01)
- [x] **zero crates.io dep** 유지 (std만; rustc=툴체인):
  `constitution-check`가 `[dependencies]` 테이블 부재를 검증. (2026-07-01)
- [x] honesty: DONE/TODO/HELD 정확히 표기, 과대주장 금지:
  stage1/stage2는 checked DONE, stage3 full chain/B==C는 비용 경계 HELD로 고정. (2026-07-01)
- [x] machine-readable stage manifest index:
  `proofs/stage-manifest.tsv` + `manifest-check`로 stage/status/check/command/timeout/cost note 검증.
  (2026-07-01)
- [x] **CI disabled**: GitHub Actions workflow는 `.github/workflows.disabled/`로 이동해
  push/PR 자동 실행을 끔. 검증 기준은 로컬 `cargo build` + `bootstrap check`
  (self/tv/typeck/source-ast/source-bundle/stage2/stage3/stage8/manifest/isolation/constitution).
  (2026-07-01)
- [x] determinism seed: native tier 생성 파일명 content-hash(`cache_key(src)`,
  `prog_{:016x}`) 사용을 `constitution-check`로 검증. (2026-07-01)
- [x] 성능 seed: `native_run`은 content-hash rustc artifact cache를 재사용하고,
  `stage8` receipt 경로는 fresh compile을 유지. `native-cache-check`로 두 번째
  compile cache hit 검증. (2026-07-01)
- [x] 회귀: 매 커밋 `bootstrap check` green 유지, 커밋당 corpus 증가 기록:
  최신 local full check는 positive 297 / negative 254 / stage proofs green. (2026-07-01)

---

### 5.6 Plan 통합 로드맵 (2026-07-02, 사용자 Plan 문서 반영)

사용자가 제시한 "rs-meta: Rust Meta-Circular Compiler/Evaluator Plan"(17개 섹션)을
현재 상태와 대조해 통합한다. 핵심 원칙 3개는 이미 헌법과 일치: (1.1) meta-circular
은 mirror만이 아니다, (1.2) 인터프리터가 신뢰 floor, (1.3) native tier는 authority가
아니라 parity proof.

**이미 DONE (플랜 §13 대부분)**: impl/연관함수/메서드, use/mod/pub, const/static,
return/break/continue/while/loop/range/인덱싱/field shorthand/turbofish(부분),
패턴 전 계열(§3.4), typeck(§5), interpreter(§4), stage ladder(§8 → 우리 stage0~N),
zero-dep(§14), TV(§7), held 목록(§15 Phase 7 = 우리 §6).

**Phase E1 — emit 레이어 (§6, §13.4)**: AST → Rust source 재생성.
- [x] `src/emit.rs`: 전 AST variant 커버 emitter (aggressive-parens 전략,
  self-host subset 준수, source_files 편입). (2026-07-02)
- [x] `roundtrip-check`: parse→emit→reparse **AST 구조 동일**(derived Debug 비교)
  + interp(emitted)==expected. corpus **298/298 PASS**, 기본 `check` 편입. (2026-07-02)
- [x] AST emission-완전성 확장 1차: **top-level `use` 보존**(Program.uses, 파서
  canonical 재구성), **`#[derive(...)]` 보존**(StructDef/EnumDef.derives), 알려진
  std 타입 PathCall 역정규화(Rc/HashMap/PathBuf/ExitCode). emit-tv 268→284. (2026-07-02)
- [x] `emit-tv-check`: rustc(emit(parse(src)))==expected. **GROW 284/298** —
  manifest row GROW, 기본 `check` 제외(green 유지 원칙). (2026-07-02)
- [x] CLI `emit -c|-f` 추가.
- [x] **E1b** std 경로/타입 위치 emission (2026-07-03 해소): `fn main() -> ExitCode` 등 타입 위치
  qualification, `std::env`/`std::fs` 계열 (fully-qualified-* / env-* 4건).
- [x] **E1c** 제네릭 파라미터 AST 보존 (2026-07-03 해소): `fn id<T>` / `struct Wrap<T>` /
  `enum Opt<T>` / `impl<T>` — AST에 generic param 저장 + emit (4건).
- [x] **E1d** lifetime AST 보존 (2026-07-03 해소, RefLt+정규화 strip): `&'static str`, `Scope<'p>` (3건; typeck 등가
  비교 ripple 주의).
- [x] **E1e** impl Trait param 표면 emit 정합 (2026-07-03 해소 — 단순 인자 이름 보존).
- [x] emit-self-host (2026-07-03): all-source 번들을 emit으로 재생성 → rustc 컴파일 →
  print_help/corpus replay 동등 (source-bundle-check의 emit판). E1b~e 이후.

**Phase E2 — artifact/witness/hash 레이어 (§9, §11)**:
- [x] `hash.rs` (2026-07-03): source/token/AST/value/emit canonical hash
  (FNV 재사용, zero-dep).
- [x] `witness.rs` (2026-07-03): witness record(kind/stage/input_hash/output_hash/status/
  error_kind) + `proof/` 리포트 파일(self/tv/typeck/stage/native/drift).
- [x] drift report (2026-07-03): witness-check가 2-pass 비교로 기계가독 drift 판정.

**Phase E3 — trace/gate (§4.4, §12)**:
- [x] eval trace facets (2026-07-03; bind/call/match arm/loop/error 구현,
  error) — 옵션(성능), 기본 off.
- [x] gate/capability (2026-07-03): native compile/run/fs-write/subprocess 명시 게이트
  (현재 native tier 호출은 무게이트; can_compile_native 등).

**Phase E4 — span/diagnostic (§3.1)** — [x] v1 DONE (2026-07-03): 계획의 경고
(AST 전면 변경 회귀 위험)를 존중해 **구조 무변경 설계** — 파서 에러가 이미
담고 있는 토큰 인덱스("at token N")를 사후 매핑: lexer에 lex_spanned(토큰
병렬 오프셋 벡터; continue arm은 루프-톱 백필) + offset_line_col, diag.rs가
인덱스→오프셋→line/col + 소스 라인 + caret 렌더. 비-위치 에러는 무변경
통과. CLI(run/native-run/ast/emit/ast-canonical/witness/trace-run) 배선.
diag-check 6/6(매핑/caret/통과/결정성/EOF 엣지/typeck fn-단위). held:
expression-레벨 span(AST-wide — 계획 경고 그대로), lex 에러 자체 위치.
E4 v2(2026-07-03): typeck 에러의 `in fn NAME`를 소스 `fn NAME` 정의로 매핑
(경계 검사로 `fn mainish` 오매치 방지) — AST 무변경, 함수 단위 위치.
자기호스트 적중 4건(rfind/parse::<usize>/repeat/ends_with → subset 형태).

**플랜과의 차이(정직 기록)**: §16의 examples/*.rsmeta 확장자는 채택하지 않음 —
rs-meta의 대상 언어는 Rust이므로 `.rs` 유지(samples/*.rs). mirror는 이미 관측
표면(stage3-mirror-check)으로 구현됨(§10 일치). 파일명도 기존(check.rs 통합형)을
유지하되 신규 레이어(emit/hash/witness)는 플랜 §16대로 분리.

**다음 큰 갈래 (사용자 지시 예고)**: `~/pnix-rs/pnix-rs` — pnix-clj/pnix-hy의
제품 lane 대응물. rs-meta(증명 lane)가 기판이 된 뒤 시작. 별도 세션/지시에서.

### 진행 로그
- 2026-07-08 IntLit -- E0689 fidelity (워크트리 feat/intlit): fuzz가 찾은 경계를
  폐쇄. Type::IntLit(typeck 내부, 파서는 생성 안 함): 미주석 리터럴+리터럴-산술이
  IntLit로 타입 -> 아무 concrete 정수로 coerce, 바인딩/컨테이너에서 i64로 collapse,
  메서드 receiver면 E0689식 거부(rustc: 메서드 해석이 리터럴 fallback 선행).
  suffixed 리터럴(5i64)은 파스 시 Cast로 desugar(렉서 IntSuffixed 토큰) -> receiver
  concrete. 함정 3개 잡음: Neg on IntLit(보존), VecLit join(IntLit-aware, 끝에
  collapse -- vec![1u64, 2]), 이항 결과 순서(I64+IntLit -> I64 concrete 우선).
  잔여 문서화: let x=5; x.pow(2)는 accept(변수 리터럴성엔 실제 추론 필요). 완전한
  IntLit 엄격화(i32+i64 등)는 의도적으로 안 함 -- i<v.len() 같은 rustc-추론 패턴이
  깨져 self-host 붕괴(forward-only typeck의 정직한 한계). 전 게이트 PASS(self 358,
  fuzz/emi 포함). corpus +suffixed-literal-method, negative +literal-method-e0689.
- 2026-07-08 fuzz 표면 확장 (워크트리 feat/fuzz-surfaces): 신규 26 feature의 조합을
  differential로 검증하기 위해 생성 표면 8종 추가(bit/shift, 복합대입, labeled
  loop-값, int 메서드, Vec sort/dedup/contains, iter chain, array+HashMap 단일키,
  trait default+override). 첫 400 실행이 즉시 E0689 경계 발견: (7-4).signum() --
  interp은 리터럴을 i64로 타입해 accept, rustc는 메서드 해석이 리터럴 fallback보다
  먼저라 모호 거부. 생성기를 typed 리터럴로 수정(soundness-by-construction).
  이후 400 프로그램 발산 0. fuzz/emi-check PASS, 회귀 0.
- 2026-07-08 impl fmt::Display (워크트리 feat/impl-display): 마지막 큰 언어 feature.
  Formatter를 런타임 String 버퍼로 모델링 -- 조각들이 이미 있었음(impl T for A
  trait_name 추적, write! Expr::Write, write_string_handle이 Val::String 수용).
  추가: typeck write 타겟에 &mut fmt::Formatter 허용, fmt::Result<->Result<(),()>
  type_compatible, check_format_args가 {}에서 (타입,fmt) 메서드 있으면 허용;
  interp format_println {}가 Struct+fmt 메서드면 버퍼로 call_method 디스패치;
  emit이 std::fmt::Display/Formatter/Result로 qualify + 파서 canonical_known_path에
  역방향(std::fmt::* -> fmt::*, roundtrip). self-host 함정: &[] 빈 슬라이스 리터럴을
  자기해석이 &Vec<Unit>로 추론 -> 명시 Vec<Type> 로컬로. format! 안에서도 작동,
  Display 없는 struct {}는 여전히 정적 거부. 회귀 0(self 357->358, roundtrip 358).
  corpus +impl-display.
- 2026-07-08 str::find + Vec::chunks/sort_by_key + iter max/min_by_key (워크트리
  feat/methods3): find->Option<Usize>, chunks(n)->Iter<Vec<elem>>(수동 그룹핑),
  sort_by_key=키를 call_callable로 뽑아 stable insertion sort(retain식 가로채기),
  max_by_key는 rustc 의미론(마지막 max/첫 min) 준수. 핵심 수정: interp Deref를
  idempotent로 -- 값 모델이 ref를 조기 평탄화하므로(클로저 인자=값) 정적으로 유효한
  *x가 plain 값을 만나면 no-op(typeck는 잘못된 deref 여전히 정적 거부 -> 새 갭 없음,
  검증). typeck by_key 클로저 입력은 Ref{elem}(**x 타입체크). 회귀 0(self 353->357,
  self-host). corpus +str-find +iter-by-key +vec-sort-by-key +vec-chunks.
- 2026-07-08 HashMap keys/values + iter flat_map (워크트리 feat/hm-iter): keys()/
  values() -> Iter<&K>/Iter<&V>(기존 iter arm 미러, ref로). flat_map -> 클로저가
  Vec/Iter 반환하면 원소 flatten(typeck는 infer_expected_closure로 리턴 추론 후
  Vec/Iter 내부 타입 추출). corpus는 순서-무관(sum)으로 -- rustc HashMap 순회는
  랜덤 순서라 순서 의존 테스트는 발산함(주의점 기록). 회귀 0(self 351->353,
  self-host). corpus +hashmap-keys-values +iter-flat-map.
- 2026-07-08 Vec::retain + iter chain/step_by/rposition (워크트리 feat/iter-methods2):
  retain은 call_vec_method가 free fn(interp 없음)이라 call_method의 Vec dispatch에서
  가로채 self.call_callable로 predicate 호출(copy-out -> filter -> write-back).
  chain=두 iter items concat, step_by=n간격 샘플링(수동 while), rposition=뒤에서부터
  원래 인덱스(position 미러). typeck: chain/step_by->Iter<elem>, rposition->
  Option<Usize>, retain은 check_expected_closure(&elem->Bool). 회귀 0(self 348->351,
  self-host). corpus +vec-retain +iter-chain-stepby +iter-rposition.
- 2026-07-08 함수 본문 use 문 (워크트리 feat/use-in-fn): 블록 statement dispatch에
  KwUse arm -- parse_use_item으로 소비 후 드롭(파서가 std 경로를 bare 이름으로
  canonicalize + emit이 재-qualify하므로 subset에서 정보 없음). HashMap 인덱싱과
  합쳐 use HashMap 관용구가 통째로 작동. 회귀 0(self 347->348, self-host).
  corpus +use-in-fn.
- 2026-07-08 HashMap 인덱싱 m[&k] (워크트리 feat/hashmap-index): interp Index에
  HashMap 분기(normalized_key + entries.find, 키 없으면 런타임 에러 = rustc panic
  대응) + typeck Index에 Generic{HashMap,[K,V]} -> V(키 타입 deref 후 K 호환 검증).
  m[k]=v(lvalue)는 기존 typeck가 이미 거부 = rustc E0594(IndexMut 없음) 일치 --
  negative corpus로 고정. 회귀 0(self 346->347, typeck 263->264, roundtrip/emit-tv
  347, self-host). corpus +hashmap-index, negative +hashmap-index-assign.
- 2026-07-04 stdlib 메서드 6종 (워크트리 feat/more-methods): i64::signum/rem_euclid
  (pow식), iter::product(sum식 곱), Vec::truncate(dedup식 copy-out), String::
  to_uppercase/to_lowercase(repeat식). 전부 interp-safe 직접 추가. 회귀 0(self
  342->346, self-host). corpus +int-signum-remeuclid +iter-product +vec-truncate
  +str-case. (진행 루프 재개 -- feature 19 후 멈췄던 것 해결.)
- 2026-07-04 Vec::insert/extend (워크트리 feat/vec-insert-extend): 남은 Vec 메서드.
  insert(i,x)=copy-out->idx에 x 넣어 rebuild->write-back(idx>=len이면 끝에). extend
  (other)=other의 원소를 self에 push(vec_handle로 other 핸들). 전부 interp-safe 수동.
  회귀 0(self 340->342, self-host). corpus +vec-insert +vec-extend. (take/skip는
  이전에 이미 작동 확인.)
- 2026-07-04 generic-close >= 분리 (워크트리 feat/ge-split): let x: Vec<i64>=v(공백
  없음)에서 렉서가 >=를 Ge로 만들어 제네릭 close 실패했던 niche 갭. 파서 toks를
  &[Tok]->Vec<Tok>(owned)로 바꿔 mutate 가능하게 + eat_gt 헬퍼(Ge->>(소비)+= 로,
  ShrEq->>(소비)+>= 로 토큰 분리). 3개 제네릭-close 사이트 교체. 비교 >=/공백 있는
  것 안 깨짐. 회귀 0(self 339->340, self-host). corpus +generic-close-eq.
- 2026-07-04 collection 메서드 + enum Debug 버그 (워크트리 feat/collection-methods):
  Vec::dedup(수동, sort식) + iter min/max(Option 반환) 추가. min/max의 {:?} 출력이
  pre-existing 버그 표면화: interp이 enum을 Option::Some(1)/E::A(5)로 렌더(enum 이름
  prefix)했으나 rustc derive(Debug)는 Some(1)/A(5)(prefix 없음)+payload debug 렌더.
  debug_display에 Enum arm 추가 수정. corpus가 {:?} on Option/enum을 안 써서 잠복했음.
  회귀 0(self 336->339, self-host). corpus +vec-dedup +iter-min-max +option-debug.
- 2026-07-04 Vec::sort (워크트리 feat/vec-sort): deferred였던 것 완료. self-host
  함정(sort_by higher-order, Vec::swap 미지원) 회피: copy-out(.get()로 로컬 Vec에)
  -> 로컬 insertion sort(로컬 Vec 인덱스 대입은 interp 지원) -> write-back(.clear()
  +.push()). i64 키 기준 오름차순. 회귀 0(self 335->336, self-host PASS 첫시도).
  corpus +vec-sort. 이제 흔한/중간/hard Rust subset 전량 커버.
- 2026-07-04 iterator adaptor / ref 산술 (워크트리 feat/ref-arith): 마지막 hard feature.
  진단: .into_iter()/range의 .map().collect()는 이미 됐고, .iter().map(|x| x*2)만
  실패 -- x가 &i64라 x*2가 arithmetic on Ref 거부(iterator가 아니라 REF 산술 문제).
  rustc는 impl Add for &T로 &i64*2 허용. 수정: typeck type_binary에 deref_num_ref
  (정수/실수 ref auto-deref) + interp int2에 deref_value(operand). 이제 .iter().map/
  sum/collect + &n+1 작동. 회귀 0(self 332->335, self-host). corpus +iter-map-collect
  +ref-arithmetic +iter-map-sum.
- 2026-07-04 trait DEFAULT method (워크트리 feat/trait-default): 가장 큰 남은 feature.
  파서가 trait 본문을 파싱(전엔 skip) -> TraitDef{methods(default,body있음),
  decls(시그니처만)}. Program.traits, ImplBlock.trait_name 추가. 핵심 설계: setup
  시 impl이 override 안 한 trait default를 구현 타입의 메서드로 FLATTEN(interp+typeck
  둘 다, plain loop) -> dispatch 로직 불변. emit/sig가 trait+impl-for+decls를 faithful
  방출(roundtrip/emit-tv). self로 다른 trait 메서드 호출도 됨(flatten된 것끼리 self
  resolve). 회귀 0(self 329->332, roundtrip/emit-tv 332, self-host).
  corpus +trait-default +override +selfcall.
- 2026-07-04 stdlib 메서드 (워크트리 feat/stdlib-methods): str::repeat(->String),
  Vec::contains(->bool), std::cmp::max/min(->i64). interp call_vec/string_method +
  PathCall + 대응 typeck. Vec::sort는 처음 sort_by로 했으나 self-host 번들이 sort_by
  (higher-order) 미지원으로 거부 -> sort drop(interp-safe 수동 구현 필요, 후순위),
  contains도 .any 대신 수동 while로 재작성. 회귀 0(self 326->329, self-host).
  corpus +str-repeat +vec-contains +cmp-max-min.
- 2026-07-04 labeled break (워크트리 feat/labeled-break): Expr::Labeled{label,body}
  래퍼로 구현(loop 변형 자체는 불변). Break{label,value}로 변경, Signal::Break에
  Option<String> label. 각 loop가 labeled break(Some)는 re-propagate, unlabeled(None)는
  처리; Labeled가 자기 label의 break를 잡음. statement context는 starts_block에
  Lifetime 추가로 block-like 인식. continue 'label은 미모델(loop가 자기 label 알아야
  해서 -- 문서화). 회귀 0(self 324->326, roundtrip/emit-tv 326, self-host).
  corpus +labeled-break +labeled-break-value.
- 2026-07-04 associated const (워크트리 feat/assoc-const): impl S { const N: T = v; }.
  ImplBlock +consts:Vec<Global>. parse_impl에서 KwConst면 parse_global. interp/typeck
  셋업에서 Target::N 이름의 global로 등록. S::N은 EnumCtor{S,N}으로 파싱되므로
  interp/typeck EnumCtor eval에 globals fallback(enum 조회 앞). emit/sig에 impl const
  방출. 회귀 0(self 323->324, roundtrip/emit-tv 324, self-host). corpus +assoc-const.
- 2026-07-04 where 절 + turbofish (워크트리 feat/generics-syntax): fn/method의 ret와
  body 사이에서 where(식별자) 감지시 { 까지 스킵(bound는 { 없음). bare-fn turbofish
  id::<T>()는 :: 뒤 <면 parse_optional_turbofish로 소비+드롭 후 Call(method turbofish는
  이미 됐음). 둘 다 interp이 동적 타입이라 무시, emit 드롭(faithful). path(enum ctor/
  Type::method) 안 깨짐. 회귀 0(self 321->323, self-host). corpus +where-clause +turbofish.
- 2026-07-04 const fn + move closure (워크트리 feat/fn-modifiers): 아이템 dispatch에서
  KwConst 뒤 KwFn면 함수로 파싱(const 스킵). 렉서 KwMove 추가 + primary에서 move
  뒤 클로저 dispatch. 둘 다 modifier를 interp가 무시(const-eval/borrow 구분 없음),
  emit에서 드롭(roundtrip/emit-tv faithful). 회귀 0(self 319->321, self-host).
  move는 식별자로 안 쓰여 키워드화 안전. corpus +const-fn +move-closure.
- 2026-07-04 복합대입 <<= >>= &= |= (워크트리 feat/compound-assign): 렉서에 ShlEq/
  ShrEq/PipeEq 토큰 + compound_assign_op 매핑. 버그 수정: AmpEq가 BinOp::And(논리)로
  잘못 매핑돼 int &= 실패 -> BitAnd로. 파생 갭 발견: bit-ops(& | ^)가 정수만 -
  rustc는 bool & bool 허용, rs-meta 소스도 bool &= 사용(source-bundle 회귀로 노출).
  BitAnd/BitOr/BitXor typeck+interp에 bool 경로 추가(bool op -> bool). 회귀 0(self
  317->319, roundtrip/emit-tv 319, self-host). corpus +compound-bit-assign +bool-bitwise.
- 2026-07-04 shift << >> (워크트리 feat/shifts): parse_shift 레벨을 cmp와 add
  사이에 삽입. <<=두 Lt, >>=두 Gt(렉서가 안 합침) -- 연속 쌍 감지로 비교(<)와
  구별. 제네릭 Vec<Vec<i64>>는 타입 파서(별도 컨텍스트)라 안 깨짐. shift 양은
  다른 정수 타입 가능(i64<<u32)이라 전용 typeck arm(혼합-width 검사 우회).
  BinOp::Shl/Shr, ast/parser/typeck/interp/emit/sig 6층. 회귀 0(self 316->317,
  roundtrip/emit-tv 317, self-host). 흔한 Rust 연산자 전부 커버 완료.
- 2026-07-04 bitwise & | (워크트리 feat/bitops): parse_bitor(^ 아래)+parse_bitand(^
  위) precedence 레벨 추가. infix 위치가 prefix ref &x/closure |x|과 구별(모호성
  해소, 전 corpus 유지). signedness-무관(비트연산은 저장된 i64 비트에 동일).
  BinOp::BitAnd/BitOr, ast/parser/typeck/interp/emit/sig 6층. typeck는 기존 산술
  arm에 합류 -> 혼합-width(u32&u64) 자동 거부(differential 검사 재사용). shift는
  held(<< 렉서 Lt Lt, >> 제네릭 모호성). 회귀 0(self 315->316, typeck 262->263,
  roundtrip/emit-tv 316, self-host 첫시도). corpus +bit-and-or +neg int-mix-bitand.
- 2026-07-04 integer .pow() (워크트리 feat/int-methods): a.pow(b) 모든 정수 타입.
  interp+typeck에 arm 추가(is_int_runtime_target/is_int_target 가드). 중요: 처음엔
  saturating_add/abs도 추가했으나 u64.saturating_sub이 발산(interp -2 / rustc 0)
  발견 -- target=runtime_type_name(i64)이라 signedness 구별 불가. 정직하게 pow만
  남기고 revert(알려진 발산 안 만듦). 회귀 0(self 314->315, emit-tv 315, self-host).
  positive corpus 314->315. 교훈 준수: 이번엔 emit-tv를 별도 단계로 커밋 전 확인.
- 2026-07-04 array type annotation (워크트리 feat/array-type): [i64; N] 타입.
  리터럴/repeat/인덱싱은 이미 작동 -- parse_type가 [T]만이라 [T;N]의 ;에서 실패.
  Type::Array(Box<Type>, String) 추가(크기는 String, emit/roundtrip faithful; Expr는
  PartialEq 없어 String). type_compatible에 Array<->Vec/Slice, vec_index_elem에
  Array. 단 1개 exhaustive match(sig_type)만 확장. 회귀 0(self 313->314, roundtrip/
  emit-tv 314, self-host). positive corpus 313->314.
- 2026-07-04 loop-as-expression feature (워크트리 feat/loop-value): let r = loop
  { break v; }; -- interp은 이미 break 값 반환했고 typeck만 HELD였음. TypeCk에
  loop_break_types 스택 추가 -> loop 타입 = break 값 타입(중첩 loop는 innermost).
  plain break;는 unit loop 유지. 회귀 0(self 312->313, typeck 262, roundtrip/
  emit-tv 313, self-host 첫시도 PASS). positive corpus 312->313.
- 2026-07-04 tuple struct feature (워크트리 feat/tuple-struct): struct P(T0,T1); +
  P(a,b) 생성(Call 경로) + p.0/p.1 접근(TupleIndex, is_tuple_struct 가드).
  StructDef +tuple 플래그, positional 필드를 이름 "0"/"1"로. regular struct .0은
  거부(정밀). 회귀 0(self 311->312, typeck 262, roundtrip/emit-tv 312, self-host).
  self-host 규율이 unwrap_or_default 잡음 -> match로. positive corpus 311->312.
- 2026-07-04 unit struct feature (워크트리 feat/unit-struct): struct D; 파싱 +
  bare D 생성(unit struct만, struct D {}는 여전히 braces 강제 -- 갭 없음).
  StructDef +unit 플래그, parser/typeck/interp/emit/sig 5층. 회귀 0(self 310->311,
  typeck 262, roundtrip 311, emit-tv 311, source-bundle self-host, source-ast 14).
  positive corpus 310->311. interp이 HashSet 미지원이라 Vec+plain-loop 헬퍼로.
- 2026-07-04 2번째 BUG FIX (differential payoff): 혼합 width 정수 산술(u32+u64,
  i32+u32)을 interp이 accept했으나 rustc 거부. type_binary에서 두 operand가
  다른 정수 타입이고 둘 다 I64(flexible 리터럴) 아니면 거부. 회귀 0(self 310/
  typeck 259/roundtrip 310/emit-tv 310/self-host), negative 259->262(비교 포함). 잔여 갭:
  i32+i64(I64가 리터럴+concrete 겸용이라 구별 불가 -- untyped-int 타입 필요, 문서화).
- 2026-07-04 REAL BUG FOUND+FIXED (differential payoff): interp이 non-unit 리턴
  타입인데 body가 fall-through(() 반환)하는 함수(fn f()->i64{}, {let y=3;})를
  accept했으나 rustc는 거부. typeck.rs block_falls_through 발산 분석으로 수정
  (제어흐름은 diverge로 간주 -> {return 5;} 정당 함수는 안 깨짐). 회귀 0(self
  310/typeck 257/roundtrip 310/emit-tv 310/self-host), negative corpus 257->259
  regression guard. 규율이 end-to-end 작동 실증.
- 2026-07-04 boundary-check — 알려진 interp!=rustc 경계 지도(differential payoff).
  const-overflow(interp wrap/rustc 거부=held const-overflow-lint) + div-by-zero
  (둘 다 거부) 명시+drift 추적, 대조로 경계가 특정적임 확인. 3/3. default check
  65->66.
- 2026-07-04 selfhost-audit (deep-research open Q #4). rs-meta 코어 7파일
  (lexer/ast/parser/typeck/interp/sig/hash)이 held-feature blocker 0(macro_rules/
  async/unsafe/trait-def) -> 어떤 held 기능도 코어 self-host를 안 막음(mrustc식).
  진짜 남은 작업 = full-chain 비용(stage3-full-chain DONE·budget-gated), held
  기능 아님. selfhost-audit-check(2/2, drift-resistant) + docs/self-hosting.md.
  default check 63->64.
- 2026-07-04 emi-metamorphic (deep-research #2: EMI/Orion PLDI14). emi-check(2/2):
  죽은 분기(if false)에 관측 가능 코드 주입 -> interp & rustc stdout 불변(4 변이);
  teeth = live 주입(if true)은 출력 변경(non-vacuous). lowering/dead-code 처리
  stress. source-bundle/ast 유지. default check 61->62.
- 2026-07-04 fuzz-diff (deep-research #1: differential testing, Csmith/PLDI11) —
  방향 상실 해소의 핵심 답. 결정적·잘-정의된 Rust 생성기(bounded 산술+let+if+match+struct+enum+helper fn/call,
  overflow/division/nondeterminism 회피) + fuzz-check(2/2): 42개 생성 프로그램
  전부 interp-stdout == rustc-stdout, 생성기 결정성. 발산=interp/rustc 버그
  국소화 -> 코퍼스 자동 성장(증명기 불필요). source-ast 14/14(fuzzer 자체가
  self-hostable). default check 60->61. 다음: 생성기 표면 확장(fn/match/struct)
  + EMI metamorphic(#2, dead-branch mutation stdout 불변).
- 2026-07-04 tv-stats — translation-validation corpus 커버리지(positive/negative
  카운트)를 pnix-rs 엔진 신뢰 증명용으로 노출. tv-stats 커맨드+tv-stats-check(2/2).
  pnix-rs engine-attestation이 소비. source-ast 14/14. default check 59->60.
- 2026-07-04 rust-surface — per-program trait+macro surface 분류(rust_surface_report
  + rust-surface 커맨드, parse-based). rust-surface-check(4/4): 지원/held-macro-
  rules/held-assoc-type/held-dyn-trait. pnix-rs verdict.surface가 이걸 소비
  (rs-meta가 분류 소유). source-ast 14/14. default check 58->59.
- 2026-07-03 rust-artifact (pnix-rs peer-engine 6순위) — stage8-repro receipt를
  per-program으로 노출. rust-artifact 커맨드 + rust-artifact-check(2/2): source_fnv
  +rustc+flags+artifact_fnv(바이너리 hash)+receipt_hash, 재현 가능+source별 상이.
  pnix-rs engine-artifact가 이걸 .px 봉투로. rs-meta pnix-free. default check 57->58.
- 2026-07-03 boundary reports (pnix-rs peer-engine 3~5순위) — rs-meta 자체 기능
  (pnix-free). borrow-boundary-check(3/3): interp은 ownership 미모델, rustc가
  oracle; borrow 위반은 held-borrow-not-modeled + rustc reason code(E0382/E0502)
  보존(정직한 갭). trait-boundary-check(3/3): 지원(inherent/trait dispatch) vs
  held(assoc-type/dyn/where/blanket), 분류가 실제 parse 경계와 합치(teeth).
  macro-boundary-check(3/3): fixed macro vs held-macro-rules(lex 경계 \$)/proc/
  derive. 세 report 모두 subset 무결(source-ast 14/14). default check 54->57.
- 2026-07-03 ast-diff (rs-meta) — pnix-rs ir-diff의 rs-meta 대응. mirror-proven
  ast-canonical 위 semantic diff: check::ast_diff(두 Rust 프로그램의 ast-canonical
  비교, 첫 차이 국소화). ast-diff-check(4/4): 동일->동일, 의미 변경->diff 국소화,
  구조 변경->diff, local rename->diff(정직 경계: ast-canonical은 faithful, 알파-
  정규화 아님). subset-safe(char 스캔). default check 53 게이트.
- 2026-07-03 ast-canonical faithfulness(제네릭): E1c가 emit을 제네릭-완전하게
  만들었으나 sig.rs는 제네릭 파라미터를 소거하고 있었음(fn id<T> → fn id,
  T는 named 타입과 구별 불가) — ast-canonical이 제네릭 표면에서 비단사.
  sig_generics 추가로 fn/struct/enum/impl/method에 `<T,U>` 직렬화. 비제네릭
  sig는 byte-identical 불변(factorial 확인) → 미러 3-way·roundtrip·emit-tv
  전부 무영향, source-ast/bundle green. ast-canonical-check 4/4(제네릭 fn
  `<T>`/다중 파라미터/injectivity/제네릭 struct). pnix-rs P6도 무영향(29/29).
  default check 52 게이트.
- 2026-07-03 Phase E3 DONE — (1) src/cap.rs 능력 게이트: 인터프리터 플로어는
  능력 0으로 동작(증명됨), native tier(rustc 컴파일/아티팩트 실행/subprocess
  프로브/workdir 쓰기)는 명시 게이트 + RSMETA_CAPS 제한 시 fail-closed(안정
  메시지). 기본 = 전부 허용(로컬 도구 정책, 문서화 — pnix-rs의 fail-closed
  기본과 의도적으로 다름: rs-meta 자신이 호스트 도구). fs-read는 비게이트
  경계 문서화. (2) interp 트레이스 facet: bind/call/match-arm/loop/error,
  기본 off(불 검사 1회), 결정성·커버리지·기본-off·에러-facet 프로브
  (trace-check 5/5); expr-레벨 enter/exit는 비용으로 held. 자기호스트 적중
  2건(char split 분리자, interp::/native:: normalize 접두사). default check
  50 게이트.
- 2026-07-03 Phase E2 DONE — src/hash.rs(FNV 통합: native.rs/check.rs 사본
  제거) + src/witness.rs(facet 증인: source/tokens/ast(sig 캐논)/emit/value,
  에러 facet 포함 — 침묵 구멍 없음). witness CLI + witness-check(결정성
  2-pass, 부정 corpus 에러 facet, 스키마 헤더, proof/witness-report.tsv;
  567 프로그램 2800 레코드). native facet은 stage8 영수증의 기관으로 분리
  유지(비용·직교성). 자기호스트 적중 5건 교체([&str;N] const, 번들 err 이름
  충돌, Result::and_then ×2, String::lines) + normalize에 crate::/hash::/
  witness:: 접두사. source_files 12, default check 편입.
- 2026-07-03 emit-self-host DONE — 방출된 전체 all-source 번들이 rustc로
  컴파일되어 (1) 원본과 동일 동작(print_help 경로), (2) **방출된 evaluator가
  corpus 310을 동일 재생**. 막힌 지점은 hex 리터럴(0xcbf29ce484222325)이
  i64 랩 음수로 방출되던 것 — Float(String) 선례로 **IntHex(i64, String)**
  (값=인터프리터 의미, 텍스트=rustc 의미) 도입. manifest DONE 행 추가,
  default check에 emit-tv + emit-self-host 편입.
- 2026-07-03 E1b~E1e 전량 해소 — emit-tv 310/310, GROW→DONE 승격 + default
  check 편입. E1c 제네릭: 정의부 generics 필드(derives 선례) 캡처·방출.
  E1b std 경로: 타입 위치·EnumCtor 경로에 qualify_type_name 적용 + fs/env
  모듈 수식. E1e impl-trait 인자: 단순 인자를 이름에 보존(Into<String>).
  E1d 라이프타임: **RefLt 변형**(파서 전용, 방출 보존) + typeck 진입 정규화
  (resolve_aliases)에서 Ref로 strip — 기존 77개 Ref 사이트 무접촉; 제네릭
  라이프타임 인자는 Named("'a") 캐리어 + 정규화 필터(placeholder Generic
  보존 조건 주의 — 회귀 4건을 self-check가 즉시 검출). 자기호스트 경계 2회
  적중(starts_with(char), slice join — substrate가 검출, subset 형태로 교체).
- 2026-07-02 f64 슬라이스: evaluated subset에 float 추가 — Tok::Float/
  Expr::Float(String, 소스 텍스트 보존으로 emit byte-왕복)/Type::F64/Val::F64.
  float 리터럴(digits.digits), 산술 + - * /(float×float만), 비교, 단항 -,
  i64↔f64 캐스트, "s".parse::<f64>(), {}(Display)·{:?}(Debug) 포맷.
  corpus 298→310(float 12종: debug/display/arith/div/cmp/cast×2/fn/parse/
  neg/let-mut), negative 124→127(1.5+1, let i64=3.5, 3.5<3 — rustc 거부 대조).
  self/tv/typeck/roundtrip 310 green; emit-tv 296/310(기존 held 14 유지);
  전 self-host 체인(bundle/stage2/stage3 mirror/fixedpoint/core-mini) green —
  **subset-경계 교훈: 인터프리터 자신의 새 f64 코드가 &f64 참조 연산을 써서
  L2에서 거부됨 → 명시 deref(let x = *a)로 subset 내 재작성**(수요처: pnix-rs
  px runtime floats(c01) — 이 lane은 그 존재를 모름, 범용 기능).
- 2026-07-02 ast-canonical: canonical AST serializer를 proofs/mirror-sig.rs에서
  `src/sig.rs`로 승격하고 `ast-canonical -c|-f` CLI로 노출 — `ast`(rustc derive
  Debug, 안정성 무보장)의 기계-파싱 가능 대안. 안정성 근거 = stage3-mirror-check
  (같은 직렬화가 3-레벨 byte-identical). mirror harness는 src/sig.rs를 정규화해
  읽음(use crate:: 필터). source_files 10개로 확장, 전 게이트 green
  (source-ast 10/10, bundle/stage2-chain/mirror/fixedpoint/roundtrip 298 유지).
- 2026-07-02 B/stage3-full-chain DONE: value_eq Vec fidelity 수정 후 full all-source
  stage2→stage2' 전체 corpus replay가 release에서 2103s PASS (interp == rustc).
  manifest row DONE(budget-gated 3600s), stage3-full-held-check를 DONE boundary
  가드로 재정의. 전체 aggregate check PASS(765s, FAIL 0) 후 승격.
- 2026-07-02 interp/value_eq: (Val::Vec, Val::Vec) ref-aware arm 추가 — depth-3
  자기해석에서 clone된 Vec<enum> slice-vs-array 등가가 derived PartialEq 추락으로
  깨지던 fidelity 버그 수정. 회귀 corpus clone-vec-enum-slice-eq (298).
- 2026-07-02 B/stage3-mirror + B/stage3-fixedpoint: 손수 작성한 canonical AST
  serializer(`proofs/mirror-sig.rs`, 전체 AST variant 커버)를 evaluator-core 번들에
  proof-harness로 붙여, 같은 serializer 소스가 stage1(native)/stage2/stage2'에서
  `samples/mirror_probe.rs`의 canonical AST + 실행 출력을 byte-identical하게 산출
  (`stage3-mirror-check`). 같은 transcript로 stage2(B) == stage2'(C) 정규화 비교
  고정(`stage3-fixedpoint-check`). derive Debug는 interp 렌더링이 rustc와
  byte-faithful하지 않음을 실측해 배제. release 빌드에서 mirror 첫 실행 ~51s
  (rustc cold), fixedpoint ~12s (캐시 워밍). 두 체크 모두 기본 `check` aggregate,
  CLI, manifest rows에 편입. full all-source 삼중 중첩 B==C는 여전히 HELD.
- 2026-07-02 release-빌드 관찰: 중첩 평가(stage3-core-mini)가 dev 44.6s → release
  10.2s (~4.4배). full all-source chain은 release로도 장시간(>16min CPU에서 계속) —
  비용 경계 실측 계속.
- 2026-07-02 local check sweep: 297 self-check, 297 tv-check, 254 typeck-check, stage-status/manifest/isolation/constitution/actions-disabled 모두 PASS. stage1~stageN proof seeds/replays PASS; stage3/9 held rows preserved. (2026-07-02)
- 2026-06-30 slice1: Rust subset interp + rustc native tier + TV (self/tv 13).
- 2026-06-30 typeck: 가벼운 타입검사 + acceptance TV(negative 6).
- 2026-06-30 data: struct/enum/tuple/match (corpus 20).
- 2026-06-30 loops: while/loop/for + break/continue + mut/대입 (corpus 26, negative 8).
- 2026-06-30 A1: impl/method/associated fn (corpus 30, negative 12).
- 2026-06-30 A2: refs/deref/mut-ref assignment (corpus 34, negative 16).
- 2026-06-30 A3-1: Vec core (corpus 38, negative 20).
- 2026-06-30 A3-2: String/&str core (corpus 42, negative 24).
- 2026-06-30 A3-3: Option/Result core (corpus 46, negative 28).
- 2026-06-30 A3-4: Box/Rc/RefCell core (corpus 50, negative 32).
- 2026-06-30 A3-5: HashMap core (corpus 54, negative 36).
- 2026-06-30 A8/A10 surface: attrs/pub/use/mod accepted (corpus 55, negative 36).
- 2026-06-30 A10: char/casts/numeric type surface (corpus 60, negative 41).
- 2026-06-30 A3-1b: Vec pop/get/last/index-write (corpus 63, negative 44).
- 2026-06-30 A3-1c: Vec foreach + string `.to_string` (corpus 66, negative 46).
- 2026-06-30 A7: `?` Option/Result early-return (corpus 69, negative 48).
- 2026-06-30 A6: match guard `pat if cond =>` (corpus 70, negative 49).
- 2026-06-30 A10: compound assignment lowering (corpus 71, negative 50).
- 2026-06-30 A6: prelude enum patterns Some/None/Ok/Err (corpus 72, negative 51).
- 2026-06-30 A10: assignment expressions in match/block expression positions (corpus 73, negative 52).
- 2026-06-30 A10: return expressions + `Never` type absorption (corpus 74, negative 53).
- 2026-06-30 A9: fixed `format!` macro with `{}` placeholder count checking (corpus 75, negative 54).
- 2026-06-30 A6: reference patterns `&pat`/`&mut pat` (corpus 76, negative 55).
- 2026-06-30 A10: Vec range slicing syntax/value model (corpus 77, negative 56).
- 2026-06-30 A5: typed closure literals + value capture + calls (corpus 78, negative 57).
- 2026-06-30 A10/A3: method turbofish + `parse::<i64>()` Result model (corpus 79, negative 58).
- 2026-06-30 A6: string literal patterns for `match &str` (corpus 80, negative 59).
- 2026-06-30 A10: slice type `[T]` / `&[T]` and `&Vec<T>` call compatibility (corpus 81, negative 60).
- 2026-06-30 A6/A10: struct-like enum variant definitions parsed (corpus 82, negative 60).
- 2026-06-30 A10: lifetime surface parsed/erased (`'a`, `&'a T`, generic lifetime params) (corpus 83, negative 60).
- 2026-06-30 A9: fixed `matches!` macro with pattern guard (corpus 84, negative 61).
- 2026-06-30 A10: struct literal field shorthand `S { x }` (corpus 85, negative 62).
- 2026-06-30 A10: semicolon-free `return expr` statement form (corpus 86, negative 62).
- 2026-06-30 A10: integer literal suffixes i64/i32/u32/u8/usize (corpus 87, negative 63).
- 2026-06-30 A6: or-pattern `a | b` (corpus 88, negative 64).
- 2026-06-30 A5: zero-arg closure and closure return annotation typeck (corpus 89, negative 65).
- 2026-06-30 A6/A10: struct-like enum variant literals `E::V { x }` (corpus 90, negative 66).
- 2026-06-30 A6: `if let pat = expr { ... } else { ... }` lowered through match (corpus 91, negative 67).
- 2026-06-30 A5: immediate closure/expression calls `(|| { ... })()` (corpus 92, negative 68).
- 2026-06-30 A6: destructuring `let pat = expr;` for tuple/enum/ref patterns (corpus 93, negative 69).
- 2026-06-30 A5/A6: closure parameter patterns `|(a, b): (T, T)| ...` (corpus 94, negative 70).
- 2026-06-30 A6: struct-like enum/struct field patterns `E::V { x }` / `S { x }` (corpus 95, negative 71).
- 2026-06-30 A6: foreach pattern binding `for pat in iter` (corpus 96, negative 72).
- 2026-06-30 A6: match block arms without comma before tuple/or-pattern arms (corpus 97, negative 72).
- 2026-06-30 A10: array literal surface `[a, b]` using Vec-backed value model (corpus 98, negative 73).
- 2026-06-30 A8/A10: top-level `type Name = ...;` alias surface accepted; later upgraded to typeck resolution.
- 2026-06-30 A4/A10: `impl Trait` type surface parsed, wildcard-compatible; trait solving held (corpus 100, negative 74).
- 2026-06-30 A9/A10: `panic!`/`unreachable!`/`todo!` fixed macros parsed as `Never` (corpus 101, negative 74).
- 2026-06-30 A10: Rust string line-continuation escape (`\` + newline) (corpus 102, negative 75).
- 2026-06-30 A8/A10: multi-segment path call parsing `std::env::args()` (parser surface; corpus unchanged).
- 2026-06-30 A10: bool `&=` compound assignment (corpus 103, negative 76).
- 2026-06-30 A9: `eprintln!` fixed macro; stderr ignored for stdout TV (corpus 104, negative 77).
- 2026-06-30 A8/A10: multi-segment path type parsing `check::Report` (parser surface; corpus unchanged).
- 2026-06-30 A9: `print!` fixed macro without trailing newline (corpus 105, negative 78).
- 2026-06-30 A11: `source-ast-check` gate 추가; `src/*.rs` 7/7 AST parse OK.
- 2026-06-30 A5/A3: narrow iterator model `str.chars().collect()` via Vec identity collect (corpus 106, negative 79).
- 2026-06-30 A6: one-layer match ergonomics for literal/destructuring patterns over refs (corpus 107, negative 80).
- 2026-06-30 A2: rvalue refs `&expr`/`&mut expr` temporaries 지원; immutable-place `&mut` 거부 유지 (corpus 109, negative 80).
- 2026-06-30 A5/A3: `Vec::iter()` + narrow `Iter::collect()` (`&char` → `String`) (corpus 110, negative 81).
- 2026-06-30 A5: narrow `Iter::filter(|x| bool)` with expected closure param inference (corpus 111, negative 82).
- 2026-06-30 A3/A7: `Result::map_err` with expected closure param inference + `?` path (corpus 112, negative 83).
- 2026-06-30 A9/A4: `{:?}` debug placeholder for print/println/format fixed macros (corpus 113, negative 83).
- 2026-06-30 A3/A10: slice `.get(index)` type surface for `&[T]` (corpus 114, negative 84).
- 2026-06-30 A3: `Option<&T>::copied()` for get/last-style references (corpus 115, negative 85).
- 2026-06-30 A1/A10: generic impl target names accepted (`impl<T> Type<T>` surface; monomorphization held) (corpus 116, negative 85).
- 2026-06-30 A10: integer comparison compatibility approximates unsuffixed literal inference (`usize < 2`) (corpus 117, negative 85).
- 2026-06-30 A2/A10: mutable struct field assignment/compound assignment places (corpus 118, negative 86).
- 2026-06-30 A3-2: string-like equality (`String`/`&str`) in typeck/runtime (corpus 119, negative 87).
- 2026-06-30 A3-3: `Option<&T>::cloned()` for parser source probes (corpus 120, negative 88).
- 2026-06-30 A10: narrow `usize::saturating_sub` for parser source probes (corpus 121, negative 89).
- 2026-06-30 A3/A7: `Option::ok_or_else` with zero-arg closure error construction (corpus 122, negative 90).
- 2026-06-30 A3/typeck: `None` placeholder assignment joining for later `Some(T)` (corpus 123, negative 91).
- 2026-06-30 A3-2/typeck: string-like `&String` accepted for `push_str`/comparison surfaces (corpus 124, negative 92).
- 2026-06-30 A5/A3: `Vec::into_iter()` snapshot iterator surface (corpus 125, negative 93).
- 2026-06-30 A5/A3: stateful `Iter::next()` with mutable/temporary receiver check (corpus 127, negative 94).
- 2026-06-30 typeck: tuple structural compatibility, including nested empty Vec placeholders (corpus 128, negative 95).
- 2026-06-30 A4: selected built-in `.clone()` surface with deep String/Vec/AST value clone (corpus 130, negative 96).
- 2026-06-30 typeck: `break`/`continue` typed as Never for match/loop absorption (corpus 131, negative 96).
- 2026-06-30 A3/A5: string-like `Vec::join(separator)` surface (corpus 132, negative 97).
- 2026-06-30 A3: `Vec::remove(index)` with mutable receiver check (corpus 133, negative 98).
- 2026-06-30 typeck: blocks with trailing diverging statement type as Never (corpus 134, negative 98).
- 2026-06-30 A5: `Iter::map` with expected closure param inference (corpus 135, negative 99).
- 2026-06-30 A11: `src/ast.rs + src/lexer.rs + src/parser.rs` concat bundle typechecks/eval-inits under rs-meta; expected stop is `interp: no fn main`.
- 2026-06-30 A6: struct/struct-like enum pattern rest `S { x, .. }` / `E::V { x, .. }` (corpus 137, negative 100); typeck bundle advances to `Box::as_ref`.
- 2026-06-30 A3-4: `Box::as_ref` / `Rc::as_ref` immutable reference surface (corpus 139, negative 102); typeck bundle advances to `Option<&Type>` vs `Option<Type>` return join.
- 2026-06-30 A4/typeck: `.clone()` on `&T` returns cloned `T` (except `&str` surface), matching self-host `Option<&Type>` → `Option<Type>` paths (corpus 140, negative 102); typeck bundle advances to `Option::map`.
- 2026-06-30 A3/A5: `Option::map` with expected closure inference + ref match ergonomics binding mode + `&Box<T>`/`&Rc<T>` deref call coercion (corpus 143, negative 104); typeck bundle advances to `Iter::zip`.
- 2026-06-30 A5: `Iter::zip` for iterator pairs (corpus 144, negative 106); typeck bundle advances to `Iter::all`.
- 2026-06-30 A5: `Iter::all` / `Iter::any` predicate adapters (corpus 146, negative 108); typeck bundle advances to mutable `Vec::push` receiver detection.
- 2026-06-30 typeck/A3: mutable field method receiver detection via `place_type`, `Vec::push` uses placeholder-aware `type_compatible`, and `&String` → `&str` call coercion (corpus 149, negative 110); typeck bundle advances to `Iter::rev`.
- 2026-06-30 A5/A3: `Iter::rev` + `HashMap<String,_>` lookup by `&str` (corpus 151, negative 111); typeck bundle advances to `Vec::last_mut`.
- 2026-06-30 A3: `Vec::last_mut` mutable receiver/type surface for self-host scope stacks (corpus 152, negative 112).
- 2026-06-30 A3: `Vec::with_capacity` associated constructor surface (corpus 153, negative 113).
- 2026-06-30 A5: `Iter::enumerate` adapter returning `(usize, item)` tuples (corpus 154, negative 114).
- 2026-06-30 A5: `Iter::find` predicate adapter returning `Option<Item>` (corpus 155, negative 116).
- 2026-06-30 A10/A5: slice `.iter()` type surface reusing Vec iterator runtime (corpus 156, negative 117).
- 2026-06-30 A10: slice indexing `xs[i]` type surface for `&[T]` (corpus 157, negative 118).
- 2026-06-30 typeck/runtime: unary `!` supports bool/int plus one-layer refs like rustc (corpus 159, negative 118).
- 2026-06-30 typeck: `Option` placeholder comparison joins `None`/`Option<&T>` without allowing direct mismatched ref equality (corpus 160, negative 119).
- 2026-06-30 A11: `ast+lexer+parser+typeck` concat bundle now reaches expected `interp: no fn main`
  under rs-meta (parse/typeck/eval-init pass). Next self-host blocker moves into `interp.rs` integration.
- 2026-06-30 A10/A4: narrow `impl Into<String>` `.into()` method support for self-host `err(msg)` (corpus 161, negative 120).
- 2026-06-30 A10/typeck: type aliases preserved/resolved + `Rc<RefCell<T>>.borrow(_mut)` deref-method surface for `Slot` (corpus 162, negative 121).
- 2026-06-30 A2/typeck: `&&T` to `&T` call coercion for self-host match helpers (corpus 163, negative 122).
- 2026-06-30 A3/A5: `Result::map` closure adapter (corpus 164, negative 123).
- 2026-06-30 A3/typeck: narrow `Rc<Vec<T>>.len/is_empty` inner Vec method surface (corpus 165, negative 124).
- 2026-06-30 A3/typeck: narrow `Rc<Vec<T>>.iter` inner Vec method surface (corpus 166, negative 125).
- 2026-06-30 A3/typeck: narrow `Rc<String>.as_str` inner String method surface (corpus 167, negative 126).
- 2026-06-30 A5: `Iter::position` predicate adapter returning `Option<usize>` (corpus 168, negative 128).
- 2026-06-30 A3/A5: `Option::and_then` closure adapter returning `Option<U>` (corpus 169, negative 130).
- 2026-06-30 A3/typeck: narrow `Rc<Vec<T>>.get` inner Vec method surface (corpus 170, negative 132).
- 2026-06-30 A6/typeck: tuple enum variant constructor values as callables (`map_err(Signal::Error)`) (corpus 171, negative 134).
- 2026-06-30 A10/typeck: `i64::saturating_sub` integer method surface (corpus 172, negative 135).
- 2026-06-30 A10/typeck: `i64::max` integer method surface (corpus 173, negative 136).
- 2026-06-30 A10/typeck: function return type alias comparison uses resolved aliases (corpus 174, negative 137).
- 2026-06-30 A3/A5/typeck: `Vec::iter_mut` read-observation surface returning `Iter<&mut T>` (corpus 175, negative 139).
- 2026-06-30 A3/typeck: recursive generic placeholder compatibility (`Rc<Vec<_>>` vs `Rc<Vec<T>>`) (corpus 176, negative 140).
- 2026-06-30 A5: `Iter::copied` / `Iter::cloned` adapters for reference iterators (corpus 178, negative 142).
- 2026-06-30 A3/A5: `String::chars()` now returns `Iter<char>`, so `chars().map(...).collect()` advances interp self-host source integration (corpus 179, negative 143).
- 2026-06-30 A3/typeck: narrow read indexing on `Rc<Vec<T>>` for self-host enum/tuple payload access (corpus 180, negative 144).
- 2026-06-30 A3/typeck: unannotated `Vec::new()` placeholder refines on `.push(T)`, while explicit `Vec<()>` stays strict; advances self-host `parts.join(...)` paths (corpus 181, negative 145).
- 2026-06-30 A3/A10: `Vec`/slice `.to_vec()` snapshot surface for self-host iterator state copies (corpus 182, negative 146).
- 2026-06-30 A10/typeck: `i64::min`/`usize::min` integer method surface for self-host iterator zip length calculation (corpus 183, negative 147).
- 2026-06-30 A5/typeck: top-level function items are callable values, enabling adapters like `.map(deref_value)` (corpus 184, negative 148).
- 2026-06-30 A3/A5: `Vec::reverse()` mutable method surface for self-host `Iter::rev` implementation (corpus 185, negative 149).
- 2026-06-30 A10/typeck: tuple index assignment places (`p.1 = value`, including `&mut tuple`) for self-host HashMap entry update (corpus 187, negative 151).
- 2026-06-30 A10: `char::from_u32` associated function surface for self-host char cast path (corpus 188, negative 152).
- 2026-06-30 A4/A5: `impl Fn(...) -> T` parameter surface is preserved as a closure type, enabling self-host `cmp(..., f)` helper (corpus 189, negative 153).
- 2026-06-30 A10/parser: Rust cast precedence fixed (`*r as u32` = `(*r) as u32`, while `r as i64` is rejected) (corpus 190, negative 154).
- 2026-06-30 A10/String: integer `.to_string()` surface for self-host value display paths (corpus 191, negative 155).
- 2026-06-30 A10/String: bool `.to_string()` surface for self-host value display paths (corpus 192, negative 156).
- 2026-06-30 A3/A10: Box/Rc `.to_string()` delegates displayable inner values for self-host value display paths (corpus 193, negative 157).
- 2026-06-30 A10/typeck: lifetime-only generic types (`Scope<'p>`) are compatible with their named runtime type (corpus 194, negative 157).
- 2026-06-30 A3: `HashMap::iter` yields `Iter<(&K, &V)>` for self-host scope snapshots (corpus 195, negative 158).
- 2026-06-30 A2/A3: recursive deref coercion supports `&Rc<String>` -> `&str` function args (corpus 196, negative 159).
- 2026-06-30 A10/typeck: slice/array equality supports `&[T]` vs array/Vec literal model for self-host turbofish checks (corpus 197, negative 160).
- 2026-06-30 A3: `Option::or_else` with zero-arg fallback closure for self-host fallback lookup paths (corpus 198, negative 161).
- 2026-06-30 A5: `bool::then` zero-arg closure adapter returns `Option<T>` for self-host optional branches (corpus 199, negative 162).
- 2026-06-30 A10: `i64::wrapping_neg`/`i32::wrapping_neg` integer method surface for self-host unary negation paths (corpus 200, negative 163).
- 2026-06-30 A10/typeck: `loop` without syntactic `break` is typed as `Never`, allowing return-only self-host loop arms (corpus 201, negative 163).
- 2026-06-30 A10/parser: block-started statement boundary prevents `if { ... } *x` from binding as binary multiplication before deref assignment (corpus 202, negative 163).
- 2026-06-30 A3: `Vec::get_mut` surface with mutable receiver checking for self-host assignment paths (corpus 203, negative 164).
- 2026-06-30 A10: integer `wrapping_add/sub/mul/div/rem` methods for self-host arithmetic paths (corpus 204, negative 165).
- 2026-06-30 A5: expected-type inference for untyped closure params in `impl Fn` argument positions (corpus 205, negative 166).
- 2026-06-30 A5/typeck: let annotation guides `Iter<char>::collect()` into `Vec<char>` for self-host `format_println` (corpus 206, negative 167).
- 2026-06-30 A3-4/A5: `Rc<String>.chars()` auto-deref surface for self-host `coerce_let_value` (corpus 207, negative 168).
- 2026-06-30 A11: `src/ast.rs + src/interp.rs` concat bundle now reaches expected `interp: no fn main`
  under rs-meta (parse/typeck/eval-init pass). Interp source surface is inside the current subset at init level.
- 2026-06-30 A3/A5: `Option::as_ref()` returns `Option<&T>` for self-host break scanners (corpus 208, negative 169).
- 2026-06-30 A3/A5: `Option::unwrap_or_else()` zero-arg fallback closure for alias resolution paths (corpus 209, negative 170).
- 2026-06-30 typeck: variable assignment refines empty `Option`/`Vec`/`HashMap` placeholders, unblocking `None -> Some(Vec<T>)` self-host paths (corpus 210, negative 171).
- 2026-06-30 A3: `HashMap::get_mut` mutable reference surface for typeck scope refinement paths (corpus 211, negative 172).
- 2026-06-30 A11: core bundle `ast+lexer+parser+typeck+interp` now reaches expected
  `interp: no fn main` under rs-meta (parse/typeck/eval-init pass). Flatten helper
  conflicts were removed by renaming interp helpers and avoiding destructured `&mut`
  tuple assignment in refinement helpers.
- 2026-06-30 A10: hex integer literals + `u64` integer surface for `native.rs`
  FNV constants (corpus 212, negative 173).
- 2026-06-30 A10: integer bitxor `^`/`^=` for `native.rs` FNV hash update
  (corpus 213, negative 174).
- 2026-06-30 A9: `cfg!(name)` fixed macro for `native.rs` platform suffix branch
  (corpus 214, negative 175).
- 2026-06-30 A10: `char::is_ascii_hexdigit` for self-host lexer hex literal scan
  (corpus 215, negative 175).
- 2026-06-30 A10: `u64::from_str_radix` for self-host lexer hex literal parse
  (corpus 216, negative 176).
- 2026-06-30 A11/std: `std::fs::{create_dir_all,write,read_to_string}` Result
  surface for native/check source typeck (corpus 217, negative 177).
- 2026-06-30 A11/std: `PathBuf::from` + `Path/PathBuf::join` for native source
  path assembly (corpus 218, negative 178).
- 2026-06-30 A9/std: fixed `{:016x}` format placeholder for native deterministic
  artifact file names (corpus 219, negative 178).
- 2026-06-30 A11/std: `Command::new().arg().output()`, `Output.status/stdout/stderr`,
  `ExitStatus::success`, and `String::from_utf8_lossy` surface for native tier
  source (corpus 220, negative 179).
- 2026-06-30 A3/A10: `String/&str::bytes()` as `Iter<u8>` for self-host FNV
  hashing loop (corpus 221, negative 180).
- 2026-06-30 A3: `String/&str::trim()` for check output comparison paths
  (corpus 222, negative 181).
- 2026-06-30 A11/std: `&PathBuf` -> `&Path` compatibility for native_run workdir
  calls (corpus 223, negative 181).
- 2026-06-30 A3/A11: `for x in &[T]` foreach surface, yielding `&T` items
  for slice-backed status tables and source probes (corpus 224, negative 181).
- 2026-06-30 A9/A11: format/println placeholder `"{:<N}"` left-align width
  support for status table printing (corpus 225, negative 182).
- 2026-06-30 A11/std: `std::env::args()` as deterministic `Iter<String>` in
  interp, enough for CLI main argument dispatch paths (corpus 226, negative 183).
- 2026-06-30 A11/std: `std::process::ExitCode::{SUCCESS,FAILURE}` unit-like
  constants for CLI `main() -> ExitCode` paths (corpus 227, negative 184).
- 2026-06-30 A9/A11: `"{:#?}"` pretty-debug placeholder accepted, currently
  rendered through the debug path, for `cmd_ast` source surface (corpus 228,
  negative 185).
- 2026-06-30 A3/A11: `Vec::first` / slice `.first()` as `Option<&T>` for
  `load_source` argument parsing paths (corpus 229, negative 186).
- 2026-06-30 A11: `src/*.rs` all-source concat bundle under rs-meta now
  typechecks and executes deterministic CLI help path with `rc=0`.
- 2026-06-30 A5: `Iter::count/sum/fold/take/skip` surface with TV and
  acceptance TV coverage (corpus 233, negative 191).
- 2026-06-30 A3: `String/&str::split(&str)` as `Iter<&str>` with collect/join
  TV coverage (corpus 234, negative 192).
- 2026-06-30 A6: `@` binding plus integer/char inclusive range patterns
  (`1..=5`, `'a'..='z'`) with mismatch negatives (corpus 236, negative 195).
- 2026-06-30 A9: `assert!` / `assert_eq!` fixed macros with bool/equality
  typeck and panic-on-fail interp behavior (corpus 238, negative 198).
- 2026-06-30 A9: `write!` / `writeln!` fixed macros for `String` and
  `&mut String` targets, returning `Result<(), ()>` in the interpreter model
  (corpus 240, negative 202).
- 2026-06-30 A6: `while let pat = expr { ... }` loop pattern binding with
  body unit/type mismatch negatives (corpus 241, negative 204).
- 2026-06-30 A6/A10: `let pat = expr else { diverge };` plus doc comments
  (`///`, `//!`) ignored through the line-comment lexer path (corpus 244,
  negative 207).
- 2026-06-30 A10: top-level immutable `const` / `static` globals readable
  from functions, with type mismatch / assignment negatives (corpus 246,
  negative 210).
- 2026-06-30 A9/A10: `{:>N}` right-align format placeholder for
  print/println/format/write paths (corpus 247, negative 211).
- 2026-06-30 A6: pattern binding modifiers `ref x` / `ref mut x` with
  deref-observable TV coverage (corpus 249, negative 213).
- 2026-06-30 A3/typeck: shallow generic monomorphic unification for
  `fn id<T>`, `struct Wrap<T>`, and custom `enum Opt<T>` call/literal/pattern
  boundaries, with mismatch acceptance TV (corpus 252, negative 216).
- 2026-06-30 A3: `HashMap::entry` surface with `or_insert`,
  `or_insert_with`, and `and_modify`, returning mutable value refs and matching
  rustc acceptance on mutability/key/value/closure errors (corpus 256,
  negative 221).
- 2026-06-30 A5: range expressions `a..b` / `a..=b` produce `Iter<i64>` so
  `(1..5).map(...).sum()` works while slice parsing remains intact (corpus 258,
  negative 223).
- 2026-06-30 A6/typeck: light match exhaustiveness for bool/custom
  enum/Option/Result; guarded arms do not count, matching rustc acceptance
  (corpus 261, negative 226).
- 2026-06-30 A7/typeck: narrow `?` error conversion for
  `Result<T, &'static str>` into `Result<T, String>` (`From<&str> for String`
  surface); incompatible error types still rejected (corpus 262, negative 227).
- 2026-06-30 A10: repeat array/vector surface `[x; n]` and `vec![x; n]`
  backed by cloned Vec values, with count type acceptance TV (corpus 264,
  negative 228).
- 2026-06-30 A4: trait item surface + `impl Trait for Type` parsed as
  value-type method surface; missing target rejected with acceptance TV
  (corpus 265, negative 229). Full trait solving/coherence remains held.
- 2026-06-30 A9/A10: named/positional fixed macro format arguments
  normalized in parser (`{0}`, `{name}`, `name = expr`), with unused/missing
  selector acceptance TV (corpus 267, negative 231).
- 2026-06-30 A4/A9: fixed macro format args now type-check Display vs
  Debug vs LowerHex surfaces (`{}`/align, `{:?}`, `{:016x}`), including
  Box/Rc Display delegation for Display inner values and rejection of
  non-Display Vec/Rc<Vec>, non-integer hex, and non-Debug closure cases
  (corpus 269, negative 235).
- 2026-06-30 A8: known fully-qualified std path canonicalization for
  `std::path::PathBuf`, `std::collections::HashMap`, `std::process::ExitCode`,
  `std::rc::Rc`, `std::fs`, and `std::env`, with arity/unknown-const acceptance
  TV (corpus 272, negative 238). General module resolver remains held.
- 2026-06-30 A11: CLI `-f` now accepts multiple files and concatenates them in
  order; `source-bundle-check` added to `bootstrap check`, covering the
  all-source eval-init/help path in-process (source-ast 8, source-bundle 1).
- 2026-06-30 A11: `source-bundle-check` upgraded to all-source interp==rustc
  over a flattened bundle. The original CLI `main` is renamed to
  `bootstrap_main` inside the bundle and a deterministic `print_help` harness is
  used for both paths.
- 2026-06-30 A11/stage2-probe: lexer/parser/typeck source-slice harnesses run
  under rs-meta and rustc with identical stdout. This required real `&mut self`
  field mutation propagation plus recursive ref deref for user method receivers
  (corpus 273, stage2-probe 3).
- 2026-06-30 A11/stage2-probe: `interp.rs` source-slice harness now runs
  `Interp::run_main()` under rs-meta and rustc with identical stdout. Ref
  Display now delegates to the inner value, matching Rust `Display for &T`
  and unblocking self-hosted `Val::display` primitive paths
  (corpus 275, stage2-probe 4).
- 2026-06-30 A11/stage2-chain: all-source bundled evaluator' first ran a
  mini-corpus (arith, recursion, struct-field, enum-match, Vec/String, user
  `&mut self` field mutation) through
  `interp_run` under rs-meta and rustc with identical stdout. Internal runtime
  String/str `.iter()` support remains hidden behind typeck rejection for user
  Rust, but unblocks self-host char-slice representation paths. `assign_field`
  now rebuilds fields by index instead of relying on `iter_mut().find`, unblocking
  self-hosted field mutation through refs.
- 2026-07-01 A11/stage2-chain: all-source bundled evaluator' now replays the
  full positive corpus under rs-meta and rustc with identical stdout.
  Fixed blockers: bootstrap host stack size for nested evaluator recursion,
  `VecElemRef` for original Vec element mutation/refs, recursive ref-aware
  equality, unit pattern `()`, `Rc` debug display, and runtime-only hidden
  String `into_iter` for self-host helper paths.
- 2026-07-01 A11/std-closure: expanded local TV/acceptance coverage for
  `fs::write/read/read_to_string`, `Path::new`/`PathBuf::display/exists`,
  `Command::env/env_clear/output` with empty stdout/stderr, and `env::var`;
  stage2-chain now uses `corpus().len()` instead of a hardcoded corpus count
  (corpus 281, negative 249).
- 2026-07-01 A5/iter: added `Iter::nth` and `Iter::last` runtime/typeck
  surface with TV and acceptance coverage; narrow iterator surface is now
  DONE while full Iterator trait solving remains held (corpus 283,
  negative 253).
- 2026-07-01 A4/clone: added direct TV coverage for deep Vec, struct-with-Vec,
  and iterator-state clones; narrow built-in `.clone()` surface is now DONE
  while full Clone trait solving remains held (corpus 286, negative 253).
- 2026-07-01 A7/question: added direct TV coverage for Option early-None and
  Result<String> direct error propagation; narrow `?` surface is now DONE
  while full `From<E>` solving remains held (corpus 288, negative 253).
- 2026-07-01 A6/match-ergonomics: added direct TV coverage for Option, tuple,
  and struct destructuring through one reference layer; narrow match ergonomics
  surface is now DONE (corpus 291, negative 253).
- 2026-07-01 A10/int-inference: added direct TV coverage for in-range
  unsuffixed integer literals in function argument, return, and Vec expected
  contexts; full integer range/suffix typing remains held (corpus 295,
  negative 253).
- 2026-07-01 B/stage3-slim-chain: added a slim evaluator chain gate. stage1
  runs a slim evaluator stage2, which embeds and evaluates a slim evaluator
  stage2' harness; rs-meta and rustc both print `42`. Full all-source
  stage2→stage2' remains held on cost (420s timeout in local probe).
- 2026-07-01 B/stage3-all-source-smoke: smoke gate was slimmed to the
  evaluator-core source bundle (ast/lexer/parser/typeck/interp + `interp_run`),
  excluding proof corpus/check/main from the nested stage3 source. The nested
  evaluator can load and evaluate a `42` harness under rs-meta and rustc; full
  all-source corpus replay/B==C remains held on cost.
- 2026-07-01 B/stage3-core-mini: extended evaluator-core stage2' mini-corpus replay
  to cover arith, factorial, enum match, struct field, Vec<String>::join, and
  explicit iterator turbofish collection (`collect::<String>()`,
  `collect::<Vec<char>>()`) under both rs-meta and rustc. This is still below
  full corpus/B==C and does not promote the full all-source HELD boundary.
- 2026-07-02 B/stage3-core-prefix: added evaluator-core stage2' replay of the
  first 8 positive corpus cases generated from `corpus()`. A 32-case local probe
  overflowed the nested evaluator stack at the previous 64MiB host stack budget;
  after raising the host stack to 128MiB, a 16-case local probe ran past the
  240s proof budget. So 8 is the current checked bounded prefix. This is a wider
  automatic prefix proof than the hand-picked mini corpus, but full all-source
  corpus/B==C remains HELD on cost.
- 2026-07-02 B/stage3-core-middle: added evaluator-core stage2' replay of the
  middle 8 positive corpus cases generated from `corpus()`. This complements
  prefix/suffix replay with a bounded moving middle shard while keeping full
  all-source corpus/B==C explicitly HELD on cost.
- 2026-07-02 B/stage3-core-suffix: added evaluator-core stage2' replay of the
  last 8 positive corpus cases generated from `corpus()`. This complements the
  prefix replay and keeps the current corpus tail under stage3 proof as new
  corpus entries are appended.
- 2026-07-02 B/stage3-core-feature: added evaluator-core stage2' replay of 10
  named later-feature corpus cases (radix parsing, Rc/RefCell aliasing, trait
  method dispatch, write macro, struct-like enum rest patterns, slice-array
  equality, deep clone, generic enum, let-else, array repeat). This complements
  prefix replay without promoting the full all-source HELD boundary.
- 2026-07-02 B/stage3-core-negative: added evaluator-core stage2' rejection of
  10 named negative corpus cases covering type mismatch, borrow/mutability,
  Vec element typing, Result adapter shape, radix arg typing, write macro
  mutability, pattern mismatches, let-else divergence, and generic mismatch.
  This extends stage3 evidence to acceptance without promoting full B==C.
- 2026-07-02 B/stage3-core-negative-middle: added evaluator-core stage2'
  rejection of the middle 8 negative corpus cases generated from
  `negative_corpus()`. This complements named/suffix rejection with a bounded
  moving middle shard while full B==C remains held on cost.
- 2026-07-02 B/stage3-core-negative-suffix: added evaluator-core stage2'
  rejection of the last 8 negative corpus cases generated from
  `negative_corpus()`. This keeps the moving negative corpus tail under stage3
  acceptance proof without promoting full B==C.
- 2026-07-01 regression: added `chars().collect()` into `Vec<char>` function
  parameter inference corpus, covering the String-backed char collection
  coercion used by self-host evaluator paths (corpus 295, negative 253).
- 2026-07-01 regression: added explicit iterator turbofish collection coverage:
  `chars().collect::<String>()`, `chars().collect::<Vec<char>>()` through `Rc`,
  and negative `Vec<i64> -> collect::<String>()` acceptance TV
  (corpus 297, negative 254).
- 2026-07-01 B/stage3-full-held: moved full all-source stage2→stage2' from
  open TODO to explicit HELD manifest row. This avoids overclaiming while keeping
  the local cost boundary machine-readable.
- 2026-07-01 B/stage3-full-held-check: added a local proof command that fails if
  the full all-source stage3 row drifts away from explicit HELD/cost-boundary status.
- 2026-07-01 C/stage8-repro-seed: native tier now exposes deterministic artifact
  receipts and builds with `SOURCE_DATE_EPOCH=0`, `-C debuginfo=0`,
  `-C metadata=rsmeta`, `-C codegen-units=1`, and `--remap-path-prefix`.
  The local `stage8-repro-check` compiles both a sample Rust source and the
  all-source evaluator bundle in two workdirs and requires identical canonical
  receipts(source hash, rustc version, flags, artifact FNV).
- 2026-07-01 C/stage8-selfhost-repro: added `stage8-selfhost-repro-check`;
  the stage2 evaluator' source bundle is built in two fresh workdirs and must
  produce identical canonical native artifact receipts.
- 2026-07-01 manifest: added `proofs/stage-manifest.tsv` and `manifest-check`.
  The manifest records stage status, local check command, timeout, and cost note;
  GitHub Actions remains disabled and local checks are the source of truth.
- 2026-07-01 bootstrap-map: added `proofs/rustc-bootstrap-map.md` to pin the
  rustc stage0/1/2/3/stage8 vocabulary mapping and held boundaries.
- 2026-07-01 isolation: added `isolation-check` to verify fresh interpreter runs
  do not leak stdout or function namespace state across programs.
- 2026-07-01 constitution: added `constitution-check` for zero crates.io deps,
  local-only Actions-disabled posture, and content-hash native artifact naming.
- 2026-07-01 native-cache: `native_run` now uses a content-hash compile cache
  keyed by deterministic flags + source. Stage8 artifact receipts still force
  fresh rustc compiles. Added `native-cache-check`.
- 2026-07-01 stage9-replay-seed: expanded clean-process replay to a lightweight
  product entrypoint matrix (`help`, `stage-status`, `run`, `native-run`, `ast`,
  `manifest-check`) with hard-fixed `SOURCE_DATE_EPOCH`, soft-observed `PATH`
  for rustc lookup, and canonical JSON receipts. Recursive aggregate replay was
  later closed by bounded aggregate replay.
- 2026-07-01 stage9-proof-matrix: added clean-process replay for all
  non-recursive proof commands with canonical JSON receipts.
- 2026-07-01 stage9-aggregate-replay: narrowed bounded clean-process replay from
  full aggregate `check` to the non-recursive proof-command matrix. This keeps
  local verification bounded while preserving clean-process proof replay.
- 2026-07-01 stage10-session-seed: added deterministic clean-process session
  replay; the same command transcript is replayed twice and canonical JSON
  receipts must match. Client/server/sandbox closure was later closed by
  `stage10-sandbox-check`.
- 2026-07-01 stage10-sandbox: added `proofs/session-sandbox.tsv` and
  `stage10-sandbox-check` for client/server/session/sandbox replay boundaries,
  local-only sandbox env, disabled Actions, HELD external sandbox rows, and
  fail-closed conflict policy.
- 2026-07-01 stage11-adapter-seed: added `proofs/adapter-schema.tsv` and
  `stage11-adapter-check` for local/DISABLED/HELD adapter rows plus explicit
  held policy and fail-closed conflict policy. Multi-domain adapter closure was
  later closed by `stage11-adapter-replay-check`.
- 2026-07-01 stage11-adapter-replay: added `proofs/adapter-replay.tsv` and
  `stage11-adapter-replay-check`; DONE adapters replay through local subprocess
  receipts, Actions stays DISABLED, and external adapters remain HELD/fail-closed.
- 2026-07-01 stage12-quarantine-seed: added `proofs/quarantine-policy.tsv`
  and `stage12-quarantine-check` for local verification, no auto-promotion,
  Actions disabled, fail-closed policy, and held rows.
- 2026-07-01 stage12-quarantine-replay: added `proofs/quarantine-replay.tsv`
  and `stage12-quarantine-replay-check`; local gates replay in subprocesses,
  candidate intake stays no-auto-promotion, Actions is DISABLED, and manual/
  self/external promotion rows remain HELD/fail-closed.
- 2026-07-01 stage13-horizon-seed: added `proofs/horizon-policy.tsv` and
  `stage13-horizon-check` for stale evidence degradation, no-boundary-leak
  policy, and manifest/replay receipt anchoring.
- 2026-07-01 stage13-horizon-replay: added `proofs/horizon-replay.tsv` and
  `stage13-horizon-replay-check`; manifest/session receipts replay locally,
  while stale/external/ambient signals remain HELD with no-boundary-leak and
  degrade-to-held policy.
- 2026-07-01 stage14-cross-impl-seed: added `proofs/cross-impl-schema.tsv`
  and `stage14-cross-impl-check` for local/native export schema rows,
  disabled Actions, held alternate implementation/toolchain rows, and fail-closed
  conflict policy.
- 2026-07-01 stage14-cross-impl-replay: added `proofs/cross-impl-replay.tsv`
  and `stage14-cross-impl-replay-check`; local rs-meta and rustc-native exports
  replay through checked subprocess receipts, while alternate toolchains,
  external evaluators, and DDC stay HELD/fail-closed.
- 2026-07-01 stage15-evidence-seed: added `proofs/evidence-federation.tsv`
  and `stage15-evidence-check` for local proof, stage manifest, disabled Actions,
  external evidence offline approval, and fail-closed conflict policy.
- 2026-07-01 stage15-evidence-replay: added `proofs/evidence-replay.tsv` and
  `stage15-evidence-replay-check`; local proof and manifest evidence replay
  through checked subprocess receipts, while external web/tool/human evidence
  remains HELD with offline/review approval and fail-closed policy.
- 2026-07-01 stageN-extension-seed: added `proofs/extension-policy.tsv` and
  `stageN-extension-check` for versioned manifest policy, timeout/cost budget,
  explicit migration, held rows, and fail-closed extension behavior.
- 2026-07-01 stageN-extension-replay: added `proofs/extension-replay.tsv`
  and `stageN-extension-replay-check`; manifest index, timeout/cost budget, and
  stageN seed replay through checked subprocess receipts while breaking/external/
  future extensions remain HELD/fail-closed.
- 2026-07-02 local-only: added explicit `actions-disabled-check` proof command
  and manifest row. `.github/workflows` must be absent and the disabled workflow
  receipt must stay in `.github/workflows.disabled/rs-meta.yml`; verification is
  local `cargo build` + `bootstrap check`, not GitHub Actions.
- 2026-07-02 CI: GitHub Actions 게이트 재검증. workflow는
  `.github/workflows.disabled/rs-meta.yml`에 보관되고, 검증은 로컬 `cargo build` +
  `bootstrap check`로 수행.

## 6. 정직 경계 (held)

- **Rust 정확성 증명 아님.** interp는 *신뢰된 오라클*, rustc도 신뢰 기판. TV는 "두
  경로 등가" 증거지 컴파일러 정확성 형식증명 아님.
- **Trusting-Trust 방어 아님.** (추후) B==C 고정점은 reproducibility 증거일 뿐.
  Wheeler DDC(다른/신뢰 컴파일러 재컴파일)는 별도 트랙, held.
- subset는 작다. 밖은 `rustc rejected`/parse-error로 정직 거부.
- self-host(stage2)는 현재 로컬 proof 기준 DONE: all-source evaluator'가 positive
  corpus 전체를 replay하고 rustc와 동일 stdout을 낸다.

## 7. 참고

- evcxr-0.21.1 = native tier 레퍼런스(스니펫 wrap→rustc→dylib/run). 여기선
  subprocess로 단순화(zero-dep).
- rustc-dev-guide bootstrapping (stage0/1/2). D. Wheeler "Diverse Double-Compiling"
  (Trusting-Trust 형식증명, 별도 트랙).
