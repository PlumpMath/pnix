# rs-meta status (peer host-meta floor)

Last verified: 2026-08-17.

## Peer-floor statement

**rs-meta** is a standalone Rust-in-Rust meta-circular compiler/evaluator
(zero crates.io deps; `rustc` is an external toolchain only). Practical peer
floor:

| Peer | Peer floor | rs-meta counterpart |
|---|---|---|
| hy-meta | stage ladder through ~15 | stage-manifest DONE through stageN seeds |
| cljs-meta | fixed-point (B==C) | stage3-fixedpoint-check (evaluator B==C) |
| clj-meta | selfhost + stock rebuild | source-bundle + stage2/3 chain + emit self-host |
| clr-meta | eval gens + C0–C3 | interp + native TV + Stage1/2-style chain |

**Core honesty:** translation validation (`interp` stdout == `rustc` stdout) is
the permanent green bar. Stage15/N rows in the manifest are **local seed/replay
closures**, not full rustc replacement or Trusting-Trust defense.
Borrow checker / full trait solver / user `macro_rules!` remain held; core
self-host is **not** blocked by those held features (see `docs/self-hosting.md`).

## Closed claims

Live-verified this session (2026-08-07):

```text
cargo build                           OK (~13s debug)
bootstrap self-check                  PASS 407/407
bootstrap tv-check                    PASS 407/407 (interp == rustc)
```

Documented DONE in `proofs/stage-manifest.tsv` (not all re-run this session):

```text
typeck-check, source-ast/bundle, emit-tv/self-host
stage2-chain/probe, stage3 shards + fixedpoint + full-chain (budget)
stage8..stageN seed + local replay closures
fuzz/emi differential discipline, selfhost-audit, constitution
```

## Open claims (do not claim)

```text
full_rustc_replacement = false
trusting-trust_defense = false
borrow_checker = held
full_trait_solver / dyn Trait / where / blanket = held
user_macro_rules_and_proc_macros = held
whole_binary_self_interpretation_default_gate = false
  (full-chain is budget-gated, not default primary)
```

**Correction (verified this session, 2026-08-11):** the "source-ast/bundle" row
above claimed DONE in `proofs/stage-manifest.tsv` is stale for
`source-bundle-check` specifically. `stage9-aggregate-replay-check` (and the
full `check` aggregate) fail on a completely unmodified checkout — confirmed
via a `git worktree` at the pre-session commit, not a regression from any
recent change. Root-caused two layered pre-existing bugs, one fixed:

```text
FIXED: parser.rs's format!-arg-usage validator didn't understand `{:.*}`
  (dynamic precision from an extra positional arg) -- it counted `{:.*}` as
  consuming one positional arg (the value) when real Rust's format! macro
  consumes two (precision, then value). Caused src/interp.rs's own
  `format!("{:.*}", precision, f)` (line ~2463, real, valid, rustc-accepted
  Rust) to fail rs-meta's OWN self-parse with "positional arg 1 is never
  used". Fixed.

FIXED: parser.rs's struct-pattern-field parser didn't recognize the `ref`
  shorthand (`Struct { ref x, .. }`) -- it read `ref` itself as if it were
  the field name. src/io.rs:97 uses exactly this
  (`Err(MetaIoError { ref error_class, .. }) if ...`), a real, valid Rust
  pattern. Fixed by special-casing `ref [mut] ident` in
  `parse_pattern_fields`, reusing the existing `Pattern::BindRef` AST node
  (already used for top-level `ref` patterns). io.rs now parses cleanly:
  source-ast-check is 16/16 (io.rs added to `source_files()` in check.rs;
  was 14/14 without it, would have been 15/15-with-a-failure without the
  ref fix).

ADDED (additive, no risk to existing passes): fs::symlink_metadata,
  fs::metadata, fs::read_dir type signatures, and Metadata/FileType method
  modeling (`.file_type()`, `.is_symlink()`/`.is_dir()`/`.is_file()`) in
  typeck.rs -- io.rs's `classify()` needs these to type-check.

FIXED: `Path::new(s)` was typed as returning owned `PathBuf` in typeck.rs;
  real Rust's `Path::new` returns borrowed `&Path`. io.rs's
  `classify(Path::new(path))` (classify takes `&Path`) surfaced this as a
  type mismatch. Changed the rule to return
  `Type::Ref { mutable: false, inner: Named("Path") }`. Verified safe: this
  is method-call-transparent (`method_target_name` already derefs `Ref` to
  find the target type, so `.exists()`/`.display()`/`.join()` etc. still
  resolve correctly), and re-confirmed self-check 407/407, tv-check 407/407,
  typeck-check 272/272 -- no regressions.

FIXED: `e.kind() == std::io::ErrorKind::X` (io.rs's `read_utf8`/`read_dir`
  error handling) didn't type-check: `fs::read`/`fs::read_dir`'s errors are
  modeled as plain `String` (a PRE-EXISTING simplification, left untouched
  since other passing code likely relies on it), which has no `.kind()`
  method, and separately `std::io::ErrorKind::NotFound`/`NotADirectory`
  parse as a single flat `Expr::Var("std::io::ErrorKind::NotFound")` (a bare
  multi-segment path with no call doesn't build a structured path node) so
  it hit "unbound variable". Fixed additively: `.kind()` is now a special
  case on `type_string_method` returning a new `IoErrorKind` type, and the
  two exact literal variable-name strings are recognized in `type_expr`'s
  `Expr::Var` handling and typed as `IoErrorKind` too, so the `==` compares
  matching types without changing what `fs::read`'s error actually is.

FIXED: `String::from_utf8(bytes)` (io.rs's `read_utf8`) wasn't registered at
  all (only `String::new`/`String::from`/`String::from_utf8_lossy` were).
  Added `Result<String, String>` (io.rs immediately discards the error via
  `.map_err(|_| ...)`, so the exact error type doesn't matter here, but
  `String` matches this codebase's existing fs-error convention).

FIXED: `read_dir`'s `for item in entries { ... }` -- ForEach's iterator-type
  match only knew Vec/&Vec/&slice/Iter<T>, not `ReadDir`. Added it (item type
  `Result<DirEntry, String>`, matching this codebase's fs-error convention),
  plus narrowly-scoped `DirEntry` (`.file_name()` -> `OsString`, `.path()` ->
  `PathBuf`), `OsString` (`.to_string_lossy()` -> `Cow`), and `Cow`
  (`.into_owned()` -> `String`) method dispatchers -- exactly the
  `file_name().to_string_lossy().into_owned()` chain io.rs uses and nothing
  more (no general std::ffi/std::borrow coverage), matching this codebase's
  existing narrow-modeling style.

FIXED: `io::FILE_READ_CAPABILITY`/`io::io_check()` in main.rs (and, it turns
  out, `cap::CAP_FS_READ` too -- confirmed by isolating cap.rs alone in a
  throwaway bundle, both fail identically) hit "unknown enum {module}".
  Root cause, found by reading the parser: any bare 2-segment path with no
  call (`module::CONST`) parses as `Expr::EnumCtor`, and typeck only
  resolved that via an enum-variant table or a `self.globals` lookup keyed
  by the literal string `"module::CONST"` -- but top-level `const`
  declarations are parsed with their bare name only (`parse_global` never
  learns a module prefix; this bundle flattens every file's un-`mod`-qualified
  top level into one program), so that qualified key never existed for ANY
  module-qualified constant, not just io's. Fixed generally: when
  `enum_name` isn't a real registered enum, fall back to a bare-name
  `self.globals` lookup before erroring -- fixes every `module::CONST`
  reference symmetrically, not just io's.

FIXED (bundling-level bug, in check.rs's `normalize_bundle_line`, not
  typeck.rs): every other bundled file already had its own self-reference
  prefix stripped (`lexer::`, `parser::`, ... `witness::`) but `io::` was
  simply missing from that list -- so real rustc (which source-bundle-check
  also compiles the bundle through) saw literal unresolvable `io::` paths
  and, confusingly, suggested "did you mean `i8`" (textually close builtin
  type), which looked like a bundling corruption bug before the real cause
  was found. Added `io::` stripping, but naively stripping it as a blind
  substring also ate the "io" out of real `std::io::` paths -- protected
  those first via a placeholder round-trip.

FIXED (separate bundling-level bug, same function): `io.rs`'s
  `use std::path::Path;` and `native.rs`'s `use std::path::{Path, PathBuf};`
  are textually different lines importing an overlapping name, so the
  bundler's exact-text-match dedup kept both, and rustc rejected `Path` as
  redefined. Replaced the dedup with one based on the actual imported
  name(s) (parsing both `use ...::Name;` and `use ...::{A, B};` forms) so a
  name already brought in by an earlier line's broader form is recognized
  and the redundant later line is dropped.

**`source-bundle-check` now PASSES** -- the check this whole investigation
  was originally trying to get past. Session total across both rounds: 9
  layers found in io.rs/main.rs/the bundler (io.rs had never been in the
  self-hosting bundle before this session), all 9 fixed. Verified no
  regressions: self-check 407/407, tv-check 407/407, typeck-check 272/272,
  independent-mini-backend-check 9/9, source-ast-check 16/16.

FIXED (10th layer, `interp.rs` runtime dispatch bug -- genuinely different
  category from the 9 above, and now closed): `stage2-chain-check`
  ("stage2 evaluator' corpus replay") failed with `interp: unsupported Vec
  method chars`. This check is meta-circular in a way nothing above it is:
  its harness calls `interp_run` *from inside interpreted code*, i.e. it
  runs the whole bundle (including `interp.rs`'s own lexer/parser/
  interpreter logic) *through rs-meta's own interpreter* rather than through
  real rustc or through typeck alone.

  Root cause: `interp.rs`'s untyped `.collect()` (no turbofish) has no
  static type information, so it *guesses* String vs `Vec<char>` from the
  runtime items it collected -- and, by explicit design (see the comment at
  its definition), an *empty* char iterator guesses `Vec` rather than
  `String`. `coerce_let_value` already patched the opposite mismatch
  (`let x: Vec<char> = ...` landing as a String) via
  `coerce_string_to_char_vec`, but had no symmetric case for a `let
  x: String = ...` landing as an empty/non-char `Vec`. That gap is
  extremely common to hit: `parser.rs`'s `normalize_format_args` does
  `let inner: String = chars[i + 1..j].iter().collect();` to extract the
  text *between* a format placeholder's braces -- which is the empty string
  for every bare `{}` placeholder, i.e. almost every `println!`/`format!`
  call in the whole self-interpreted program. Confirmed by isolated repro
  (`bootstrap run -c`) before touching any real file. Fixed additively and
  symmetrically: added `coerce_char_vec_to_string` (mirrors
  `coerce_string_to_char_vec`) and a matching `(Type::Named("String"),
  Val::Vec)` arm in `coerce_let_value`.

  This unblocked `stage2-chain-check` far enough to expose a second,
  previously-unreachable gap in the same area: `interp.rs`'s own
  `format_println` (the RUNTIME renderer used when the interpreter executes
  a real `println!`/`format!` call) had no case for `{:.*}` (dynamic
  precision) -- only `normalize_format_args` (parser.rs, parse-time arg
  counting) and `format_placeholder_kinds` (typeck.rs, type checking) had
  been taught about it earlier this session; the runtime renderer was
  never updated to match, and this was masked until the Vec/chars fix let
  execution reach far enough to hit it. This mattered because `interp.rs`'s
  *own source* calls `format!("{:.*}", precision, f)` in its
  fixed-precision formatting branch, and self-interpreting `interp.rs` runs
  that exact line as interpreted code, which needs its own `format_println`
  to render it. Fixed additively: added a `{:.*}` branch to `format_println`
  that consumes two args (precision, then value) exactly like the
  parse/typeck-time fix already does.

  Verified: `stage2-chain-check` now PASSES, and so does
  `stage9-aggregate-replay-check` -- the check this whole investigation was
  originally trying to get past (`aggregate-proof-matrix -> success`).
  No regressions: self-check 407/407, tv-check 407/407, typeck-check
  272/272, independent-mini-backend-check 9/9, source-ast-check 16/16,
  source-bundle-check PASS, ast-diff-check 4/4, rust-ir-check 4/4,
  emit-tv-check 407/407.
```

**Three more small/medium items closed, same wave (2026-08-12/13):**

```text
FIXED: A4 generic-inference tail -- Rc::ptr_eq(&Rc::new(vec![1]),
  &Rc::new(vec![1u64])) now infers u64 for both sides like rustc, while
  explicit Rc<i64> vs Rc<u64> still gets rejected. Root cause: vec!'s own
  typeck eagerly collapses an unsuffixed literal to i64 before Rc::ptr_eq
  ever compares its two arguments, so the flexibility needed to unify is
  already gone by then. A blanket fix (stop collapsing in vec!'s typeck
  entirely) was tried and reverted -- it broke a real positive fixture
  (`vec![3, 1, 2]; v.sort_by(|a, b| b.cmp(a))`) that legitimately needs the
  literal to default to i64 in isolation. Fixed narrowly instead: a
  fallback inside Rc::ptr_eq's own check that only runs on the failure
  path, re-deriving each side's vec! element type without the eager
  default for exactly the `&Rc::new(vec![...])` shape. Self-hosting
  gotcha hit while building this: `.collect::<Result<Vec<_>, _>>()` isn't
  modeled by interp.rs's own Iter::collect turbofish handling -- rewritten
  as two plain sequential calls.

FIXED: independent-mini-backend-check widened 9->13 fixtures (nested if,
  double-recursive fibonacci, 3-arg, 4-arg) using capabilities the backend
  already had -- no new backend features needed for this pass.

FIXED: trait-boundary-check's held-assoc-type classification was false --
  associated types genuinely parse, typecheck, AND execute correctly now
  for a supported (struct) impl target, confirmed by actually running the
  code, not just checking it parses. The original fixture
  (`impl Two for i64`) had conflated two separate boundaries: associated
  types (now supported) and implementing any trait for a primitive type
  (still separately unsupported -- confirmed the same error occurs for a
  trait with NO associated type against the same i64 target). Also
  confirmed the "support" is shallow: Self::Out in the trait signature is
  never checked against the impl's own `type Out = ...` binding, so a
  mismatched impl is silently accepted -- documented as an explicit,
  honest gap via a new classification (`assoc-type-accepted-unenforced`)
  rather than either overclaiming full support or leaving the false
  held-assoc-type label in place. A second caller
  (`rust_surface_check`, consumed by pnix-rs's peer-engine `verdict`
  field) had its own separate hardcoded assertion of the old label --
  missed on the first pass, caught by re-running the full
  `bin/rs-meta-gate check` aggregate before considering this closed.

Verified (all three together): self-check 408/408, tv-check 408/408,
  typeck-check 273/273, independent-mini-backend-check 13/13,
  source-ast-check 16/16, source-bundle-check PASS, macro-boundary-check
  3/3, borrow-boundary-check 3/3, trait-boundary-check 4/4,
  rust-surface-check 4/4, full `bin/rs-meta-gate check` aggregate PASS --
  no regressions.
```

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

**`mrustc` turned out not to be usable here.** It's packaged in nixpkgs, but
marked `platforms = [ "x86_64-linux" ]` only — this dev machine is
`x86_64-darwin`, and forcing an unsupported-platform build of a *trust*
witness through `NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM` would be actively
counterproductive (a shakily cross-built mrustc could silently miscompile
and give false DDC confidence, which is worse than not having the check).
The mrustc phased plan below remains a valid option on Linux; the concrete
progress this session instead follows the same in-house pattern the other
four hosts already used.

**Independent mini backend added this session (2026-08-11):**
`independent_mini_backend.rs` is a new, from-scratch tokenizer/parser/
tree-walking interpreter for a small `i64` Rust subset (`fn`, `if`/`else`,
`+`/`-`/`*`, `<`/`>`/`<=`/`>=`/`==`, recursive calls), sharing zero code with
`lexer.rs`/`parser.rs`/`ast.rs`/`typeck.rs`/`interp.rs` — the evaluator core
`tv-check` already proves `== rustc` on. `rustc` itself remains the trusted
oracle, the same honest role real upstream Hy plays for the Python host's
`independent_mini_backend.py` and the self-hosted compiler plays for the
ClojureScript host's `independent_mini_backend.js`.

Covers 9 fixtures, cross-validated against real `rustc` (via `native::native_run`,
the same mechanism `tv-check` uses) — both agree on all 9, including a
recursive factorial. Wired into `independent-mini-backend-check` (both as its
own CLI subcommand and folded into the `check` aggregate). Verified live this
session: 9/9 accepted, `self-check` 407/407 and `tv-check` 407/407 re-run
unaffected (no regressions), full `check` aggregate green.

**What this closes and what it still doesn't:** a genuine 2-way behavioral
comparison (real `rustc` ≡ from-scratch interpreter) now exists and passes,
not just a documented plan. It is still only a bounded fixture set against a
*fresh* independent implementation — not the 407-case corpus, and (same
honest bar every host settled on this session) an *interpreter*, not a second
*compiler*, so it does not by itself clear the full Wheeler bar the way a
genuine mrustc-vs-rustc compiler comparison would. (Widened 9→13 in an
earlier pass — nested `if`, double-recursive fibonacci, 3-/4-arg fns, no new
backend features needed; see `todo.md` item 2.)

**`let`/`while`/assignment 확장, 2026-08-17:** clj-meta/clr-meta의 U6/mini-backend
witness가 이번 세션에 `let` → `loop`/`recur` → closure 순서로 크게 넓어진 것과
균형을 맞추기 위해, 사용자가 "나머지 host들도 clojure만큼" 지시 후 clr-meta를
먼저 끝내고 이제 rs-meta 차례로 명시적으로 지목. 지금까지 이 backend는
`fn`/`if`/arithmetic/comparison/재귀 호출만 있었고 지역 변수(`let`)도, 반복문도
없었음 — 매 fixture가 순수 재귀로만 루프를 표현해야 했음(예:
`mini-recursive-factorial`).

**1단계 (`let`):** `fn` 본문과 `if`/`else` 각 분기를 `MiniExpr`(단일 식) 대신
새 `MiniBlock { stmts: Vec<MiniStmt>, tail: Box<MiniExpr> }`로 재설계 — 0개
이상의 `let` 문 다음 필수 tail 식이라는, 실제 Rust 블록과 정확히 같은 모양.
`MiniStmt::Let(name, init)`이 평가 시점에 (호출마다 새로 만드는) 평면
`HashMap<String,i64>` env에 직접 저장. **의도적으로 얕은 스코프**: 진짜 Rust처럼
블록이 끝나면 그 안에서 선언된 이름이 스코프 밖으로 나가는 게 아니라, 같은 함수
호출의 평면 env에 계속 남음 — 이 차이에 의존하는 fixture는 하나도 안 씀(전부
서로 다른 이름 사용)이라 문제 없지만, 코드 주석으로 정직하게 문서화. `mut`
파라미터/바인딩 키워드도 파싱만 하고(실제 mutation은 다음 단계) 받아들이게 추가.
sequential let, shadowing, if-분기 안의 let, mut 파라미터 4개 fixture 추가
(13→17), 전부 실제 `rustc` 대비 검증.

**2단계 (`while`+대입), 같은 날:** `let`이 만든 블록/문 인프라 위에 `MiniStmt::
Assign(name, value)`(재대입)와 `MiniStmt::While(cond, Vec<MiniStmt>)`(본문은
tail 없는 순수 문장 시퀀스 — 실제 Rust의 unit 타입 `while`과 동일하게, 이
backend에선 mutation 부수효과로만 쓰이므로 자기 값이 필요 없음) 추가. 파서는
`IDENT =`(2-token lookahead, `==`와는 별개 토큰이라 충돌 없음)로 대입문을 인식.
합산 loop, factorial loop, mut 파라미터 loop, 중첩 while 4개 fixture 추가
(17→21), 전부 실제 `rustc` 대비 검증. 회귀 없음: `self-check` 408/408,
`tv-check` 408/408(두 단계 각각 재실행) — `independent_mini_backend.rs` 자체가
rs-meta 자기호스팅 corpus의 일부라 self-check가 이 파일도 rs-meta 자신의
프론트엔드로 파싱/타입체크됨을 의미 있게 검증. `independent-mini-backend-check`
9→13(이전 패스)→17(`let`)→21(`while`/대입), 매 fixture 전부 `rustc`와 직접 비교.

**3단계 (진짜 클로저), 같은 날:** clj-meta/clr-meta가 nested-fn 다음으로 거친
"진짜 first-class 클로저" 단계의 대응물. `MiniVal` enum(`Int(i64)` |
`Closure(Rc<ClosureVal>)`)을 도입해 env 값 타입을 `i64`에서 `MiniVal`로
일반화, `ClosureVal { params, body, env }`(정의 시점 env를 그대로 clone —
lexical/creation-time scoping, dynamic scoping 아님)를 추가. 새
`MiniExpr::Closure(Vec<String>, Box<MiniExpr>)`가 `[move] |params| EXPR`
리터럴(단일 식 본문만, `{ }` block 본문은 미지원 — 실제 Rust도 두 형태 다
허용하지만 어떤 fixture도 block 본문이 필요 없어서 안 함). `move` 키워드는
파싱만 하고 무시 — 이 interpreter는 항상 env 전체를 clone해서 캡처하므로,
`move`가 강제하는 by-value capture와 이미 정확히 같은 동작.

`Call(name, args)`는 **콜리가 항상 bare 이름**(임의 식 아님 — 실제 Rust는
`(|x| x)(1)` 같은 즉시호출도 허용하지만 어떤 fixture도 필요 없어서 미지원,
필요해지면 `Call`을 `Box<MiniExpr>` 콜리로 넓혀야 함)이라는 기존 모양을 그대로
재사용: eval 시점에 그 이름이 env 안에서 클로저 값에 묶여 있으면 클로저
호출(실제 Rust의 shadowing 규칙대로 — 지역 클로저가 같은 이름의 top-level
`fn`을 가림), 아니면 기존 top-level `fn` 조회로 fallback.

**clr-meta 대비 흥미로운 차이점**: clr-meta의 진짜 클로저는 .NET
`DynamicMethod`가 `TypeBuilder` 멤버를 참조 못 하는 실제 플랫폼 제약 때문에
완전히 새 TypeBuilder-hosted codegen 경로가 필요했고, 그래서 단일 파라미터·
클로저-캡처-클로저 불가 같은 인위적 경계를 명시적으로 그어야 했음. rs-meta는
단순 tree-walking interpreter라 그런 codegen 제약이 아예 없음 — 여러 파라미터
클로저(`mini-closure-two-params`)도, 클로저를 캡처하는 클로저
(`mini-closure-captures-closure`)도 어떤 인위적 제한 없이 자연스럽게 동작함을
fixture로 실제 검증(다른 host들의 "의도적으로 좁힌 경계"와 달리, 여기선 진짜
구현 제약이 없어서 좁힐 이유도 없었음). 여러 번 호출, non-tail 호출,
let-바인딩/파라미터 캡처, 2-파라미터 클로저, top-level fn을 가리는 클로저,
클로저를 캡처하는 클로저까지 6개 fixture 추가(21→27), 전부 실제 `rustc` 대비
검증. 회귀 없음: `self-check` 408/408, `tv-check` 408/408.

**다음 후보:** `!=`, 문자열/불리언 처리는 여전히 열려 있음(todo.md item 2).
클로저를 `let` 아닌 top-level `fn` 인자로 넘기는 형태(고차 함수)는 아직
미시도 — 다음 자연스러운 확장 후보.

**4단계 (`loop`+`break`), 같은 날:** clj-meta/clr-meta의 `loop`/`recur`
대응물. `while`은 항상 unit이라 값을 못 만드는 반면, 실제 Rust의 `loop { ...
break EXPR; ... }`는 **표현식**이고 `break`로만 값을 만들어냄 — `MiniExpr::
Loop(Vec<MiniStmt>)`로 추가. `break`/조건부 종료를 표현하려면 새 `MiniStmt::
Break(EXPR)`과, else 없는 `if COND { .. }`를 문(statement)으로 쓰는 새
`MiniStmt::IfStmt` 필요.

**의도적으로 분리한 파서 함수**: 이 둘(`IfStmt`/`Break`)은 기존
`parse_stmt_list`(모든 `fn` 본문/`if`-분기 block이 쓰는 것)가 아니라 완전히
새로운 `parse_loop_body`에서만 인식되게 함 — `parse_stmt_list`가 `if`를
statement-starter로도 인식하게 만들면, 기존 21개 이상 fixture가 전부 쓰는
"tail 위치의 `if`/`else`"(값을 만드는 표현식)와 충돌함: greedy하게 `if`를
문으로 먼저 소비해버리면 `else`가 남아서 파싱이 깨짐. `loop`의 본문은 애초에
tail expression이 필요 없는 순수 문장 시퀀스라서, 새 case를 거기에만 한정하면
이 충돌이 아예 안 생김 — 대신 `if`-무-`else`/`break`는 `loop` 본문 안에서만
쓸 수 있고 일반 `fn`/`while` 본문에 직접 못 씀(어떤 fixture도 필요 없음).

**break 신호 전파**: `exec_stmts`의 반환 타입을 `Result<(), String>`에서
`Result<Option<MiniVal>, String>`로 바꿔 — `None`은 "끝까지 정상 실행",
`Some(v)`는 "break v가 일어나 unwind 중"이라는 뜻. `IfStmt`/`While`은 안쪽
`exec_stmts` 호출이 `Some`을 반환하면 그대로 위로 전파(또는 `while`은 자기
반복을 멈춤 — "innermost loop" 규칙); `MiniExpr::Loop`가 실제로 `Some(v)`를
잡아서 `v` 자체를 loop 표현식의 값으로 삼고, `None`이면(break 없이 본문이
끝까지 실행됨) 실제 Rust의 `loop`처럼 무조건 다시 반복. 예외/시그널 타입을
새로 만들지 않고 `Option`을 `Result` 옆에 태우는 것으로 충분 — `break`가
유일한 non-local 제어 흐름이라(`continue`/`return` 없음) 이 정도로 충분.

부수적으로 `%`(modulo, `checked_rem`)도 추가 — nested-if fixture에서
even/odd 판정에 필요해서 겸사겸사 추가한 작은 opcode. 합산 loop, tail 위치
loop, 중첩 if-무-else+break(짝수일 때만 탈출), loop 안에 while 중첩 4개
fixture 추가(27→31), 전부 실제 `rustc` 대비 검증. 회귀 없음: `self-check`
408/408, `tv-check` 408/408.

**다음 후보:** `!=`, 문자열/불리언 처리, 클로저를 top-level fn 인자로 넘기는
고차 함수 형태는 여전히 열려 있음.

**5단계 (`!=` + 고차 함수), 같은 날:** 위에 적어둔 두 후보 중 둘을 처리.
`!=`는 기존 비교 연산자 패턴 그대로 `MiniBinOp::Ne` 추가(토크나이저 2-char
표에 `!=` 추가, `==`와 같은 자리에 파싱/평가) — 사소함.

고차 함수(클로저를 top-level `fn`의 파라미터로 넘기는 형태,
`fn apply_twice(f: impl Fn(i64) -> i64, x: i64) -> i64 { f(f(x)) }`류)는
**파서만 확장하면 됐고 인터프리터 쪽은 손댈 필요가 전혀 없었음** — 흥미로운
발견. `skip_type_annotation`이 이제 bare `i64` 외에 `impl Fn(T, ...) -> T`
모양도 인식해서 건너뛰도록만 넓혔을 뿐(다른 타입 어노테이션 형태 —
`&dyn Fn`, `Box<dyn Fn>`, generic `F: Fn(...)` bound 등 — 는 어떤 fixture도
필요 없어서 미지원, 이 backend는 애초에 타입체크를 안 하므로 타입 어노테이션은
그냥 건너뛰는 존재일 뿐). 왜 인터프리터가 안 바뀌어도 됐는지: 클로저 값이
`let`으로 묶였든 fn **파라미터**로 묶였든 결국 같은 `env: HashMap<String,
MiniVal>`에 똑같이 저장되고, 기존 `Call` dispatch 로직(`env`에서 그 이름을
먼저 찾아 클로저면 클로저 호출)이 이름의 "출신"(let-binding vs param)을
전혀 구분하지 않기 때문 — 3단계(클로저) 슬라이스에서 이미 일반적으로 설계해둔
덕을 여기서 봄.

`!=` 비교 1개, 고차 함수 3개(단순 apply-twice, 2-파라미터 클로저를 받는
버전, 캡처를 가진 클로저를 여러 번 인자로 넘기는 버전) 총 4개 fixture 추가
(31→35), 전부 실제 `rustc` 대비 검증. 회귀 없음: `self-check` 408/408,
`tv-check` 408/408.

**다음 후보:** 문자열/불리언 처리는 여전히 열려 있음. 나머지는 특별히 없음
— clj-meta/clr-meta가 이번 세션에 거친 let/loop-recur/closure 축 전체를
rs-meta도 사실상 따라잡음.

## Primary gate

```sh
# From pnix-rs/rs-meta/
./bin/rs-meta-gate                 # cargo build + self-check + tv-check
./bin/rs-meta-gate self-check
./bin/rs-meta-gate check           # full local check aggregate (long)
```

## Last run (this machine, 2026-08-17)

| Gate | Result | Notes |
|---|---|---|
| `self-check` | **PASS** 408/408 | cargo 1.97.1 / rustc 1.97.1 |
| `tv-check` | **PASS** 408/408 | |
| `independent-mini-backend-check` | **PASS** 35/35 | 13→17→21→27→31→35 (`let`/`while`/closures/`loop`/`!=`+higher-order), this session |
| full `bootstrap check` | not default primary | longer; includes stage matrix |
