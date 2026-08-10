# clj-meta

Clean Clojure host-language proof lane for `pnix-clj`.

This directory has two separate lanes:

- `stage7-gate.sh` is a reproducible-build lane for stock Clojure 1.12.5.
  It proves deterministic hosted rebuilds; it is not the meta-circular
  compiler proof.
- `src/pnix/clj_meta/compiler.clj` and `selfhost.clj` are the meta lane: a
  Clojure-written analyzer/ASM bytecode compiler plus deterministic self-host
  checks.

The source snapshot lives outside this directory:

```sh
clojure-clojure-1.12.5/
```

Generated stage trees, logs, and proof receipts stay under `clj-meta/` and are
ignored by git:

```text
clj-meta/work/
clj-meta/logs/
clj-meta/proof/
```

## Status / primary gate

See [STATUS.md](STATUS.md). Primary gate: `./bin/clj-meta-gate` (practical floor: `./bin/clj-meta-gate selfhost`).

## Commands

```sh
clj-meta/stage7-gate.sh status
clj-meta/stage7-gate.sh stage7-check
clojure -M:compiler-smoke
clojure -M:conformance
clojure -M:selfhost-check
clojure -M:mirror-smoke
clojure -M:audit-self-source
clojure -M:gate
```

`stage7-check` builds the full hosted replay:

```text
stage1 -> stage2 -> stage3 -> stage4 -> stage5 -> stage6 -> stage7
```

Stages 3 through 7 compile the same Clojure 1.12.5 source snapshot with the
previous stage's Java runtime-only Clojure host jar, then compares the generated
Clojure jars against the previous stage by stable zip entry names and entry
content hashes.

The stage snapshot is patched only inside `clj-meta/work/` to make the upstream
build deterministic across JVM processes: locals clearing is disabled and
closed-over locals are sorted before emitting fn/reify constructor fields.

The final stage compiles and runs:

- `pnix.clj-meta.core`
- `pnix.clj-meta.stm`

## Boundary

Neither lane claims JVM-free Clojure self-hosting. The JVM, Java runtime
classes under the Clojure source tree, Maven, and the local JDK are permanent
substrate. The reproducible-build lane also does not claim a Clojure-written
compiler; it rebuilds stock Clojure with hosted Java compiler infrastructure.

This directory also does not own `pnix-clj` semantics, brain codecs, or redb
ingest. It prepares and validates Clojure host/compiler substrate that
`pnix-clj` can run on.
