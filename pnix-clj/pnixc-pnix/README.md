# pnixc-pnix (data-only model)


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This root-level document is a conceptual, operational, governance, or historical support surface.
Canonical repository direction lives in `prd.md`, `todo.md`, and `todo-3d.md`.
The current convergence base remains the shared substrate for state, meaning, observation, plan, and evidence; `pnix`, `freecat`, and pnixc-meta closed-action/receipt lanes should be read as projections over that substrate rather than separate final ontologies. Historical client/control lanes are absorbed into pnixc-meta mirror primitives.

This directory contains a **data-only** pnix model of the pnixc compiler
pipeline. It is used for Stage0 subset checks and IR emission in the
meta-circular runner, but it is **not** an executable compiler yet.

Design goals:
- strict pnix-subset-v1 compliance
- no IO/import/builtin side effects
- deterministic, reproducible metadata

Files:
- pnixc.px: compiler pipeline overview
- driver.px: CLI model (flags/modes)
- exec/plan.px: execution plan (non-executable)
- exec/runtime.px: runtime (expr + module data-only parsing/lowering)
- ast/pnix_ast.px: pnix AST schema (data-only)
- ast/unified_ast.px: unified AST schema (data-only)
- lower/parse.px, lower/lower.px: frontend pipeline steps
- emit/*.px: emission targets (ir/ssa/aot)
- version.px: version stamp

When pnixc-in-pnix becomes executable, these files should remain as
stable, audited metadata used by the proof log and regression tests.

The data-only model is validated by `pnixc_model_runner` and serialized
to `tmp/pnixc/proof/pnixc-model.json` during the meta-circular run.
