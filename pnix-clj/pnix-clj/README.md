
## Scope lock

See [`SCOPE_LOCK.md`](./SCOPE_LOCK.md).

`pnix-clj` core is limited to the Clojure-hosted pnix meta-circular proof lane:
parser, lowering, evaluator, CAS, store, snapshot, mirror, purity, determinism,
tower, witness, receipt, replay, and the `clj-meta` host proof lane.

NL/MSV/Hangul, Korean mirror/dictionary, graph-gate/gate-graph, multi-language
emit, coding-agent emit, puck-cli bridge, tick runner, redb ingest brain, and
NL meaning graph lanes are out of scope for core.
