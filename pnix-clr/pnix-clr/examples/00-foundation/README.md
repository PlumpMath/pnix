# 00 — foundation (clr seed)

First step of the **core 00–06** path (see [FOUNDATION_PATH.md](../FOUNDATION_PATH.md)).

From the outer `pnix-clr/` directory:

```sh
./bin/pnix-clr pnix-clr/examples/00-foundation/program.px
./bin/pnix-clr -e '(-7) * (-6)'
./bin/pnix-clr --pnix-meta-smoke
```

The first command exercises the CLR-native seed directly. The second exercises
the source-originated Int64 arithmetic path. The smoke command loads canonical
`bool-01`, `builtin-dead-import-01`, `hasattr-apply-precedence-01`, and
`production-checked-i64-01` cases from the sibling `pnix-meta` tree and
requires each canonical JSON record to match its pin. This example does not
claim float/BigInt/general numeric promotion or primitive-manifest enforcement.

Catalog index: [../README.md](../README.md). Cross-host balance:
monorepo `examples/EXAMPLES_BALANCE.md`.
