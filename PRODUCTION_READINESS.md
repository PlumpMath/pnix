# PNIX production-readiness contract

`production-ready` in this repository means **ready to start writing
`pnix-meta` as one shared `.px` stdlib**, not that the stdlib already exists.
The order is fixed:

1. each host language closes its own host-meta self-host/fixed-point ladder at
   Stage 15 or beyond;
2. each `pnix-host` evaluates PNIX directly without requiring proof policy;
3. PNIX can express and execute a recursive PNIX-in-PNIX interpreter;
4. host applications can import the runtime and receive native host data;
5. the product has an exercised composition boundary with its host-meta;
6. the same `.px` library bytes import and run in all five products.
7. a host can call a named curried export from that `.px` library through a
   JSON-safe pure-data boundary without serializing a closure.

The executable floor is `bin/production-readiness-gate`.  It verifies the
four portable fixture hashes, executes the host-owned drivers, then requires
all five installed/public library imports with zero skips.  `--full` adds the
five local export/package smokes and the long host aggregate gates. For Hy it
uses `HY_META_PYTHON`/`PNIX_HY_PYTHON` when supplied, otherwise resolves the
flake-pinned `proofPython` with Nix.

## What this closes

| Contract | clj | hy | rs | cljs | clr |
|---|---:|---:|---:|---:|---:|
| Direct PNIX runtime | gate | gate | gate | gate | gate |
| Identical `.px` import seed | gate | gate | gate | gate | gate |
| PNIX-written recursive interpreter | gate | gate | gate | gate | gate |
| Public host import + native data projection | gate | gate | gate | gate | gate |
| Host-meta live execution / fixed-point evidence | gate | gate | gate | gate | gate |
| Host -> exported `.px` function (pure data) | `call-file` | `call_file` | `call_file_json` | `callFile*` | `Eval.CallFile` |

The portable files stay copied into each self-contained host tree; no product
depends on a sibling checkout.  The root gate only detects drift.

## Live validation snapshot (2026-08-22)

This is evidence for the shared floor, not a percentage estimate of every
host-specific research feature.

| Pair | Host-meta evidence | pnix-host / boundary evidence |
|---|---|---|
| clj | Stage15/N metacircular gate READY | foundation 141 tests / 3,666 assertions; compiler smoke 159/159; conformance 116/116 plus 22/22 negative cases |
| hy | self-host + raw-pyc fixed point reproduced; Stage15/N closure reproduced | runtime 1,243/1,243; shared corpus 1,273/1,273; four mirror lanes 495 each; toolkit 73/73 |
| rs | `rs-meta check` PASS through Stage15/N aggregate replay | `all_ready: true`; tower 47/47; interop 4/4; substrate 3-way; receipt `80e326ddd28c434d03178731d4f4ed9673393d08d76ed4d955b0c8c8d71d6ab9` |
| cljs | self-host matrix + fixed-point runtime PASS; independent mini backend 41 fixtures | aggregate JavaScript runtime matrix and capability drift gate PASS |
| clr | Compiler Stage1→15/N and StageN PASS; Stage1→7 self-reproduction hash identical | 24 tests / 235 assertions; process/in-process parity 17/17; production outcome gate PASS |

The common gate also passed all five portable drivers and all ten installed
host-import checks with zero skips. The five local export/package smokes each
called `double 21` through the host API and received `42`.

## Boundary that is still intentionally open

Native values, source/file evaluation, imports, pure exported-function calls,
errors, and fixed-point composition now have one common floor. Arbitrary in-memory host callbacks
and opaque host-object lifecycles do **not** yet have one equal ABI:

- clj and hy already expose richer effect/capability and callable/opaque APIs;
- rs has a capability-checked, data-only host boundary and intentionally no
  opaque `PxVal` variant;
- cljs exposes native JS result data but not a stable callback-handle ABI;
- clr exposes C# process/in-process evaluation but not a stable callback-handle
  ABI across the ClojureCLR boundary.

Those differences must not be relabeled as parity.  A future common callback
ABI needs an explicit ownership, laziness, error, capability, and serialization
contract before implementation.  It is separate from starting the portable
`.px` stdlib, whose code should not depend on host-object identity.

`pnix-meta` itself remains deferred: the current `library.px` is only a small
portability fixture, not the beginning of the real stdlib catalog.
