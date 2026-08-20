# pnix-executor-graph


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

Executor for applying FxCore graphs and wiring runtime engines.

Quick start:

```bash
pnix-executor-graph --dist dist
```

External inputs (JSON object):

```bash
pnix-executor-graph --dist dist --input gain=0.5 --input name=\"osc\"
```

More details: `docs/executor.md`
