# 0009 — pnix-hy as a pnix semantic / action VM (thin action layer)

- Status: **ACCEPTED + IMPLEMENTED 2026-07-02**. Direction accepted by the user; implementation
  shipped as the thin `pnix_hy/action.py` layer, CLI flags, and example 18.
- Scope: pnix-hy only. **Folder structure is preserved**; the one new file is a thin
  `pnix_hy/action.py` (+ small CLI flags + one examples section). No hy-meta change.
- Placeholder/out-of-scope check: reuses the EXISTING pnix VM + gate + mirror + explain +
  witness; adds NO second evaluator/mirror/gate; does NOT touch the host lanes.

## The one-line goal

```
pnix-hy = the pnix language VM on the Hy/Python host, wrapped by
          gate / witness / explain / action-checkpoint into an AI-coding-agent semantic VM.
```

Python/Hy self-hosting은 hy-meta의 일. pnix-hy는 pnix runtime/compiler/evaluator +
safe_eval + purity/effect gate + witness/receipt + mirror/reify/explain + Hy↔pnix projection +
(new) **action checkpoint** 레이어를 소유 — LLM/algorithm step을 accepted/held/rejected
term으로 고정.

## Role lock (owns / does not own)

**pnix-hy owns:** pnix reader/parser · pnix AST/IR · pnix evaluator · pnix compiler/emitter ·
safe_eval · purity/effect gate · witness/receipt · mirror/reify/explain · Hy/Python↔pnix
projection · the action-step semantic layer.

**pnix-hy does NOT own** (hy-meta's, do not reimplement): Python compiler · Python
AST/code-object/pyc/marshal · Hy reader/macro implementation · raw host importlib/sys.modules
control · host introspection core.

## What is ALREADY done (reuse, do NOT rebuild)

Core VM + wrappers exist and are `--gate`-green: pnix eval + compile with `eval==compile`
4-lane parity (545×4), stable IR hash (`ir.py`), meaning-preservation roundtrip
(`hy_to_pnix_value_roundtrip` / `roundtrip_status`), `safe_eval` (+`pure_only`),
`gate_check`/`static_purity_check`, deterministic `make_witness`, singleton `mirror_run` /
`singleton_mirror_run`, unified `explain_pnix`, interop loss/effect/capability.

## The ONLY new work — `pnix_hy/action.py` (thin)

기존 조각을 감싸 한 action step을 판정하는 레이어. 반드시 호출 — 재구현 금지 —
`safe_eval`, `static_purity_check`, `gate.gate_check`, `gate.make_witness`,
`mirror.mirror_run`, `pnix_mirror.explain_pnix`, `roundtrip_status` /
`hy_to_pnix_value_roundtrip`.

Sketch (subject to impl):
```
begin_action(intent, before_snapshot) -> {action_id, intent_id, before_hash}
check_action(source, *, intent=None, granted=()) -> verdict record (no side effects)
verify_action(source, *, before_snapshot=None, granted=()) -> verdict + witness + explain
action_report() -> self-check (accepted/held/rejected paths)
```
Verdict record (schema `pnix-hy.action.report.v0`):
```json
{ "status": "accepted|held|rejected", "phase": "eval",
  "source_hash": "...", "ir_hash": "...", "value_hash": "...",
  "gate": {...}, "explain": {...}, "effects": ["pure"],
  "witness_id": "...", "rollback_ref": "..." }
```
Snapshot/rollback은 **hash references** (before/after content hashes), NOT a large file-backup
system. `held` = needs a capability not granted; `rejected` = impure/limit/gate failure;
`accepted` = pure (or granted) + within bounds + witness stamped.

## Minimum completion criteria (this phase is "closed" when all hold)

1. pnix eval works · 2. pnix compile/run works · 3. `eval == compile` parity ·
4. IR hash stable · 5. roundtrip meaning preserved · 6. action check rejects impure by default
(gated) · 7. gate records required effects · 8. witness/receipt deterministic ·
9. `mirror_run` is the single mirror · 10. `explain_pnix` returns a unified explanation ·
11. Hy↔pnix projection has a loss status · 12. action checkpoint API can accept/hold/reject steps.

(1–5, 7–11 already hold; 6 & 12 are the new deliverables via `action.py`.)

## Forbidden (kept)

- No action governance / LLM-step / file-backup system inside `pnix_runtime.py` (VM core stays clean).
- No second evaluator / mirror / gate. No copying hy-meta host machinery into pnix-hy.
- No host object placed directly into a pnix canonical term (use interop opaque refs).
- New capability entered todo only via this accepted proposal (SCOPE_LOCK §7).

## Implementation result

Shipped:
- `pnix_hy/action.py`: `begin_action`, `check_action`, `verify_action`, `action_report`.
- `cli.py`: `--action-check SRC` and `--action-explain SRC`.
- `examples/18-action-checkpoint/`: plain limit vs pnix-hy action verdict demo.

Verified:
- `--check` 56/56 green.
- `--gate` PASS from the `pnix-hy/` package root.
- Existing eval==compile 545x4, mirror singleton, and meaning-preservation reports unchanged.
- `pnix_runtime.py` untouched by action governance.

## Done-when

`--check` green (existing 55 + new `action` report → 56), `--gate` green, `eval==compile`
parity + mirror-singleton + meaning-preservation reports unchanged (no regression).
