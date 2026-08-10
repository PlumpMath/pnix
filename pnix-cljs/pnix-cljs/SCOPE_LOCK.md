# pnix-cljs scope lock

## Product owner

`pnix-cljs` owns only the ClojureScript/JavaScript projection mechanism:

- PNIX source tokenization and parsing
- evaluation over native ClojureScript values
- nominal machine outcome values
- Node and CommonJS interop

## Semantic owner

Portable language meaning belongs to repository-level `pnix-meta`. This host
must consume that meaning rather than retain copied Clojure/JVM runtime trees.
Its native seed may not claim canonical cross-host parity until the shared
conformance corpus is connected and compared by the all-host gate.

## Excluded from this seed

- service policy and admission status
- evaluator fallback
- proof-receipt-gated execution
- JVM/Java/ASM implementation code
- retained effects and filesystem execution
- automatic application code generation
- authoritative string-encoded types
- copied `stdlib`, `pnixc-pnix`, `pnix-mirror-runtime`, or domain-content roots
