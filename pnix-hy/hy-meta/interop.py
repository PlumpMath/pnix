"""Host-side interop adapter for hy-meta SR5.

pnix-hy owns pnix value semantics. This module owns live Python/Hy object refs, host
callable invocation, exception capture, and host witness emission.
"""

from __future__ import annotations

import inspect
import sys
from pathlib import Path
from typing import Any

_HERE = str(Path(__file__).resolve().parent)
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

from witness import conversion_witness, make_witness, record_witness  # noqa: E402

OPAQUE_KEY = "__hy_meta_opaque__"
_OPAQUE: dict[int, Any] = {}


def make_opaque_ref(obj: Any, kind: str = "host-object") -> dict[str, Any]:
    """Keep a live host object behind an opaque ref and emit a conversion witness."""
    rid = id(obj)
    _OPAQUE[rid] = obj
    witness = conversion_witness(
        {
            "operation": "make-opaque-ref",
            "kind": kind,
            "object_type": type(obj).__name__,
            "repr": repr(obj)[:120],
        }
    )
    return {
        OPAQUE_KEY: rid,
        "kind": kind,
        "repr": repr(obj)[:120],
        "witness_id": witness["witness_id"],
    }


def is_opaque_ref(value: Any) -> bool:
    return isinstance(value, dict) and OPAQUE_KEY in value


def resolve_opaque(ref: dict[str, Any]) -> Any:
    if not is_opaque_ref(ref):
        raise KeyError("not a hy-meta opaque ref")
    return _OPAQUE[ref[OPAQUE_KEY]]


def opaque_ref_id(ref: dict[str, Any]) -> int:
    if not is_opaque_ref(ref):
        raise KeyError("not a hy-meta opaque ref")
    return int(ref[OPAQUE_KEY])


def inspect_object(ref_or_obj: Any) -> dict[str, Any]:
    """Return safe host object metadata plus a resolvable witness_id."""
    obj = resolve_opaque(ref_or_obj) if is_opaque_ref(ref_or_obj) else ref_or_obj
    try:
        signature = str(inspect.signature(obj)) if callable(obj) else None
    except Exception:  # noqa: BLE001
        signature = None
    info = {
        "type": type(obj).__name__,
        "module": getattr(type(obj), "__module__", ""),
        "callable": callable(obj),
        "signature": signature,
        "repr": repr(obj)[:120],
    }
    witness = record_witness("inspect-object", info)
    return {**info, "witness_id": witness["witness_id"]}


def opaque_allowed_methods(ref_or_obj: Any) -> list[str]:
    """List public callable methods available for explicit method-level interop."""
    obj = resolve_opaque(ref_or_obj) if is_opaque_ref(ref_or_obj) else ref_or_obj
    methods: list[str] = []
    for name in dir(obj):
        if name.startswith("_"):
            continue
        try:
            value = getattr(obj, name)
        except Exception:  # noqa: BLE001
            continue
        if callable(value):
            methods.append(name)
    return sorted(methods)


def call_opaque(
    ref_or_callable: Any,
    args: tuple[Any, ...] | list[Any] = (),
    kwargs: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Call a host callable, capturing exceptions instead of leaking them across interop."""
    fn = resolve_opaque(ref_or_callable) if is_opaque_ref(ref_or_callable) else ref_or_callable
    kwargs = dict(kwargs or {})
    call_payload = {
        "callable_type": type(fn).__name__,
        "arg_count": len(args),
        "kwarg_names": sorted(kwargs),
    }
    try:
        value = fn(*tuple(args), **kwargs)
    except Exception as exc:  # noqa: BLE001
        error = {
            "type": type(exc).__name__,
            "message": str(exc),
        }
        witness = record_witness("host-call-exception", {**call_payload, "exception": error})
        return {
            "ok": False,
            "exception": error,
            "witness_id": witness["witness_id"],
        }
    witness = record_witness(
        "host-call",
        {**call_payload, "result_type": type(value).__name__, "result_repr": repr(value)[:120]},
    )
    return {
        "ok": True,
        "value": value,
        "value_type": type(value).__name__,
        "witness_id": witness["witness_id"],
    }


def call_method(
    ref_or_obj: Any,
    method_name: str,
    args: tuple[Any, ...] | list[Any] = (),
    kwargs: dict[str, Any] | None = None,
    allowed_methods: tuple[str, ...] | list[str] | None = None,
) -> dict[str, Any]:
    """Call a public host-object method, reusing call_opaque for witness + exception capture."""
    obj = resolve_opaque(ref_or_obj) if is_opaque_ref(ref_or_obj) else ref_or_obj
    if method_name.startswith("_"):
        witness = record_witness("host-method-denied", {"method": method_name, "reason": "private"})
        return {"ok": False, "denied": "private method", "witness_id": witness["witness_id"]}
    allowed = set(allowed_methods) if allowed_methods is not None else set(opaque_allowed_methods(obj))
    if method_name not in allowed:
        witness = record_witness("host-method-denied", {"method": method_name, "reason": "not allowed"})
        return {"ok": False, "denied": "method not allowed", "witness_id": witness["witness_id"]}
    try:
        method = getattr(obj, method_name)
    except Exception as exc:  # noqa: BLE001
        witness = record_witness("host-method-exception", {"method": method_name, "exception": str(exc)})
        return {"ok": False, "exception": {"type": type(exc).__name__, "message": str(exc)},
                "witness_id": witness["witness_id"]}
    if not callable(method):
        witness = record_witness("host-method-denied", {"method": method_name, "reason": "not callable"})
        return {"ok": False, "denied": "method not callable", "witness_id": witness["witness_id"]}
    result = call_opaque(method, args, kwargs)
    result["method"] = method_name
    return result


opaque_call_method = call_method


def release(ref: dict[str, Any]) -> dict[str, Any]:
    """Release an opaque host ref from the process-local registry."""
    if not is_opaque_ref(ref):
        raise KeyError("not a hy-meta opaque ref")
    removed = _OPAQUE.pop(ref[OPAQUE_KEY], None) is not None
    witness = record_witness(
        "release-opaque-ref",
        {"kind": ref.get("kind"), "removed": removed},
    )
    return {"released": removed, "witness_id": witness["witness_id"]}


def interop_report() -> dict[str, Any]:
    try:
        ref = make_opaque_ref(len, "host-callable")
        text_ref = make_opaque_ref("abc", "host-object")
        info = inspect_object(ref)
        called = call_opaque(ref, ([1, 2, 3],))
        failed = call_opaque(ref, ())
        methods = opaque_allowed_methods(text_ref)
        method_called = call_method(text_ref, "upper")
        method_denied = call_method(text_ref, "__class__")
        released = release(ref)
        ready = (
            is_opaque_ref(ref)
            and opaque_ref_id(ref) == ref[OPAQUE_KEY]
            and info["callable"] is True
            and called["ok"] is True
            and called["value"] == 3
            and failed["ok"] is False
            and "upper" in methods
            and method_called["ok"] is True
            and method_called["value"] == "ABC"
            and method_denied["ok"] is False
            and released["released"] is True
        )
        return {
            "schema": "hy-meta.interop.report.v0",
            "ready": ready,
            "available": True,
            "inspect_witness_id": info["witness_id"],
            "call_witness_id": called["witness_id"],
            "exception_witness_id": failed["witness_id"],
            "method_witness_id": method_called["witness_id"],
            "release_witness_id": released["witness_id"],
        }
    except Exception as exc:  # noqa: BLE001
        return {
            "schema": "hy-meta.interop.report.v0",
            "ready": False,
            "available": False,
            "error": f"{type(exc).__name__}: {exc}",
        }


__all__ = [
    "OPAQUE_KEY",
    "call_opaque",
    "call_method",
    "inspect_object",
    "interop_report",
    "is_opaque_ref",
    "make_witness",
    "make_opaque_ref",
    "opaque_allowed_methods",
    "opaque_call_method",
    "opaque_ref_id",
    "release",
    "resolve_opaque",
]
