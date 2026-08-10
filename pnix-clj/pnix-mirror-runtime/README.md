# pnix-mirror-runtime

> **`.px` project, built by pnixc-meta.** Not a Rust crate.
>
> 2026-06-02 terminology guard: this directory is the pnixc-meta mirror
> primitive `.px` surface. It should not be described with retired product
> labels; hosts are bootstrap/transport only, while semantic primitive law
> stays in `.px`.

## What this is

The substrate-execution layer reimplemented in `.px`. Hosts the 4
mirror-spawn primitives that the 2026-05-13 walking experiments
(R10/R7/R8/R9) confirmed are missing from the current substrate:

- **P1 mirror-identity-registry** — typed, replay-stable identity
  per mirror spawn
- **P2 ontology-extension-event** — explicit ontology delta when a
  mirror is born
- **P3 boundary-projection** — in-mirror live vs out-of-mirror
  reference shadow
- **P4 typed-receipt-stream** — append-only audit chain over all
  primitive events

## Why this replaces the Rust `pnix-query-runtime`

Per OWNER-LAW CONSTITUTION:
> semantic / ontology / learning / floor / coverage / provenance /
> lineage law 는 Pnix/.px owner 로만 구현한다.

The 4 primitives are *substrate semantic law*. They cannot live as
Rust code — they must live in `.px` so the meta-circular substrate
can reason about itself. The Rust crate `pnix-query-runtime`
remains as transitional bootstrap host until this `.px` project is
buildable via `pnixc-meta` end-to-end.

## Build path

```
pnixc (Rust stage 0)
  │ builds pnixc-pnix/*.px → stage1
  ▼
pnixc-meta (stage 2, self-hosted)
  │ builds this directory →
  ▼
pnix-mirror-runtime artifact
  │ host calls (analogous to `turn-exec interpreter.px`)
  ▼
mirror sandbox active — P1-P4 enforce primitives, the 4 axes
(in-mirror vs out / depth / sibling) become observable via P4
receipt stream
```

## Layout

```
pnix-mirror-runtime/
  ├── README.md
  ├── project.px                          # pnixc-meta entry point
  └── primitives/
      ├── p1-mirror-identity-registry.px
      ├── p2-ontology-extension-event.px
      ├── p3-boundary-projection.px
      └── p4-typed-receipt-stream.px
```

## Live-Coding Evaluation Rule

Mirror primitive development must run the platform while coding. The canonical
interpreter target is `pnixc-meta`: this project exists so mirror runtime
surfaces can be evaluated as a pnixc-meta-built `.px` project, not as a final
Rust helper API. When a primitive exposes a new unknown / Held / trace /
verdict / boundary shape, that observation must become a test, harness check,
inventory row, gate, or explicit Held path before the slice closes.

Direct `pnix-query-px-eval` / Rust `eval_to_json` calls are bridge-debt
observation paths. For the generic mirror dispatch operation
(`applyMirrorPlateWithLensDispatch`), pnixc-meta is already the canonical
target; new proof should not treat pnix-eval as canonical. Remaining direct
callers are caller-cleanup debt tracked by the 2026-05-17 convergence plan in
`project-wiki/maps/non-mirror-px-pnixc-meta-migration-plan.md`.

## Ontology-Example Upgrade Criterion

The runtime is not complete merely because primitive exports evaluate. The next
effect criterion is replaying the deterministic ontology baseline from
`ontology-examples.md` as mirror computation:

```text
meaning atom -> mirror plate -> runtime function -> receipt / trace -> next turn
```

That route is tracked in
`project-wiki/maps/ontology-examples-to-mirror-meta-interpreter-map.md`.

## Cross-references

- `project-wiki/maps/ankh-macro-mirror-turn-axiom.md` — vocabulary
- `project-wiki/maps/mirror-spawn-substrate-4-primitives-design.md` — primitive design draft
- `project-wiki/maps/mirror-upgrade-experiment-routes.md` — walking results that triggered this rewrite
- `project-wiki/maps/ontology-examples-to-mirror-meta-interpreter-map.md` — effect criterion for exceeding ontology examples
