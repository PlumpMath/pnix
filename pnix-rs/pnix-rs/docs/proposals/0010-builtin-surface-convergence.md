# 0010 - builtin surface convergence

Status: phases 1-2 oracle-pinned and tri-host-covered (2026-07-10).
Raw-surface, path/context, and canonical-float convergence remain open.

## Demand

공유 corpus가 호스트 builtin 맵이 관측 가능하게 다르다는 것을 드러냈다.
Discovery baseline은 Nix 이름 118개, pnix-rs에는 77개뿐. Phase 1-2 후
측정 raw tables: Nix 118, rs 91, hy 163, clj 169; pnix 호스트 공통 이름 87,
그중 78이 Nix에 속함. 프로그램은 남은 drift를 `builtins ? x`로 여전히
관측할 수 있다.

## Boundary

이 제안은 두 책임을 분리한다:

- `pnix-rs`는 라이브러리 값 import만으로는 공급할 수 없는 작은 primitive Nix
  표면을 구현 (constants, reflection/effect seams, primitive error control).
- Nix-expressible aliases와 collection helpers는 공통 `.px`에 한 번 두고
  right-biased builtin overlay로 합성. 세 호스트 커널에 복사하지 않음.

첫 native tranche는 의도적으로 작음: `break`, `parseDrvName`, `toPath`,
`tryEval`, 현재 representable value domain 위 `isPath`,
`unsafeDiscardOutputDependency`, `unsafeDiscardStringContext`, 및
`true`/`false`/`null`/version/store constants. IO, fetch, store construction,
derivation operations는 capability/effect 작업으로 남김; fake success path
추가 없음.

Portable overlay는 raw host table mutation이 아니라 explicit composition:

```nix
B: let builtins = B // ((import ./builtin-overlay.px) B); in ...
```

현재 15 Nix-expressible compatibility names 정의. 따라서 unwrapped host
tables에서 `builtins ? append`는 여전히 다름. Standard loader/prelude
composition seam은 별도 follow-up; phase 1을 full raw-surface convergence로
서술하지 말 것.

Context-discard operations는 각 호스트의 현재 string-context model이
represent 가능한 값에서만 exact. pnix-rs는 아직 context payload 없으므로
context-free strings로 의도적으로 제한. Store/context convergence는 열림.

## Evidence and gates

모든 동작은 코드 전에 local `nix-instantiate 2.34.7`에 pin. Shared corpus는
presence와 behavior를 분리 테스트, 포함: discriminating
`parseDrvName "a-1-b-2"` case, lexical `toPath`, signed integer division,
ignored function arguments, overlay short-circuiting, `tryEval`이 `throw`를
잡으면서 division-by-zero는 escape. Literal list elements도 lazy pin
(`length`/`tail`/non-selected `elemAt`이 force하지 않음). Follow-up audit
pins: failed-thunk replay, deferred
`map`/`genList`/`mapAttrs`/`zipAttrsWith` results, dead positions의 missing
imports, non-string `throw`가 `tryEval`이 catch하기 전에 실패한다는 사실.

Gate receipts (2026-07-10):

- pnix-rs: release build; `px-check` 26/26; `gate-check` 24/24;
  `substrate-check` 1/1; aggregate `check` reports `all_ready: true`.
- pnix-hy ship gate: runtime 1113/1113, Rust corpus 1260/1260,
  four-lane parity 449 per lane, toolkit 74/74.
- pnix-clj push-authority gate: 193 tests / 4311 assertions, zero failures;
  compiler smoke 159/159; compiler conformance 116/116 plus 22/22 negative.
- Shared conformance: all hosts ready on 148 cases; `bin/tri-host-gate`
  reports PASS with zero differing rows.

## Phase 2: numeric and hash behavior

Phase 2 primarily closes behavior gaps; the only raw presence addition is the
missing Clojure `hashString` name:

- operators and `builtins.add/sub/mul/div` share checked i64 semantics;
- mixed int/float arithmetic, comparison, nested equality/order, signed
  division, zero division, `ceil`, and `floor` match Nix 2.34.7;
- finite float `toString` uses six decimals, distinguishes unary literal zero
  from arithmetic negative zero, accepts exponent literals, and observes
  NaN/Infinity with Nix spellings and comparison rules;
- recursive equality/order/`elem` preserve Nix's shared nested identity without
  changing top-level function/NaN equality or hiding a shared failing thunk;
- `hashString` supports the Nix default md5/sha1/sha256/sha512 profile, UTF-8,
  exact raw bytes, padding boundaries, lowercase hex, and Nix argument force
  order. Legacy algorithms remain available in the Nix-compatible profile;
  stricter policy belongs in a separate profile, not a divergent raw builtin.

Implementation preserves the meta-first boundary. pnix-rs uses no hash crate;
its self-interpretable code required only generic rs-meta support for numeric
`format!("{:.N}")` and `Rc::ptr_eq`. `substrate-check` executes the new
numeric/hash/shared-identity paths and matches native Rust.

Phase-2 gate receipts (2026-07-10):

- rs-meta `self-check` 407/407, `tv-check` 407/407, and `typeck-check` 272/272;
- pnix-rs `px-check` 30/30, `substrate-check` 1/1, aggregate `all_ready`;
- shared conformance 182/182 (161 conformance + 21 legacy-eval), with 34 new
  cases: 6 value cases and 28 error cases (26 eval + 2 parse);
- Hy runtime 1223/1223, Rust-derived corpus 1270/1270, four-lane mirror
  497/497 per lane, and closure gate PASS;
- Clojure push-authority gate PASS: 196 tests / 4563 assertions, compiler smoke
  159/159, compiler conformance 116/116 plus 22/22 negative;
- `bin/tri-host-gate` PASS with zero differing rows.

This does not close path/string-context semantics: clj/hy verify that hash data
context is discarded and algorithm context is rejected, but rs cannot yet
represent those values. It also does not claim canonical JSON float parity;
exponent spelling/shortest-roundtrip, direct non-finite encoding, common error
classes, and Nix-version policy remain B1 work.
