"""Thin action-checkpoint layer for pnix semantic actions (proposal 0009).

This module deliberately owns no evaluator, mirror, gate, backup system, or host
machinery. It composes the existing pnix surfaces into one verdict record for a
single proposed action step.
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

# Absolute imports: bypass the package's lazy __getattr__ (see the note in
# pnix_mirror.py) so this module stays safe to import before .meta/.proof
# have loaded these submodules some other way.
import pnix_hy.gate as gate
import pnix_hy.ir as ir
import pnix_hy.mirror as mirror
import pnix_hy.pnix_mirror as pm

ACTION_SCHEMA = "pnix-hy.action.report.v0"
ACTION_REPORT_SCHEMA = "pnix-hy.action.report.v0"
ACTION_STATUSES = ("accepted", "held", "rejected")


def _stable_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), default=repr)


def _sha256(value: Any) -> str:
    return hashlib.sha256(_stable_json(value).encode("utf-8")).hexdigest()


def _sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _hash_ref(kind: str, value: Any) -> str:
    return f"{kind}:{_sha256(value)}"


def _before_hash(before_snapshot: Any) -> str | None:
    return None if before_snapshot is None else _sha256(before_snapshot)


def _value_hash(safe: dict[str, Any]) -> str | None:
    if not safe.get("ok"):
        return None
    if safe.get("value_json") is not None:
        return _sha256_text(str(safe["value_json"]))
    return _sha256(safe.get("value"))


def _status_for_gate(g: dict[str, Any]) -> tuple[str, str]:
    if g.get("parse_error"):
        return "rejected", "parse"
    if g.get("allowed"):
        return "accepted", "gate"
    if g.get("denials"):
        return "held", "gate"
    return "rejected", "gate"


def _normalize_granted(granted: tuple[str, ...] | list[str] | str) -> tuple[str, ...]:
    if isinstance(granted, str):
        return (granted,)
    return tuple(granted)


def begin_action(intent: str, before_snapshot: Any = None) -> dict[str, Any]:
    """Start an action checkpoint with only content-addressed references."""
    intent_id = _hash_ref("intent", intent)
    before = _before_hash(before_snapshot)
    action_id = _hash_ref("action", {"intent_id": intent_id, "before_hash": before})
    return {
        "schema": "pnix-hy.action.begin.v0",
        "action_id": action_id,
        "intent": intent,
        "intent_id": intent_id,
        "before_hash": before,
        "rollback_ref": f"before:{before}" if before else None,
    }


def _base_record(
    source: str,
    *,
    intent: str | None,
    granted: tuple[str, ...],
    before_snapshot: Any = None,
) -> dict[str, Any]:
    source_hash = _sha256_text(source)
    before = _before_hash(before_snapshot)
    return {
        "schema": ACTION_SCHEMA,
        "status": "rejected",
        "phase": "start",
        "source": source,
        "intent": intent,
        "source_hash": source_hash,
        "ir_hash": None,
        "value_hash": None,
        "gate": None,
        "explain": None,
        "effects": [],
        "witness_id": None,
        "rollback_ref": f"before:{before}" if before else None,
        "granted": list(granted),
    }


def _stamp_witness(record: dict[str, Any]) -> dict[str, Any]:
    payload = {
        "status": record.get("status"),
        "phase": record.get("phase"),
        "source_sha256": record.get("source_hash"),
        "out_hash": record.get("value_hash"),
        "env_hash": record.get("rollback_ref"),
        "input_kind": "pnix-source",
        "output_kind": "action-verdict",
        "loss_status": "lossless" if record.get("status") == "accepted" else "held",
        "effect_class": ",".join(record.get("effects") or ["pure"]),
        "capability_required": ",".join((record.get("gate") or {}).get("required_effects") or []),
    }
    witness = gate.make_witness("pnix-action", payload)
    record["witness_id"] = witness["witness_id"]
    record["witness"] = witness
    return record


def check_action(
    source: str,
    *,
    intent: str | None = None,
    granted: tuple[str, ...] | list[str] | str = (),
    before_snapshot: Any = None,
    include_roundtrip: bool = False,
) -> dict[str, Any]:
    """Return an accepted/held/rejected action verdict without adding a second VM path."""
    granted_tuple = _normalize_granted(granted)
    record = _base_record(source, intent=intent, granted=granted_tuple, before_snapshot=before_snapshot)
    g = gate.gate_check(source, granted=granted_tuple)
    record["gate"] = g
    record["effects"] = g.get("required_effects") or ["pure"]
    status, phase = _status_for_gate(g)
    if status != "accepted":
        record["status"] = status
        record["phase"] = phase
        record["explain"] = pm.explain_pnix(source, granted=granted_tuple)
        return _stamp_witness(record)

    try:
        ir_bundle = ir.ir_of(source)
        record["ir_hash"] = ir_bundle["ir_sha256"]
    except Exception as exc:  # noqa: BLE001 - parse errors are normal verdicts
        record["status"] = "rejected"
        record["phase"] = "parse"
        record["error"] = f"{type(exc).__name__}: {exc}"
        record["explain"] = pm.explain_pnix(source, granted=granted_tuple)
        return _stamp_witness(record)

    safe = pm.safe_eval(source, granted=granted_tuple)
    record["safe_eval"] = safe
    if not safe.get("ok"):
        record["status"] = "rejected"
        record["phase"] = "eval"
        record["limit_exceeded"] = safe.get("limit_exceeded")
        record["error"] = safe.get("error")
        record["explain"] = pm.explain_pnix(source, granted=granted_tuple)
        return _stamp_witness(record)

    run = mirror.mirror_run(source, trace=False)
    record["mirror"] = {
        "ok": run.get("ok"),
        "run_id": run.get("run_id"),
        "result_sha256": run.get("result_sha256"),
        "facet_count": len(run.get("facets") or []),
    }
    record["roundtrip"] = pm.roundtrip_status(source) if include_roundtrip else None
    record["explain"] = pm.explain_pnix(source, granted=granted_tuple)
    record["value_hash"] = _value_hash(safe) or run.get("result_sha256")
    record["status"] = "accepted" if run.get("ok") and (record["explain"] or {}).get("ok") else "rejected"
    record["phase"] = "eval"
    return _stamp_witness(record)


def verify_action(
    source: str,
    *,
    intent: str | None = None,
    before_snapshot: Any = None,
    granted: tuple[str, ...] | list[str] | str = (),
    include_roundtrip: bool = False,
) -> dict[str, Any]:
    """Attach begin-action hash refs to the action verdict."""
    begun = begin_action(intent or source, before_snapshot)
    verdict = check_action(
        source,
        intent=intent,
        granted=granted,
        before_snapshot=before_snapshot,
        include_roundtrip=include_roundtrip,
    )
    return {
        "schema": "pnix-hy.action.verify.v0",
        "action": begun,
        "verdict": verdict,
        "status": verdict.get("status"),
        "witness_id": verdict.get("witness_id"),
    }


def action_report() -> dict[str, Any]:
    """Self-check: action checkpoints accept, hold, reject, and grant as expected."""
    try:
        accepted = verify_action("let a = 1; in a + 2", intent="compute a small value",
                                 before_snapshot={"workspace": "clean"})
        accepted_again = verify_action("let a = 1; in a + 2", intent="compute a small value",
                                       before_snapshot={"workspace": "clean"})
        held = check_action('builtins.pathExists "/etc/passwd"', granted=())
        granted = check_action('builtins.pathExists "/etc/passwd"', granted=("file-read",))
        rejected = check_action("1 +")
        verdict = accepted["verdict"]
        ready = (
            accepted.get("status") == "accepted"
            and accepted.get("witness_id") == accepted_again.get("witness_id")
            and held.get("status") == "held"
            and held.get("phase") == "gate"
            and held.get("effects") == ["file-read"]
            and granted.get("status") == "accepted"
            and rejected.get("status") == "rejected"
            and rejected.get("phase") == "parse"
            and verdict.get("schema") == ACTION_SCHEMA
            and bool(verdict.get("ir_hash"))
            and bool(verdict.get("value_hash"))
            and bool(verdict.get("rollback_ref"))
            and (verdict.get("mirror") or {}).get("facet_count", 0) > 0
            and (verdict.get("roundtrip") is None
                 or (verdict.get("roundtrip") or {}).get("schema") == "pnix-hy.roundtrip-status.v0")
        )
        return {
            "schema": ACTION_REPORT_SCHEMA,
            "ready": bool(ready),
            "available": True,
            "statuses": {
                "accepted": accepted.get("status"),
                "held": held.get("status"),
                "granted": granted.get("status"),
                "rejected": rejected.get("status"),
            },
            "accepted": {
                "source_hash": verdict.get("source_hash"),
                "ir_hash": verdict.get("ir_hash"),
                "value_hash": verdict.get("value_hash"),
                "witness_id": verdict.get("witness_id"),
                "rollback_ref": verdict.get("rollback_ref"),
            },
            "held_effects": held.get("effects"),
            "deterministic_witness": accepted.get("witness_id") == accepted_again.get("witness_id"),
        }
    except Exception as exc:  # noqa: BLE001
        return {
            "schema": ACTION_REPORT_SCHEMA,
            "ready": False,
            "available": False,
            "error": f"{type(exc).__name__}: {exc}",
        }


__all__ = [
    "ACTION_REPORT_SCHEMA",
    "ACTION_SCHEMA",
    "ACTION_STATUSES",
    "action_report",
    "begin_action",
    "check_action",
    "verify_action",
]
