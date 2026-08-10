"""Nominal machine outcomes plus the basic production evaluator adapter."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import sys
from typing import Any, Generic, TypeVar


SCHEMA = "pnix.machine.host-outcome.v1"
PRODUCTION_PROJECTION_SCHEMA = "pnix.production-basic-outcome-projection.v1"
V = TypeVar("V")


@dataclass(frozen=True, slots=True)
class EvalError:
    phase: str
    error_class: str
    evidence: dict[str, Any]


@dataclass(frozen=True, slots=True)
class EffectRequest:
    effect: str
    args: dict[str, Any]


@dataclass(frozen=True, slots=True)
class Continuation:
    id: int


@dataclass(frozen=True, slots=True)
class Checkpoint:
    id: int


@dataclass(frozen=True, slots=True)
class ResourceReason:
    reason_class: str
    divergence_proven: bool = False


@dataclass(frozen=True, slots=True)
class Done(Generic[V]):
    value: V


@dataclass(frozen=True, slots=True)
class Failed:
    error: EvalError


@dataclass(frozen=True, slots=True)
class Requested:
    request: EffectRequest
    continuation: Continuation


@dataclass(frozen=True, slots=True)
class Suspended:
    checkpoint: Checkpoint
    reason: ResourceReason


MACHINE_OUTCOME_TYPES = (Done, Failed, Requested, Suspended)


def is_machine_outcome(value: object) -> bool:
    return isinstance(value, MACHINE_OUTCOME_TYPES)


def observe(outcome: object) -> dict[str, Any]:
    if isinstance(outcome, Done):
        return {"status": "done", "value": outcome.value}
    if isinstance(outcome, Failed):
        return {
            "status": "failed",
            "error": {
                "phase": outcome.error.phase,
                "class": outcome.error.error_class,
                "evidence": outcome.error.evidence,
            },
        }
    if isinstance(outcome, Requested):
        return {
            "status": "requested",
            "request": {
                "effect": outcome.request.effect,
                "args": outcome.request.args,
            },
            "continuation": {"id": outcome.continuation.id},
        }
    if isinstance(outcome, Suspended):
        return {
            "status": "suspended",
            "checkpoint": {"id": outcome.checkpoint.id},
            "reason": {
                "class": outcome.reason.reason_class,
                "divergence_proven": outcome.reason.divergence_proven,
            },
        }
    raise TypeError("value is not a MachineOutcome")


def eval_source_outcome(source: str) -> object:
    from . import pnix_runtime
    try:
        return Done(pnix_runtime.eval_source(source))
    except pnix_runtime.PnixError as exc:
        return Failed(EvalError(exc.phase, exc.error_class, dict(exc.evidence)))


def project_production_outcome(outcome: object) -> dict[str, Any]:
    if isinstance(outcome, Done):
        from . import pnix_runtime
        value_json = json.dumps(
            pnix_runtime.stable_data(outcome.value),
            ensure_ascii=False, sort_keys=True, separators=(",", ":"),
        )
        return {
            "schema": PRODUCTION_PROJECTION_SCHEMA,
            "outcome_kind": "done",
            "error_phase": None, "error_class": None,
            "value_json": value_json,
        }
    if isinstance(outcome, Failed):
        return {
            "schema": PRODUCTION_PROJECTION_SCHEMA,
            "outcome_kind": "failed",
            "error_phase": outcome.error.phase,
            "error_class": outcome.error.error_class,
            "value_json": None,
        }
    raise TypeError("basic projection accepts Done or Failed")


def _production_report(path: str) -> dict[str, Any]:
    matrix: list[dict[str, Any]] = []
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        case, kind, phase, error_class, value_json, source = line.split("\t", 5)
        projection = project_production_outcome(eval_source_outcome(source))
        expected = {
            "schema": PRODUCTION_PROJECTION_SCHEMA,
            "outcome_kind": kind,
            "error_phase": phase or None,
            "error_class": error_class or None,
            "value_json": value_json or None,
        }
        matrix.append({
            "case": case,
            "matches_expected": projection == expected,
            "projection": projection,
        })
    return {
        "host": "pnix-hy", "host_outcome_schema": SCHEMA, "matrix": matrix,
        "model_schema": "pnix.machine.eval-outcome-model.v1",
        "schema": "pnix.production-basic-outcome-report.v1",
        "status": {
            "automatic_codegen": False,
            "basic_language_errors_are_held": False,
            "legacy_error_transport_is_semantic_owner": False,
            "production_basic_outcome_convergence_v1": True,
            "production_common_machine_replacement": False,
            "production_requested_integration": False,
            "production_suspension_equivalence": False,
        },
    }


def self_check() -> dict[str, Any]:
    done = Done("value")
    failed = Failed(EvalError("eval", "not-callable", {}))
    requested = Requested(EffectRequest("open", {}), Continuation(1))
    suspended = Suspended(
        Checkpoint(2),
        ResourceReason("resource-budget-exhausted", False),
    )
    guest_shape = {"outcome_kind": "done"}

    done_observed = observe(done)
    failed_observed = observe(failed)
    requested_observed = observe(requested)
    suspended_observed = observe(suspended)

    assert done_observed["status"] == "done"
    assert failed_observed["error"]["phase"] == "eval"
    assert failed_observed["error"]["class"] == "not-callable"
    assert requested_observed["request"]["effect"] == "open"
    assert suspended_observed["reason"]["divergence_proven"] is False
    assert not is_machine_outcome(guest_shape)

    return {
        "all_ok": True,
        "done": done_observed["status"],
        "failed_class": failed_observed["error"]["class"],
        "failed_phase": failed_observed["error"]["phase"],
        "guest_shape_is_outcome": is_machine_outcome(guest_shape),
        "requested": requested_observed["status"],
        "requested_effect": requested_observed["request"]["effect"],
        "schema": SCHEMA,
        "suspended": suspended_observed["status"],
        "suspended_divergence_proven": suspended_observed["reason"][
            "divergence_proven"
        ],
    }


def main() -> None:
    report = (
        _production_report(sys.argv[2])
        if len(sys.argv) == 3 and sys.argv[1] == "--production"
        else self_check()
    )
    print(
        json.dumps(
            report,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
