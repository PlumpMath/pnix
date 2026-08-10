"""Host-side witness records for hy-meta SR6."""

from __future__ import annotations

import hashlib
import json
from typing import Any

WITNESS_SCHEMA = "hy-meta.witness.v0"
WITNESS_FIELD_SCHEMA = (
    "direction",
    "source_lang",
    "target_lang",
    "input_kind",
    "output_kind",
    "loss_status",
    "effect_class",
    "capability_required",
    "in_hash",
    "out_hash",
    "env_hash",
    "status",
    "loss",
)

_WITNESSES: dict[str, dict[str, Any]] = {}


def _canonical(payload: Any) -> str:
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), default=repr)


def witness_fields(payload: Any) -> dict[str, Any]:
    """Project shared witness fields out of payload without owning payload semantics."""
    if not isinstance(payload, dict):
        return {field: None for field in WITNESS_FIELD_SCHEMA}
    in_hash = payload.get("in_hash") or payload.get("input_sha256") or payload.get("source_sha256")
    out_hash = (
        payload.get("out_hash")
        or payload.get("output_sha256")
        or payload.get("result_sha256")
        or payload.get("code_sha256")
    )
    env_hash = payload.get("env_hash") or payload.get("environment_sha256")
    loss_status = payload.get("loss_status")
    return {
        "direction": payload.get("direction"),
        "source_lang": payload.get("source_lang"),
        "target_lang": payload.get("target_lang"),
        "input_kind": payload.get("input_kind"),
        "output_kind": payload.get("output_kind"),
        "loss_status": loss_status,
        "effect_class": payload.get("effect_class"),
        "capability_required": payload.get("capability_required"),
        "in_hash": in_hash,
        "out_hash": out_hash,
        "env_hash": env_hash,
        "status": payload.get("status"),
        "loss": payload.get("loss") or loss_status,
    }


def make_witness(kind: str, payload: Any) -> dict[str, Any]:
    """Create a deterministic content-hashed host witness record."""
    canonical = _canonical({"kind": kind, "payload": payload})
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    return {
        "schema": WITNESS_SCHEMA,
        "kind": kind,
        **witness_fields(payload),
        "payload": payload,
        "sha256": digest,
        "witness_id": digest[:16],
    }


def record_witness(kind: str, payload: Any) -> dict[str, Any]:
    """Create and retain a witness so interop records can resolve by witness_id."""
    witness = make_witness(kind, payload)
    _WITNESSES[witness["witness_id"]] = witness
    return witness


def resolve_witness(witness_id: str) -> dict[str, Any] | None:
    """Resolve a host witness_id produced in this process."""
    return _WITNESSES.get(witness_id)


def conversion_witness(payload: Any) -> dict[str, Any]:
    return record_witness("conversion", payload)


def replay_witness(payload: Any) -> dict[str, Any]:
    return record_witness("clean-replay", payload)


def drift_witness(payload: Any) -> dict[str, Any]:
    return record_witness("drift", payload)


def witness_report() -> dict[str, Any]:
    first = record_witness("probe", {"value": 42, "name": "answer"})
    second = make_witness("probe", {"name": "answer", "value": 42})
    interop = make_witness(
        "interop",
        {
            "direction": "pnix->host",
            "source_lang": "pnix",
            "target_lang": "python",
            "input_kind": "int",
            "output_kind": "int",
            "loss_status": "lossless",
            "effect_class": "pure",
            "source_sha256": "a" * 64,
            "result_sha256": "b" * 64,
            "env_hash": "c" * 64,
            "status": "ok",
        },
    )
    resolved = resolve_witness(first["witness_id"])
    fields_ready = (
        tuple(WITNESS_FIELD_SCHEMA)
        and interop["direction"] == "pnix->host"
        and interop["loss_status"] == "lossless"
        and interop["loss"] == "lossless"
        and interop["in_hash"] == "a" * 64
        and interop["out_hash"] == "b" * 64
        and interop["env_hash"] == "c" * 64
    )
    return {
        "schema": "hy-meta.witness.report.v0",
        "ready": first["sha256"] == second["sha256"] and resolved == first and fields_ready,
        "deterministic": first["sha256"] == second["sha256"],
        "resolves": resolved == first,
        "field_schema": WITNESS_FIELD_SCHEMA,
        "fields_ready": fields_ready,
        "witness_id": first["witness_id"],
    }


__all__ = [
    "WITNESS_SCHEMA",
    "WITNESS_FIELD_SCHEMA",
    "conversion_witness",
    "drift_witness",
    "make_witness",
    "record_witness",
    "replay_witness",
    "resolve_witness",
    "witness_fields",
    "witness_report",
]
