"""Closed checked-i64 primitive kernel for the production pnix-hy evaluator."""

from __future__ import annotations

import json
from typing import Any


LANE_CLASSIFICATION = {
    "lane": "core",
    "scope": "production-checked-i64-primitive-kernel",
    "role": "closed-native-primitive-kernel",
    "semantic_authority": "pnix-meta-manifest-only",
}

ABI_VERSION = "pnix.primitive-abi.v1"
MANIFEST_DIGEST = "f133ee0f3a5c6073eabb6855f3abf44bf36366083f26fbe76e9524521a2a5fd6"
I64_MIN = -(2**63)
I64_MAX = 2**63 - 1

CHECKED_INTEGER_PRIMITIVE_IDS = [
    "i64-add-checked",
    "i64-sub-checked",
    "i64-mul-checked",
    "i64-div-checked",
]

OPERATOR_TO_PRIMITIVE_ID = {
    "+": "i64-add-checked",
    "-": "i64-sub-checked",
    "*": "i64-mul-checked",
    "/": "i64-div-checked",
}


def _ok(value: int) -> dict[str, Any]:
    return {"kind": "ok", "value": value}


def _failure(phase: str, error_class: str) -> dict[str, Any]:
    return {"kind": "error", "phase": phase, "class": error_class}


def _contract_failure() -> dict[str, Any]:
    return _failure("primitive-contract", "primitive-contract-violation")


def _is_i64(value: Any) -> bool:
    return type(value) is int and I64_MIN <= value <= I64_MAX


def _checked_operation(primitive_id: str, left: int, right: int) -> dict[str, Any]:
    if primitive_id == "i64-add-checked":
        value = left + right
    elif primitive_id == "i64-sub-checked":
        value = left - right
    elif primitive_id == "i64-mul-checked":
        value = left * right
    elif primitive_id == "i64-div-checked":
        if right == 0:
            return _failure("eval", "division-by-zero")
        if left == I64_MIN and right == -1:
            return _failure("eval", "integer-overflow")
        value = abs(left) // abs(right)
        if (left < 0) != (right < 0):
            value = -value
    else:
        return _contract_failure()
    if not _is_i64(value):
        return _failure("eval", "integer-overflow")
    return _ok(value)


def invoke(request: dict[str, Any]) -> dict[str, Any]:
    try:
        if request.get("abi_version") != ABI_VERSION:
            return _contract_failure()
        if request.get("manifest_sha256") != MANIFEST_DIGEST:
            return _contract_failure()
        primitive_id = request.get("primitive_id")
        if primitive_id not in CHECKED_INTEGER_PRIMITIVE_IDS:
            return _contract_failure()
        operands = request.get("operands")
        if not isinstance(operands, (list, tuple)) or len(operands) != 2:
            return _contract_failure()
        if not all(_is_i64(operand) for operand in operands):
            return _failure("eval", "type-error")
        return _checked_operation(primitive_id, operands[0], operands[1])
    except Exception:
        return _contract_failure()


def _legacy_invoke(operator: str, left: int, right: int) -> dict[str, Any]:
    try:
        if operator == "+":
            value = left + right
        elif operator == "-":
            value = left - right
        elif operator == "*":
            value = left * right
        elif operator == "/":
            if right == 0:
                return _failure("eval", "division-by-zero")
            value = abs(left) // abs(right)
            if (left < 0) != (right < 0):
                value = -value
        else:
            return _contract_failure()
        if not _is_i64(value):
            return _failure("eval", "integer-overflow")
        return _ok(value)
    except Exception:
        return _contract_failure()


def invoke_shadow(operator: str, left: int, right: int) -> dict[str, Any]:
    primitive_id = OPERATOR_TO_PRIMITIVE_ID.get(operator)
    if primitive_id is None:
        return _contract_failure()
    legacy = _legacy_invoke(operator, left, right)
    routed = invoke(
        {
            "abi_version": ABI_VERSION,
            "manifest_sha256": MANIFEST_DIGEST,
            "primitive_id": primitive_id,
            "operands": [left, right],
        }
    )
    return routed if legacy == routed else _contract_failure()


def _public_outcome(case_name: str, outcome: dict[str, Any]) -> dict[str, Any]:
    return {"case": case_name, **outcome}


def _matrix() -> list[dict[str, Any]]:
    cases = [
        ("add-positive", "+", 1, 2),
        ("sub-signed", "-", -7, 5),
        ("mul-signed", "*", -7, -6),
        ("div-negative-left", "/", -7, 3),
        ("div-negative-right", "/", 7, -3),
        ("add-overflow", "+", I64_MAX, 1),
        ("sub-overflow", "-", I64_MIN, 1),
        ("mul-overflow", "*", I64_MAX, 2),
        ("div-overflow", "/", I64_MIN, -1),
        ("division-by-zero", "/", 1, 0),
    ]
    return [_public_outcome(name, invoke_shadow(op, left, right))
            for name, op, left, right in cases]


def _contract_matrix() -> list[dict[str, Any]]:
    base = {
        "abi_version": ABI_VERSION,
        "manifest_sha256": MANIFEST_DIGEST,
        "primitive_id": "i64-add-checked",
        "operands": [1, 2],
    }
    requests = [
        ("wrong-abi", {**base, "abi_version": "wrong"}),
        ("wrong-digest", {**base, "manifest_sha256": "wrong"}),
        ("unknown-id", {**base, "primitive_id": "unknown"}),
        ("wrong-arity", {**base, "operands": [1]}),
    ]
    return [_public_outcome(name, invoke(request)) for name, request in requests]


def report() -> dict[str, Any]:
    strict_args = {primitive_id: [0, 1]
                   for primitive_id in CHECKED_INTEGER_PRIMITIVE_IDS}
    validation_errors = {primitive_id: ["type-error"]
                         for primitive_id in CHECKED_INTEGER_PRIMITIVE_IDS}
    return {
        "schema": "pnix.production-primitive-gate.v1",
        "abi_version": ABI_VERSION,
        "manifest_digest": MANIFEST_DIGEST,
        "checked_integer_primitive_ids": CHECKED_INTEGER_PRIMITIVE_IDS,
        "strict_args": strict_args,
        "execution_error_classes": {
            "i64-add-checked": ["integer-overflow"],
            "i64-sub-checked": ["integer-overflow"],
            "i64-mul-checked": ["integer-overflow"],
            "i64-div-checked": ["division-by-zero", "integer-overflow"],
        },
        "validation_error_classes": validation_errors,
        "force_order": [0, 1],
        "shadow_mode": True,
        "matrix": _matrix(),
        "contract_matrix": _contract_matrix(),
        "status": {
            "production_checked_i64_manifest_enforced": True,
            "production_evaluator_manifest_enforced": False,
            "full_builtin_surface_manifest_enforced": False,
        },
    }


def main() -> None:
    print(json.dumps(report(), sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
