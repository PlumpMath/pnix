# 14 — runners & REPLs (role-classified)

Two languages, two roles each. This shows every flake runner doing its job.

| runner | role |
|---|---|
| `rs-meta -- run` | RUST interpreter (the meta-circular trusted floor) |
| `rs-meta -- native-run` | RUST compiler (rustc native tier; == interp by translation validation) |
| `repl-pnix-rs-rust` | interactive RUST REPL (drives the rs-meta interpreter) |
| `pnix-rs-pnix -- -f default.px` | PNIX (px) compiler/evaluator over a `.px` file |
| `repl-pnix-rs-pnix` | interactive PNIX (px) REPL |

rs-meta is a subset **meta-circular peer engine** that *uses* the rustc toolchain
— not a cargo/rustc drop-in. It stays a pure floor (no interactive io); **pnix-rs**
drives both REPLs, calling rs-meta as a peer across the bootstrap CLI.

Run: `bash pnix_rs_way.sh` (from a devShell, or with `PNIX_RS`/`RS_META_BOOTSTRAP` set).
