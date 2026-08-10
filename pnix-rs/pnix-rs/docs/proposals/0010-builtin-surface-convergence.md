# 0010 - builtin surface convergence

Status: phases 1-2 oracle-pinned and tri-host-covered (2026-07-10).
Raw-surface, path/context, and canonical-float convergence remain open.

## Demand

The shared corpus exposed that the host builtin maps are observably different.
The discovery baseline had 118 Nix names and only 77 in pnix-rs. After phases
1-2 the measured raw tables are Nix 118, rs 91, hy 163, and clj 169; 87 names
are common to the pnix hosts and 78 of those belong to Nix. Programs can still
observe the remaining drift with `builtins ? x`.

## Boundary

This proposal separates two responsibilities:

- `pnix-rs` implements the small primitive Nix surface that cannot be supplied
  merely by importing a library value (constants, reflection/effect seams, and
  primitive error control).
- Nix-expressible aliases and collection helpers live once in common `.px` and
  are composed as a right-biased builtin overlay. They are not copied into
  three host kernels.

The first native tranche is deliberately small: `break`, `parseDrvName`,
`toPath`, `tryEval`, `isPath` over the currently representable value domain,
`unsafeDiscardOutputDependency`, `unsafeDiscardStringContext`, and the
`true`/`false`/`null`/version/store constants. IO, fetch, store construction,
and derivation operations remain capability/effect work; no fake success path
is added.

The portable overlay is explicit composition, not mutation of the raw host
table:

```nix
B: let builtins = B // ((import ./builtin-overlay.px) B); in ...
```

It currently defines 15 Nix-expressible compatibility names. Therefore
`builtins ? append` still differs on the unwrapped host tables. A standard
loader/prelude composition seam is a separate follow-up; phase 1 must not be
described as full raw-surface convergence.

The context-discard operations are exact only on values representable by each
host's current string-context model. pnix-rs has no context payload yet, so its
implementation is intentionally limited to context-free strings. Store and
context convergence remain open.

## Evidence and gates

Every behavior is pinned against local `nix-instantiate 2.34.7` before code.
The shared corpus tests presence and behavior separately, including the
discriminating `parseDrvName "a-1-b-2"` case, lexical `toPath`, signed integer
division, ignored function arguments, overlay short-circuiting, and `tryEval`
catching `throw` while allowing division-by-zero to escape. Literal list
elements are also pinned lazy (`length`/`tail`/non-selected `elemAt` do not
force them). Follow-up audit pins cover failed-thunk replay, deferred
`map`/`genList`/`mapAttrs`/`zipAttrsWith` results, missing imports in dead
positions, and the fact that a non-string `throw` fails before `tryEval` can
catch it.

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

The implementation preserves the meta-first boundary. pnix-rs uses no hash
crate; its self-interpretable code required only generic rs-meta support for
numeric `format!("{:.N}")` and `Rc::ptr_eq`. `substrate-check` executes the new
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
