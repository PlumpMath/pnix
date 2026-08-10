"""pnix_hy.interop -- the explicit Hy/Python <-> pnix interop boundary (SEP IB1-IB2).

Before this module, "interop" was implicit: the projection/synthesis toolkit
(pnix_to_hy_form / synthesize_pnix_from_hy / *_roundtrip) plus de-facto value mapping via
rt.stable_data. There was NO explicit protocol record and NO loss/effect/capability
marking, and host objects could in principle leak into pnix terms.

This module makes the boundary explicit and bidirectional:
- IB1: a typed `InteropRecord` for every conversion (direction, kinds, loss-status,
  effect-class, capability-required, witness-id).
- IB2: value mapping with an OPAQUE-REF type. Pure data crosses losslessly; host
  callables / modules / arbitrary objects become opaque refs (they MUST NOT enter pnix
  canonical terms directly) and carry an effect-class + capability requirement.
- a simple effect/capability gate (`check_capability`).

Pnix-side only; the host-side adapter (opaque Python object control, callable invocation)
is hy-meta's SR5. Works with the mirror OFF -- interop is conversion, not observation.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass
import importlib
import importlib.util
import inspect
import os
from pathlib import Path
import sys
import tempfile
from typing import Any


def _hy_meta_dir() -> Path:
    """Locate the hy-meta tree: `PNIX_HY_HOME` override (for an off-tree install) else the repo
    sibling of pnix_hy (so in-repo / flake behavior is unchanged)."""
    home = os.environ.get("PNIX_HY_HOME")
    local = Path(__file__).resolve().parents[2] / "hy-meta"
    if home:
        configured = Path(home).expanduser().resolve() / "hy-meta"
        if (configured / "io_capability.py").is_file():
            return configured
    return local

from . import pnix_runtime as rt

LOSS_STATUSES = ("lossless", "lossy", "opaque", "effectful", "unsupported", "dangerous")
EFFECT_CLASSES = (
    "pure", "host-call", "import", "file-read", "file-write", "subprocess", "network",
    "global-mutation", "module-mutation", "unknown",
)
IMPURE_BUILTIN_EFFECTS: dict[str, str] = {
    "getEnv": "host-call",
    "currentTime": "host-call",
    "currentSystem": "host-call",
    "trace": "host-call",
    "traceVerbose": "host-call",
    "readFile": "file-read",
    "readDir": "file-read",
    "readFileType": "file-read",
    "pathExists": "file-read",
    "hashFile": "file-read",
    "storePath": "file-read",
    "toFile": "file-write",
    "scopedImport": "import",
    "exec": "subprocess",
    "getFlake": "network",
    "fetchurl": "network",
    "fetchTarball": "network",
    "fetchGit": "network",
    "fetchTree": "network",
}
IMPURE_BUILTINS = frozenset(IMPURE_BUILTIN_EFFECTS)
_OPAQUE_REFS = ("__pnix_opaque__", "__hy_meta_opaque__")

_FUNCTION_SENTINELS = ("#<pnix-hy-closure>", "#<pnix-hy-native>")
_HY_META_IO: Any | None = None


def _load_hy_meta_io() -> Any:
    """Load the pnix-agnostic I/O substrate from the sibling hy-meta tree."""
    global _HY_META_IO
    if _HY_META_IO is not None:
        return _HY_META_IO
    path = _hy_meta_dir() / "io_capability.py"
    spec = importlib.util.spec_from_file_location("pnix_hy_meta_io_capability", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("hy-meta I/O substrate unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    _HY_META_IO = module
    return module


READ_ONLY_EFFECT_NAMES = frozenset(
    ("fs.path-exists", "fs.open", "fs.file-type", "fs.read-dir")
)


def _effect_field(value: Any, key: str) -> Any:
    return value.get(key) if isinstance(value, dict) else None


def _effect_receipt(effect: str | None, capability_class: Any, executed: bool) -> dict[str, Any]:
    cap = capability_class if isinstance(capability_class, dict) else {}
    return {
        "kind": "effect-request-receipt",
        "effect": effect,
        "risk_tier": cap.get("risk_tier", "unknown"),
        "capability_id": cap.get("capability_id", "unknown"),
        "executed": executed,
        "adapter": "host-meta-io-v1",
    }


class _EffectAdapterResult:
    """Nominal base for internal adapter results; never guest data."""


class _EffectExecuted(_EffectAdapterResult):
    __slots__ = ("operation_id", "value", "receipt")

    def __init__(self, operation_id: str, value: Any, receipt: dict[str, Any]) -> None:
        self.operation_id = operation_id
        self.value = value
        self.receipt = receipt


class _EffectFailed(_EffectAdapterResult):
    __slots__ = ("phase", "error_class", "operation_id", "receipt")

    def __init__(
        self,
        phase: str,
        error_class: str,
        operation_id: str | None,
        receipt: dict[str, Any],
    ) -> None:
        self.phase = phase
        self.error_class = error_class
        self.operation_id = operation_id
        self.receipt = receipt


def _project_effect_adapter_result(result: _EffectAdapterResult) -> dict[str, Any]:
    if isinstance(result, _EffectExecuted):
        return {
            "outcome": "effect-executed",
            "effect": result.operation_id,
            "value": result.value,
            "receipt": result.receipt,
        }
    if isinstance(result, _EffectFailed):
        return {
            "outcome": "failed",
            "error": {"phase": result.phase, "class": result.error_class},
            "effect": result.operation_id,
            "receipt": result.receipt,
        }
    raise TypeError("invalid effect adapter result")


def _failed_effect(
    effect: str | None, capability_class: Any, reason: str
) -> _EffectFailed:
    phase, error_class = {
        "effect-adapter-unsupported": ("effect-contract", "unknown-effect-operation"),
        "effect-args-invalid": ("effect-contract", "invalid-effect-args"),
        "capability-denied": ("effect", "effect-denied"),
    }.get(reason, ("effect", "effect-adapter-error"))
    return _EffectFailed(
        phase,
        error_class,
        effect,
        _effect_receipt(effect, capability_class, False),
    )


def _apply_effect_request_outcome(
    request: Any, granted: tuple[str, ...] | list[str] = ()
) -> _EffectAdapterResult:
    """Execute one validated request and return a nominal adapter result."""
    effect = _effect_field(request, "operation_id")
    args = _effect_field(request, "args")
    capability_class = _effect_field(request, "capability_class")
    path = _effect_field(args, "path")
    grants = tuple(str(item) for item in granted)
    if effect not in READ_ONLY_EFFECT_NAMES:
        return _failed_effect(effect, capability_class, "effect-adapter-unsupported")
    if not isinstance(path, str):
        return _failed_effect(effect, capability_class, "effect-args-invalid")
    if "file-read" not in grants:
        return _failed_effect(effect, capability_class, "capability-denied")
    try:
        meta_io = _load_hy_meta_io()
        value = {
            "fs.path-exists": meta_io.path_exists,
            "fs.open": meta_io.read_utf8,
            "fs.file-type": meta_io.file_type,
            "fs.read-dir": meta_io.read_dir,
        }[effect](path, grants)
        return _EffectExecuted(
            effect, value, _effect_receipt(effect, capability_class, True)
        )
    except Exception as exc:  # noqa: BLE001 - normalized at the boundary
        return _failed_effect(
            effect, capability_class, getattr(exc, "error_class", "io-error")
        )


def apply_effect_request(
    request: Any, granted: tuple[str, ...] | list[str] = ()
) -> dict[str, Any]:
    """Compatibility projection of the nominal effect adapter result."""
    return _project_effect_adapter_result(
        _apply_effect_request_outcome(request, granted)
    )


def _contains_function_sentinel(x: Any) -> bool:
    """A pnix function nested inside a container collapses to a sentinel str under stable_data;
    detect that so to_host never claims lossless for a value that dropped a function."""
    if isinstance(x, str):
        return x in _FUNCTION_SENTINELS
    if isinstance(x, list):
        return any(_contains_function_sentinel(i) for i in x)
    if isinstance(x, dict):
        return any(_contains_function_sentinel(v) for v in x.values())
    return False


def _contains_path_like(x: Any, depth: int = 8) -> bool:
    """True if a PnixPath (or context-carrying PnixString) hides anywhere in `x`, forcing lazy
    values as it walks (pnix values are WHNF -- nested members are Thunks; by the time this runs
    stable_data has already forced the whole structure, so forcing again is memoized-cheap)."""
    if depth <= 0:
        return False
    try:
        x = rt.force_value(x)
    except Exception:  # noqa: BLE001 - an unforceable member is not a path
        return False
    if isinstance(x, rt.PnixPath):
        return True
    if isinstance(x, rt.PnixString):
        return bool(getattr(x, "context", None))
    if isinstance(x, dict):
        return (any(_contains_path_like(k, depth - 1) for k in x.keys())
                or any(_contains_path_like(v, depth - 1) for v in x.values()))
    if isinstance(x, (list, tuple, set, frozenset)):
        return any(_contains_path_like(v, depth - 1) for v in x)
    return False


def _set_key(x: Any) -> tuple[str, str]:
    """Stable mixed-type sort key for sets with non-orderable members."""
    return (type(x).__name__, repr(x))


_JSON_SAFE_INT = 2**53 - 1  # IEEE-754 double / JSON interop safe-integer bound


def numeric_fits(value: Any, kind: str) -> bool:
    """0015 (I7, GraalVM fitsIn*): PREDICATE a numeric boundary conversion before it happens.
    kind: 'int' (float carries an exact integer), 'float' (int survives a float roundtrip at
    53-bit precision), 'json-number' (finite and within the JSON safe-integer range)."""
    if kind == "int":
        return isinstance(value, float) and value == value and value not in (float("inf"), float("-inf")) \
            and value.is_integer()
    if kind == "float":
        if not isinstance(value, int) or isinstance(value, bool):
            return False
        try:
            return int(float(value)) == value
        except (OverflowError, ValueError):
            return False
    if kind == "json-number":
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            return False
        if isinstance(value, float):
            return value == value and value not in (float("inf"), float("-inf"))
        return -_JSON_SAFE_INT <= value <= _JSON_SAFE_INT
    raise ValueError(f"numeric_fits: unknown kind {kind!r}")


def _numeric_boundary_lossy(value: Any) -> bool:
    """True when a numeric value will not survive the JSON/double interop envelope losslessly
    (marking only -- the crossing VALUE is never changed)."""
    return isinstance(value, (int, float)) and not isinstance(value, bool) \
        and not numeric_fits(value, "json-number")


def _is_opaque_ref_like(value: dict[str, Any]) -> bool:
    if "__hy_meta_opaque__" in value:
        return "witness_id" in value
    if "__pnix_opaque__" in value:
        return all(k in value for k in ("__pnix_opaque__", "kind", "repr", "witness_id"))
    return False

# severity order for combining loss statuses across a multi-step crossing (A1 roundtrip).
_LOSS_SEVERITY = {"lossless": 0, "lossy": 1, "opaque": 2, "effectful": 2, "unsupported": 3, "dangerous": 4}


def _worst_loss(statuses: list[str]) -> str:
    return max(statuses, key=lambda s: _LOSS_SEVERITY.get(s, 3)) if statuses else "lossless"


def _worst_effect(a: str, b: str) -> str:
    """Aggregate two effect classes: any non-`pure` dominates (a nested host-callable makes the
    whole container carry the host-call effect)."""
    if a == b or b == "pure":
        return a
    if a == "pure":
        return b
    return a  # two differing non-pure -> keep the first (rare in nested pure data)


class InteropError(Exception):
    """A host-facing interop error. `wrap_pnix_callable` raises this instead of leaking a raw
    pnix `PnixError` across the boundary into host code (D1). 0020/I4: carries an optional
    `blame` direction ('host' | 'pnix') -- WHICH side of the boundary violated the contract
    (blame calculus, Wadler & Findler). An exception attribute, NOT a schema field."""

    def __init__(self, message: str, *, blame: str | None = None) -> None:
        super().__init__(message)
        self.blame = blame


# D1: an UNAMBIGUOUS cross-boundary error value. The reserved key can never be confused with a
# legitimate pnix attrset that happens to carry an `exception` field.
_INTEROP_ERROR_KEY = "__interop_error__"


def _interop_error(type_name: str, message: str, kind: str = "host-exception",
                   blame: str | None = None) -> dict[str, Any]:
    if blame is None and kind == "host-exception":
        blame = "host"  # I4: a raising host callable is the host side's contract violation
    return {_INTEROP_ERROR_KEY: {"kind": kind, "type": type_name, "message": message, "blame": blame}}


def is_interop_error(result: Any) -> bool:
    """True if `result` is a cross-boundary error value from call_host / call_host_method (D1)."""
    return isinstance(result, dict) and _INTEROP_ERROR_KEY in result


def interop_error_of(result: Any) -> dict[str, Any] | None:
    """The `{kind,type,message}` of a cross-boundary error value, or None."""
    return result.get(_INTEROP_ERROR_KEY) if is_interop_error(result) else None


class CapabilityHandle:
    """0020/I1 (SES-style): a RUNTIME-revocable capability grant. Pass the handle inside
    `granted=(...)` anywhere a capability tuple is accepted; the host can then `attenuate`
    (drop effects), `suspend`/`resume`, or permanently `revoke` it -- taking effect on the
    guest's NEXT boundary crossing. Plain capability strings keep working unchanged."""

    def __init__(self, effects: tuple[str, ...]) -> None:
        self.handle_id = _next_id("cap")
        self._effects: set[str] = set(effects)
        self._suspended = False
        self._revoked = False

    def effective(self) -> set[str]:
        return set() if (self._revoked or self._suspended) else set(self._effects)

    def attenuate(self, *drop: str) -> "CapabilityHandle":
        self._effects -= set(drop)
        return self

    def suspend(self) -> "CapabilityHandle":
        self._suspended = True
        return self

    def resume(self) -> "CapabilityHandle":
        if self._revoked:
            raise InteropError("capability handle revoked; cannot resume", blame="host")
        self._suspended = False
        return self

    def revoke(self) -> "CapabilityHandle":
        self._revoked = True
        self._effects = set()
        return self


def grant_capability(*effects: str) -> CapabilityHandle:
    """0020/I1: grant a revocable capability handle for the given effect classes."""
    return CapabilityHandle(tuple(effects))


def _effective_granted(granted: Any) -> set[str]:
    """Normalize a granted spec: a bare string counts as one capability (A19), plain strings
    pass through, and CapabilityHandles contribute their CURRENT effective effects (I1)."""
    if isinstance(granted, str):
        granted = (granted,)
    out: set[str] = set()
    for g in granted or ():
        if isinstance(g, CapabilityHandle):
            out |= g.effective()
        else:
            out.add(g)
    return out


_id_counter = {"n": 0}


def _next_id(prefix: str = "io") -> str:
    _id_counter["n"] += 1
    return f"{prefix}-{_id_counter['n']}"


@dataclass
class InteropRecord:
    """One conversion across the Hy/Python <-> pnix boundary (IB1)."""
    interop_id: str
    direction: str            # "pnix->host" | "host->pnix"
    source_lang: str
    target_lang: str
    input_kind: str
    output_kind: str
    loss_status: str          # one of LOSS_STATUSES
    effect_class: str         # one of EFFECT_CLASSES
    capability_required: str | None = None
    witness_id: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


# --- IB2: opaque-ref registry (host objects must not enter pnix canonical terms) ---
_OPAQUE: dict[int, Any] = {}
# D2 (lane-local): lifecycle accounting for the pnix fallback registry. The opaque-ref DICT shape
# is NOT changed. `live` is len(_OPAQUE) -- an object's id is PINNED while it is held (strong ref),
# so a live id is never reused, making the count id-reuse-safe. created/released are monotonic
# counters incremented only on a genuine add/pop. Invariant: total == live + released.
_OPAQUE_TOTAL = {"n": 0}
_OPAQUE_RELEASED = {"n": 0}
# 0016 (I2, Canonical-ABI own/borrow): lane-local lend accounting keyed by opaque_ref_id.
# The opaque-ref DICT shape stays untouched (SCOPE_LOCK §6) -- this is sidecar metadata.
_OPAQUE_LEND: dict[int, int] = {}
_LEND_VIOLATIONS = {"n": 0}
# 0020/I3 (GraalVM Context-style): opaque refs made INSIDE an interop_context() are lifecycle-
# bound to it -- closing the context releases them all; later access is a typed error.
_CTX_STACK: list["InteropContext"] = []
_OPAQUE_CTX: dict[int, str] = {}
_CLOSED_CTX: set[str] = set()


class InteropContext:
    def __init__(self) -> None:
        self.context_id = _next_id("ctx")
        self.ref_ids: list[int] = []

    def __enter__(self) -> "InteropContext":
        _CTX_STACK.append(self)
        return self

    def __exit__(self, *_exc: Any) -> bool:
        _CTX_STACK.remove(self)
        _CLOSED_CTX.add(self.context_id)
        for rid in self.ref_ids:  # closing releases every context-bound ref (lends included)
            _OPAQUE_LEND.pop(rid, None)
            if _OPAQUE.pop(rid, _RELEASE_MISS) is not _RELEASE_MISS:
                _OPAQUE_RELEASED["n"] += 1
        return False


def interop_context() -> InteropContext:
    """0020/I3: a lifecycle scope for opaque refs -- everything created inside is released on
    exit, and any later use of those refs raises `InteropError('... context ... closed')`."""
    return InteropContext()


def _ctx_guard(rid: int) -> None:
    ctx_id = _OPAQUE_CTX.get(rid)
    if ctx_id is not None and ctx_id in _CLOSED_CTX:
        raise InteropError(f"opaque ref {rid}: interop context {ctx_id} closed", blame="host")
_RELEASE_MISS = object()
_HOST_INTEROP_MOD: Any = None
_HOST_INTEROP_TRIED = False
_HOST_IMPORT_HOOK_MOD: Any = None
_HOST_IMPORT_HOOK_TRIED = False


def _host_interop() -> Any:
    """Load hy-meta's host-side interop adapter when available."""
    global _HOST_INTEROP_MOD, _HOST_INTEROP_TRIED
    if _HOST_INTEROP_TRIED:
        return _HOST_INTEROP_MOD
    _HOST_INTEROP_TRIED = True
    path = _hy_meta_dir() / "interop.py"
    if not path.exists():
        return None
    added = False
    hy_meta_dir = str(path.parent)
    if hy_meta_dir not in sys.path:
        sys.path.insert(0, hy_meta_dir)
        added = True
    try:
        spec = importlib.util.spec_from_file_location("pnix_hy_host_interop", str(path))
        if spec is None or spec.loader is None:
            return None
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        _HOST_INTEROP_MOD = mod
    except Exception:  # noqa: BLE001 - standalone pnix interop falls back to local refs
        _HOST_INTEROP_MOD = None
    finally:
        if added:
            try:
                sys.path.remove(hy_meta_dir)
            except ValueError:
                pass
    return _HOST_INTEROP_MOD


def _host_import_hook() -> Any:
    """Load hy-meta's pnix import-hook service (SR4) when available."""
    global _HOST_IMPORT_HOOK_MOD, _HOST_IMPORT_HOOK_TRIED
    if _HOST_IMPORT_HOOK_TRIED:
        return _HOST_IMPORT_HOOK_MOD
    _HOST_IMPORT_HOOK_TRIED = True
    path = _hy_meta_dir() / "import_hook.py"
    if not path.exists():
        return None
    try:
        spec = importlib.util.spec_from_file_location("pnix_hy_host_import_hook", str(path))
        if spec is None or spec.loader is None:
            return None
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        _HOST_IMPORT_HOOK_MOD = mod
    except Exception:  # noqa: BLE001 - import hooks are optional outside the split repo
        _HOST_IMPORT_HOOK_MOD = None
    return _HOST_IMPORT_HOOK_MOD


def _pnix_module_loader(*args: Any, **kwargs: Any) -> Any:
    path = kwargs.get("path") or (args[0] if args else None)
    if path is None:
        raise TypeError("pnix module loader requires a path")
    value = rt.stable_data(rt.run_px(str(path)))
    return value


def install_pnix_import_hook(roots: list[str | Path], pnix_loader: Any | None = None) -> Any:
    """Install hy-meta's SR4 import hook with pnix-hy's `.px` runtime semantics.

    The host owns `sys.meta_path` integration; pnix-hy supplies only the `.px` evaluator
    (`pnix_runtime.run_px`). Returns hy-meta's context manager.
    """
    host = _host_import_hook()
    if host is None:
        raise RuntimeError("hy-meta import_hook.py unavailable")
    return host.install_pnix_import_hook(pnix_loader or _pnix_module_loader, roots)


def _local_witness(kind: str, payload: Any) -> dict[str, Any]:
    from . import gate as _gate  # noqa: PLC0415 - avoid module-load cycle
    return _gate.make_witness(kind, payload)


def make_opaque_ref(obj: Any, kind: str = "host-object", *, prefer_host: bool = True) -> dict[str, Any]:
    if prefer_host:
        host = _host_interop()
        if host is not None:
            try:
                return host.make_opaque_ref(obj, kind)
            except Exception:  # noqa: BLE001 - keep local fallback for standalone use
                pass
    rid = id(obj)
    if rid not in _OPAQUE:  # D2: count only a genuinely new live ref (id pinned while held)
        _OPAQUE_TOTAL["n"] += 1
    _OPAQUE[rid] = obj  # keep alive + resolvable
    if _CTX_STACK:  # I3: bind to the innermost open interop context
        ctx = _CTX_STACK[-1]
        _OPAQUE_CTX[rid] = ctx.context_id
        ctx.ref_ids.append(rid)
    witness = _local_witness("make-opaque-ref", {
        "kind": kind,
        "object_type": type(obj).__name__,
        "repr": repr(obj)[:120],
    })
    return {"__pnix_opaque__": rid, "kind": kind, "repr": repr(obj)[:120],
            "witness_id": witness["witness_id"]}


def is_opaque_ref(value: Any) -> bool:
    return isinstance(value, dict) and _is_opaque_ref_like(value)


def resolve_opaque(ref: dict[str, Any]) -> Any:
    if not is_opaque_ref(ref):
        raise KeyError("not an opaque ref")
    if "__hy_meta_opaque__" in ref:
        host = _host_interop()
        if host is None:
            raise KeyError("hy-meta host interop unavailable")
        return host.resolve_opaque(ref)
    rid = int(ref["__pnix_opaque__"])
    _ctx_guard(rid)
    if rid not in _OPAQUE:
        raise InteropError(f"opaque ref {rid}: released or unknown", blame="host")
    return _OPAQUE[rid]


def opaque_ref_id(ref: dict[str, Any]) -> int:
    if not is_opaque_ref(ref):
        raise KeyError("not an opaque ref")
    return int(ref.get("__hy_meta_opaque__", ref.get("__pnix_opaque__")))


def inspect_opaque(ref_or_obj: Any) -> dict[str, Any]:
    """Inspect an opaque host object without exposing the object itself."""
    if is_opaque_ref(ref_or_obj) and "__hy_meta_opaque__" in ref_or_obj:
        host = _host_interop()
        if host is not None and hasattr(host, "inspect_object"):
            return host.inspect_object(ref_or_obj)
    obj = resolve_opaque(ref_or_obj) if is_opaque_ref(ref_or_obj) else ref_or_obj
    info = {
        "type": type(obj).__name__,
        "module": getattr(type(obj), "__module__", ""),
        "callable": callable(obj),
        "repr": repr(obj)[:120],
    }
    witness = _local_witness("inspect-object", info)
    return {**info, "witness_id": witness["witness_id"]}


def opaque_allowed_methods(ref_or_obj: Any) -> list[str]:
    """List public callable methods for explicit method-level interop."""
    if is_opaque_ref(ref_or_obj) and "__hy_meta_opaque__" in ref_or_obj:
        host = _host_interop()
        if host is not None and hasattr(host, "opaque_allowed_methods"):
            return host.opaque_allowed_methods(ref_or_obj)
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


def lend_opaque(ref: dict[str, Any]):
    """0016 (I2): a call-scoped BORROW of an opaque ref (Canonical-ABI own/borrow). While at
    least one lend is active, the OWNER cannot release the ref -- `release_opaque` raises a
    typed `InteropError` instead of yanking an object out from under a borrower. Usage:

        with lend_opaque(ref):
            ... pass ref across the boundary ...
    """
    from contextlib import contextmanager  # noqa: PLC0415

    if not is_opaque_ref(ref):
        raise InteropError("lend_opaque: not an opaque ref")
    rid = opaque_ref_id(ref)

    @contextmanager
    def _lease():
        _OPAQUE_LEND[rid] = _OPAQUE_LEND.get(rid, 0) + 1
        try:
            yield ref
        finally:
            n = _OPAQUE_LEND.get(rid, 0) - 1
            if n <= 0:
                _OPAQUE_LEND.pop(rid, None)
            else:
                _OPAQUE_LEND[rid] = n

    return _lease()


def release_opaque(ref: dict[str, Any]) -> None:
    if is_opaque_ref(ref):
        rid = opaque_ref_id(ref)
        lends = _OPAQUE_LEND.get(rid, 0)
        if lends > 0:  # 0016: canon_resource_drop traps while num_lends > 0
            _LEND_VIOLATIONS["n"] += 1
            raise InteropError(f"release_opaque: ref {rid} released while lent (num_lends={lends})",
                               blame="host")
    if is_opaque_ref(ref) and "__hy_meta_opaque__" in ref:
        host = _host_interop()
        if host is not None:
            host.release(ref)
            return
    if is_opaque_ref(ref) and "__pnix_opaque__" in ref:
        if _OPAQUE.pop(ref["__pnix_opaque__"], _RELEASE_MISS) is not _RELEASE_MISS:
            _OPAQUE_RELEASED["n"] += 1  # D2: count only a real release (no double-count)


def opaque_lifecycle() -> dict[str, Any]:
    """D2: a snapshot of the pnix fallback opaque-ref lifecycle. `live` = objects currently held
    (len of the registry; id-reuse-safe since a held id is pinned); `released`/`total` are
    monotonic counters. Invariant: total == live + released. A leak is a ref created but never
    released (live grows without a matching release)."""
    return {"schema": "pnix-hy.interop.opaque-lifecycle.v0",
            "live": len(_OPAQUE), "released": _OPAQUE_RELEASED["n"], "total": _OPAQUE_TOTAL["n"],
            "lends_active": sum(_OPAQUE_LEND.values()), "lend_violations": _LEND_VIOLATIONS["n"]}


def _pnix_kind(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        return "int"
    if isinstance(value, float):
        return "float"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "list"
    if isinstance(value, dict):
        return "attrset"
    return "opaque"


def _is_pnix_function(raw: Any) -> bool:
    try:
        forced = rt.force_value(raw)
    except Exception:  # noqa: BLE001
        forced = raw
    return isinstance(forced, (rt.Closure, rt.NativeFunc))


def to_host(raw: Any, *, witness_id: str | None = None) -> tuple[Any, InteropRecord]:
    """pnix runtime value -> Python host value, with an InteropRecord. Data is lossless;
    pnix functions become opaque refs (effect=host-call)."""
    if _is_pnix_function(raw):
        ref = make_opaque_ref(raw, "pnix-function")
        witness = witness_id or ref.get("witness_id")
        return ref, InteropRecord(_next_id(), "pnix->host", "pnix", "python",
                                  "pnix-function", "opaque-ref", "opaque", "host-call",
                                  capability_required="host-call", witness_id=witness)
    try:
        data = rt.stable_data(raw)
    except Exception:  # noqa: BLE001 - unforceable/non-data -> opaque
        ref = make_opaque_ref(raw, "pnix-opaque")
        witness = witness_id or ref.get("witness_id")
        return ref, InteropRecord(_next_id(), "pnix->host", "pnix", "python",
                                  "unknown", "opaque-ref", "opaque", "unknown", witness_id=witness)
    # A11: only a GENUINE function may take the opaque path -- a real pnix string whose value
    # happens to equal a sentinel crosses as that string (the top-level function case was already
    # caught by _is_pnix_function above, so this branch is a belt-and-braces guard).
    if isinstance(data, str) and data in _FUNCTION_SENTINELS and _is_pnix_function(raw):
        ref = make_opaque_ref(raw, "pnix-function")
        witness = witness_id or ref.get("witness_id")
        return ref, InteropRecord(_next_id(), "pnix->host", "pnix", "python",
                                  "pnix-function", "opaque-ref", "opaque", "host-call",
                                  capability_required="host-call", witness_id=witness)
    # A6: stable_data collapses PnixPath/PnixString to a plain str, losing path / string-context
    # provenance. Inspect the forced raw value BEFORE that and mark it (realize_value untouched).
    # 0015 (I7): numbers leaving the JSON/double safe envelope are marked lossy (value unchanged).
    output_kind, effect = type(data).__name__, "pure"
    loss = "lossy" if _numeric_boundary_lossy(data) else "lossless"
    try:
        forced = rt.force_value(raw)
    except Exception:  # noqa: BLE001
        forced = None
    if isinstance(forced, rt.PnixPath):
        output_kind, loss = "path", "lossy"
    elif isinstance(forced, rt.PnixString) and getattr(forced, "context", None):
        output_kind, loss = "string-context", "lossy"
    elif _contains_path_like(forced):
        loss = "lossy"
    elif not isinstance(data, str) and _contains_function_sentinel(data):
        # a pnix function nested in a CONTAINER was dropped to a sentinel; a top-level genuine
        # string equal to a sentinel is real data (A11) and stays lossless.
        loss, effect = "lossy", "host-call"
    return data, InteropRecord(_next_id(), "pnix->host", "pnix", "python",
                               _pnix_kind(data), output_kind, loss, effect,
                               witness_id=witness_id)


def from_host(value: Any, *, witness_id: str | None = None) -> tuple[Any, InteropRecord]:
    """Python host value -> pnix-usable representation, with an InteropRecord. Pure data
    crosses losslessly; callables/modules/arbitrary objects become opaque refs."""
    if value is None or isinstance(value, (bool, int, float, str)):
        # 0015 (I7): a number outside the JSON/double safe envelope is marked lossy up front
        # (value unchanged -- fitsIn*-style predicate, GraalVM Value API).
        loss = "lossy" if _numeric_boundary_lossy(value) else "lossless"
        return value, InteropRecord(_next_id(), "host->pnix", "python", "pnix",
                                    type(value).__name__, _pnix_kind(value), loss, "pure",
                                    witness_id=witness_id)
    if isinstance(value, (bytes, bytearray)):
        # A4: bytes have no pnix representation; project to a reversible int-octet list, marked
        # lossy (the bytes/text distinction is not preserved).
        return [int(b) for b in value], InteropRecord(
            _next_id(), "host->pnix", "python", "pnix", type(value).__name__, "list",
            "lossy", "pure", witness_id=witness_id)
    if isinstance(value, (set, frozenset)):
        # A5: set/frozenset -> pnix list (sorted when orderable); lossy (set-ness and any
        # ordering are not preserved).
        ordered = sorted(value, key=_set_key)
        items: list[Any] = []
        effect, cap = "pure", None  # aggregate nested effect/capability
        for v in ordered:
            pv, r = from_host(v)
            items.append(pv)
            effect = _worst_effect(effect, r.effect_class)
            cap = cap or r.capability_required
        return items, InteropRecord(
            _next_id(), "host->pnix", "python", "pnix", type(value).__name__, "list",
            "lossy", effect, capability_required=cap, witness_id=witness_id)
    if isinstance(value, (list, tuple)):
        items: list[Any] = []
        lossless = True
        effect, cap = "pure", None  # aggregate nested effect/capability
        for v in value:
            pv, r = from_host(v)
            items.append(pv)
            lossless = lossless and r.loss_status == "lossless"
            effect = _worst_effect(effect, r.effect_class)
            cap = cap or r.capability_required
        # A2: a tuple collapses to a pnix list (tuple-ness lost) -> at least lossy.
        loss = "lossless" if (lossless and not isinstance(value, tuple)) else "lossy"
        return items, InteropRecord(_next_id(), "host->pnix", "python", "pnix",
                                    type(value).__name__, "list", loss,
                                    effect, capability_required=cap, witness_id=witness_id)
    if isinstance(value, dict):
        out: dict[str, Any] = {}
        lossless = True
        nonstr_key = False
        effect, cap = "pure", None  # aggregate nested effect/capability
        reserved_key = False
        for k, v in value.items():
            pv, r = from_host(v)
            if not isinstance(k, str):
                nonstr_key = True  # A3: a non-str key must be str()'d into a pnix attr name
            if isinstance(k, str) and k in _OPAQUE_REFS:
                reserved_key = True
            out[str(k)] = pv
            lossless = lossless and r.loss_status == "lossless"
            effect = _worst_effect(effect, r.effect_class)
            cap = cap or r.capability_required
        collision = len(out) < len(value)  # A3: distinct keys collapsed to one attr name
        loss = "lossless" if (lossless and not nonstr_key and not collision and not reserved_key) else "lossy"
        if reserved_key:
            cap = cap or "opaque"
        return out, InteropRecord(_next_id(), "host->pnix", "python", "pnix",
                                  "dict", "attrset", loss, effect, capability_required=cap,
                                  witness_id=witness_id)
    # callable / module / arbitrary host object -> opaque ref
    callable_ = callable(value)
    kind = "host-callable" if callable_ else "host-object"
    effect = "host-call" if callable_ else "unknown"
    ref = make_opaque_ref(value, kind)
    witness = witness_id or ref.get("witness_id")
    return ref, InteropRecord(
        _next_id(), "host->pnix", "python", "pnix", type(value).__name__, "opaque-ref",
        "opaque", effect, capability_required=("host-call" if callable_ else None),
        witness_id=witness)


def check_capability(record: InteropRecord, granted: tuple[str, ...] | list[str]) -> bool:
    """Effect/capability gate: a conversion requiring a capability is allowed only when
    that capability is granted. Records with no requirement are always allowed."""
    cap = record.capability_required
    return cap is None or cap in _effective_granted(granted)


def to_host_eval(source: str, *, witness_id: str | None = None) -> tuple[Any, InteropRecord]:
    """Evaluate a pnix source fragment, then convert the result to the host."""
    return to_host(rt.eval_source(source), witness_id=witness_id)


def roundtrip_host_value(value: Any, *, witness_id: str | None = None) -> dict[str, Any]:
    """A1: cross a host value host->pnix->host and report fidelity in ONE place.

    Pure data crosses back and is compared for equality; opaque values (callables/modules/
    arbitrary objects) round-trip by reference via `resolve_opaque`. The combined `loss_status`
    is the worst of the two crossings, forced to `lossy` when the round-tripped value differs
    (this is what surfaces tuple->list, set->list, bytes->octets, non-str-key and path losses).
    Read-only apart from registering the opaque ref; safe with the mirror OFF.
    """
    pv, in_rec = from_host(value, witness_id=witness_id)
    is_opaque_from_host = in_rec.output_kind == "opaque-ref"
    if is_opaque_from_host:
        try:
            back = resolve_opaque(pv)
            equal = back is value or back == value
        except Exception:  # noqa: BLE001
            equal = False
        return {"schema": "pnix-hy.interop.roundtrip.v0", "input_kind": in_rec.input_kind,
                "roundtrip": "by-ref", "from_host_loss": in_rec.loss_status,
                "to_host_loss": "opaque", "loss_status": in_rec.loss_status,
                "equal": bool(equal), "opaque": True, "witness_id": in_rec.witness_id}
    try:
        hv, out_rec = to_host(pv, witness_id=in_rec.witness_id)
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop.roundtrip.v0", "input_kind": in_rec.input_kind,
                "roundtrip": "by-value", "from_host_loss": in_rec.loss_status,
                "to_host_loss": "unsupported", "loss_status": "unsupported", "equal": False,
                "opaque": False, "error": f"{type(exc).__name__}: {exc}",
                "witness_id": in_rec.witness_id}
    equal = hv == value
    combined = _worst_loss([in_rec.loss_status, out_rec.loss_status,
                            "lossless" if equal else "lossy"])
    return {"schema": "pnix-hy.interop.roundtrip.v0", "input_kind": in_rec.input_kind,
            "roundtrip": "by-value", "from_host_loss": in_rec.loss_status,
            "to_host_loss": out_rec.loss_status, "loss_status": combined,
            "equal": bool(equal), "opaque": False, "value_out": hv,
            "witness_id": in_rec.witness_id}


# --- B3: host-callable signature projection (mirrors rt.function_args_value) ---

def host_callable_arity(host_callable: Any) -> dict[str, Any]:
    """B3: project a host callable's signature into the pnix `functionArgs` shape
    (`{param: has_default}`), with `*name`/`**name` markers for var-positional/var-keyword.
    Returns {} when no signature is available (e.g. some C builtins)."""
    fn = resolve_opaque(host_callable) if is_opaque_ref(host_callable) else host_callable
    try:
        sig = inspect.signature(fn)
    except (TypeError, ValueError):
        return {}
    out: dict[str, Any] = {}
    for name, p in sig.parameters.items():
        if p.kind == inspect.Parameter.VAR_POSITIONAL:
            out["*" + name] = False
        elif p.kind == inspect.Parameter.VAR_KEYWORD:
            out["**" + name] = False
        else:
            out[name] = p.default is not inspect.Parameter.empty
    return out


def _required_positional_count(fn: Any) -> int | None:
    """Count of required positional params, for currying. None = unknown/variadic -> unary."""
    try:
        sig = inspect.signature(fn)
    except (TypeError, ValueError):
        return None
    n = 0
    for p in sig.parameters.values():
        if p.kind in (inspect.Parameter.VAR_POSITIONAL, inspect.Parameter.VAR_KEYWORD):
            continue
        if p.kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD):
            if p.default is inspect.Parameter.empty:
                n += 1
    return n  # 0 = genuinely nullary (distinct from None = unknown/variadic)


def host_callable_to_pnix(host_callable: Any, *, name: str | None = None,
                          arity: int | None = None,
                          granted: tuple[str, ...] | list[str] = ("host-call",)) -> Any:
    """B1: wrap a HOST callable as a pnix `NativeFunc` so pnix SOURCE can apply it as a builtin.

    Place the result into `ctx["env"]` (merged by `eval_source_raw`) and pnix code can call it
    by name. Each pnix arg is converted host-ward via `to_host`, the callable is invoked, and
    the result is converted back via `from_host`. Multi-arg callables are curried by required-
    positional arity (override with `arity`). Reaching a host function from pnix is a genuine
    `host-call` effect: when `host-call` is not granted the wrapper denies at apply time."""
    fn = resolve_opaque(host_callable) if is_opaque_ref(host_callable) else host_callable
    allow = "host-call" in _effective_granted(granted)
    n = arity if arity is not None else _required_positional_count(fn)

    if n == 0:
        # pnix has no zero-arg application; accept (and ignore) one pnix arg, call fn() eagerly
        def _nullary(_ignored: Any) -> Any:
            if not allow:
                rt.pnix_error("host-call capability not granted for host callable")
            return from_host(fn())[0]
        return rt.NativeFunc(_nullary, force_arg=True)

    def _make(collected: list[Any]) -> Any:
        def _step(pnix_arg: Any) -> Any:
            if not allow:
                rt.pnix_error("host-call capability not granted for host callable")
            host_arg, _ = to_host(pnix_arg)
            new = collected + [host_arg]
            if n is not None and len(new) < n:
                return rt.NativeFunc(_make(new), force_arg=True)
            out, _ = from_host(fn(*new))
            return out
        return _step

    return rt.NativeFunc(_make([]), force_arg=True)


# --- IB3: callable bridges (both directions) ---

def call_host(host_callable: Any, args: tuple[Any, ...] = (), *,
              kwargs: dict[str, Any] | None = None,
              granted: tuple[str, ...] | list[str] = ("host-call",),
              witness_id: str | None = None) -> tuple[Any, InteropRecord]:
    """Invoke a HOST callable from the pnix side, capability-gated. `host_callable` may be a
    raw callable or an opaque ref; `args` are host values. Captures exceptions; the result
    is mapped back via from_host. Returns (result_or_status, InteropRecord)."""
    rec = InteropRecord(_next_id(), "pnix->host", "pnix", "python", "host-call", "value",
                        "effectful", "host-call", capability_required="host-call", witness_id=witness_id)
    if "host-call" not in _effective_granted(granted):
        rec.loss_status = "unsupported"
        return {"denied": "host-call capability not granted"}, rec
    fn = resolve_opaque(host_callable) if is_opaque_ref(host_callable) else host_callable
    host = _host_interop()
    if host is not None:
        called = host.call_opaque(fn, args, kwargs)  # B2: kwargs threaded to the host adapter
        rec.witness_id = rec.witness_id or called.get("witness_id")
        if not called.get("ok"):
            rec.output_kind = "exception"
            error = called.get("exception") or {}
            return _interop_error(error.get("type", "Exception"), error.get("message", "")), rec  # D1
        out, out_rec = from_host(called.get("value"), witness_id=called.get("witness_id"))
        rec.output_kind = out_rec.output_kind
        return out, rec
    try:
        result = fn(*args, **dict(kwargs or {}))  # B2: kwargs threaded to the local call
    except Exception as exc:  # noqa: BLE001 - host call failed; surface, don't leak
        rec.output_kind = "exception"
        return _interop_error(type(exc).__name__, str(exc)), rec  # D1: unambiguous error value
    out, out_rec = from_host(result)
    rec.output_kind = out_rec.output_kind
    rec.witness_id = rec.witness_id or _local_witness(  # B4: every crossing carries a witness
        "call-host", {"input_kind": "host-call", "output_kind": out_rec.output_kind,
                      "effect_class": "host-call"})["witness_id"]
    return out, rec


def call_host_method(ref_or_obj: Any, method_name: str, args: tuple[Any, ...] = (), *,
                     kwargs: dict[str, Any] | None = None,
                     granted: tuple[str, ...] | list[str] = ("host-call",),
                     allowed_methods: tuple[str, ...] | list[str] | None = None,
                     witness_id: str | None = None) -> tuple[Any, InteropRecord]:
    """Invoke a host object's public method across the interop boundary.

    This is the method-level counterpart to `call_host`: the call is capability-gated, host
    exceptions are captured, and returned values are mapped back through `from_host`.
    """
    rec = InteropRecord(_next_id(), "pnix->host", "pnix", "python", "host-method", "value",
                        "effectful", "host-call", capability_required="host-call",
                        witness_id=witness_id)
    if "host-call" not in _effective_granted(granted):
        rec.loss_status = "unsupported"
        return {"denied": "host-call capability not granted"}, rec
    host = _host_interop()
    if is_opaque_ref(ref_or_obj) and "__hy_meta_opaque__" in ref_or_obj and host is not None:
        if hasattr(host, "call_method"):
            called = host.call_method(ref_or_obj, method_name, args, kwargs, allowed_methods)
        else:
            obj = host.resolve_opaque(ref_or_obj)
            called = host.call_opaque(getattr(obj, method_name), args, kwargs)
        rec.witness_id = rec.witness_id or called.get("witness_id")
        if not called.get("ok"):
            if called.get("exception"):  # D1: unambiguous error; denial keeps its shape
                rec.output_kind = "exception"
                err = called.get("exception") or {}
                return _interop_error(err.get("type", "Exception"), err.get("message", "")), rec
            rec.output_kind = "denied"
            return called, rec
        out, out_rec = from_host(called.get("value"), witness_id=called.get("witness_id"))
        rec.output_kind = out_rec.output_kind
        return out, rec

    obj = resolve_opaque(ref_or_obj) if is_opaque_ref(ref_or_obj) else ref_or_obj
    if is_opaque_ref(ref_or_obj):  # 0020: substrate-enforced guards at the SINGLE entrypoint
        try:
            _surface_guard(ref_or_obj, obj)      # I5: hardened surface unchanged
            _invariant_guard(ref_or_obj, obj)    # I6: declared frozen attrs unchanged
        except InteropError as exc:  # D1: guests get an unambiguous error VALUE, not a leak
            rec.output_kind = "exception"
            return _interop_error("InteropError", str(exc), blame=exc.blame), rec
    allowed = set(allowed_methods) if allowed_methods is not None else set(opaque_allowed_methods(obj))
    if method_name.startswith("_") or method_name not in allowed:
        rec.loss_status = "unsupported"
        witness = _local_witness("host-method-denied", {"method": method_name})
        rec.witness_id = rec.witness_id or witness["witness_id"]
        return {"denied": "method not allowed"}, rec
    try:
        method = getattr(obj, method_name)
        if not callable(method):
            raise TypeError(f"{method_name} is not callable")
        value = method(*tuple(args), **dict(kwargs or {}))
    except Exception as exc:  # noqa: BLE001
        rec.output_kind = "exception"
        witness = _local_witness("host-method-exception", {"method": method_name, "exception": str(exc)})
        rec.witness_id = rec.witness_id or witness["witness_id"]
        return _interop_error(type(exc).__name__, str(exc)), rec  # D1: unambiguous error value
    out, out_rec = from_host(value)
    rec.output_kind = out_rec.output_kind
    rec.witness_id = rec.witness_id or out_rec.witness_id or _local_witness(  # B4
        "call-host-method", {"method": method_name, "output_kind": out_rec.output_kind,
                             "effect_class": "host-call"})["witness_id"]
    return out, rec


apply_host_method = call_host_method
opaque_call_method = call_host_method


def wrap_pnix_callable(closure_raw: Any, ctx: dict[str, Any] | None = None):
    """Wrap a pnix function (raw, unrealized Closure) as a Python callable so HOST code can
    call it. Each host arg is from_host'd into pnix, applied (curried for multiple args),
    and the result is to_host'd back."""
    ctx = ctx or rt.runtime_context(None)
    try:
        fn = rt.force_value(closure_raw)
    except rt.PnixError as exc:  # D1: don't leak force-time pnix errors into host callers
        raise InteropError(str(exc), blame="pnix") from exc
    except Exception as exc:  # noqa: BLE001
        raise InteropError(f"{type(exc).__name__}: {exc}", blame="pnix") from exc

    def _call(*args: Any) -> Any:
        cur = fn
        try:
            for a in args:
                pv, _ = from_host(a)
                cur = rt.force_value(rt.apply_pnix(cur, rt.Thunk(lambda v=pv: v), ctx))
            host_out, _ = to_host(cur)
            return host_out
        except rt.PnixError as exc:  # D1: don't leak a raw pnix error to host callers
            raise InteropError(str(exc), blame="pnix") from exc

    return _call


def try_call_host(host_callable: Any, args: tuple[Any, ...] = (), *,
                  kwargs: dict[str, Any] | None = None,
                  granted: tuple[str, ...] | list[str] = ("host-call",),
                  witness_id: str | None = None) -> dict[str, Any]:
    """D1: a `tryEval`-shaped wrapper over `call_host` -- returns `{"success": True, "value": v}`
    on success, or `{"success": False, "error": {...}}` on a host exception or capability denial.
    Never collides with a plain attrset."""
    result, rec = call_host(host_callable, args, kwargs=kwargs, granted=granted, witness_id=witness_id)
    if is_interop_error(result) and rec.output_kind == "exception":  # guard: a legit result
        return {"success": False, "error": interop_error_of(result), "record": rec.to_dict()}
    if isinstance(result, dict) and "denied" in result and rec.loss_status == "unsupported":
        return {"success": False, "error": {"kind": "denied", "message": result["denied"]},
                "record": rec.to_dict()}
    return {"success": True, "value": result, "record": rec.to_dict()}


def pnix_callable(source: str, ctx: dict[str, Any] | None = None):
    """Compile a pnix lambda source into a host-callable wrapper (IB3)."""
    ctx = ctx or rt.runtime_context(None)
    try:  # D1: eval of the source itself (e.g. an eager `throw`) must not leak a raw PnixError
        raw = rt.eval_source_raw(source, ctx, realize=False)
    except rt.PnixError as exc:
        raise InteropError(str(exc), blame="pnix") from exc
    except InteropError:
        raise
    except Exception as exc:  # noqa: BLE001
        raise InteropError(f"{type(exc).__name__}: {exc}", blame="pnix") from exc
    return wrap_pnix_callable(raw, ctx)


# --- IB4: module bridges ---

def host_module_to_pnix(module: Any, *, wrap_callables: bool = False,
                        granted: tuple[str, ...] | list[str] = ("host-call",)) -> dict[str, Any]:
    """Map a host module/namespace to a pnix attrset: public attributes become pure pnix
    values, or opaque refs for callables/objects (host objects never enter pnix terms).

    B5: with `wrap_callables=True`, callable public attrs instead become applicable pnix
    NativeFuncs (via `host_callable_to_pnix`), so pnix source can call module members directly."""
    out: dict[str, Any] = {}
    for name in dir(module):
        if name.startswith("_"):
            continue
        try:
            attr = getattr(module, name)
        except Exception:  # noqa: BLE001 - skip un-fetchable attrs
            continue
        if wrap_callables and callable(attr):
            out[name] = host_callable_to_pnix(attr, name=name, granted=granted)
            continue
        try:
            pv, _ = from_host(attr)
            out[name] = pv
        except Exception:  # noqa: BLE001
            continue
    return out


def pnix_module_to_host(attrset_raw: Any, ctx: dict[str, Any] | None = None) -> dict[str, Any]:
    """Map a pnix attrset (of values/functions) to a host dict: functions become callable
    wrappers, data becomes host values."""
    ctx = ctx or rt.runtime_context(None)
    forced = rt.force_value(attrset_raw)
    out: dict[str, Any] = {}
    if not isinstance(forced, dict):
        return out
    for key in forced:
        raw = forced[key]
        val = rt.force_value(raw)
        if isinstance(val, (rt.Closure, rt.NativeFunc)):
            out[str(key)] = wrap_pnix_callable(val, ctx)
        else:
            hv, _ = to_host(val)
            out[str(key)] = hv
    return out


def pnix_import_hook_report() -> dict[str, Any]:
    """Self-check: hy-meta's SR4 import hook imports a `.px` file through pnix_runtime.run_px."""
    try:
        host = _host_import_hook()
        if host is None:
            return {"schema": "pnix-hy.pnix-import-hook.report.v0", "ready": False,
                    "available": False, "error": "hy-meta import_hook.py unavailable"}
        with tempfile.TemporaryDirectory(prefix="pnix-hy-import-hook-") as temp_dir:
            root = Path(temp_dir)
            (root / "pnix_hook_probe.px").write_text('{ answer = 42; label = "ok"; }\n', encoding="utf-8")
            snapshot = host.snapshot_sys_modules(["pnix_hook_probe"])
            with install_pnix_import_hook([root]) as finder:
                hook_installed = finder in sys.meta_path
                module = importlib.import_module("pnix_hook_probe")
            hook_removed = finder not in sys.meta_path
            answer = getattr(module, "answer", None)
            label = getattr(module, "label", None)
            host.rollback_sys_modules(snapshot)
            module_removed = "pnix_hook_probe" not in sys.modules
        ready = bool(hook_installed and hook_removed and module_removed and answer == 42 and label == "ok")
        return {"schema": "pnix-hy.pnix-import-hook.report.v0", "ready": ready,
                "available": True, "answer": answer, "label": label,
                "hook_installed": hook_installed, "hook_removed": hook_removed,
                "module_removed": module_removed}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.pnix-import-hook.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def interop_report() -> dict[str, Any]:
    """Self-check: data crosses lossless both ways; pnix functions and host callables
    become opaque refs with a host-call capability that the gate enforces."""
    try:
        # host -> pnix, pure data lossless
        data_ok = True
        for v in [None, True, 7, 3.5, "hi", [1, 2, [3]], {"a": 1, "b": [2, 3]}]:
            pv, r = from_host(v)
            data_ok = data_ok and pv == v and r.loss_status == "lossless" and r.effect_class == "pure"
        # pnix value -> host: attrset lossless, function opaque
        attr_v, attr_r = to_host(rt.eval_source("{ a = 1; b = [2 3]; }"))
        attr_ok = attr_v == {"a": 1, "b": [2, 3]} and attr_r.loss_status == "lossless"
        # a RAW (unrealized) pnix function -> opaque ref wrapping the live Closure. (A realized
        # function has already collapsed to a sentinel STRING and is indistinguishable from real
        # string data -- see the sentinel_string case below.)
        fn_raw = rt.eval_source_raw("x: x + 1", rt.runtime_context(None), realize=False)
        fn_v, fn_r = to_host(fn_raw)
        fn_ok = is_opaque_ref(fn_v) and fn_r.loss_status == "opaque" and fn_r.effect_class == "host-call"
        fn_witness_ok = bool(fn_r.witness_id)
        # A11 regression pin: a GENUINE string equal to a function sentinel crosses as that string
        s_v, s_r = to_host(rt.eval_source('"#<pnix-hy-closure>"'))
        sentinel_str_ok = s_v == "#<pnix-hy-closure>" and s_r.loss_status == "lossless"
        # host callable -> opaque ref + capability gate
        cb_v, cb_r = from_host(len)
        cap_ok = (is_opaque_ref(cb_v) and cb_r.capability_required == "host-call"
                  and check_capability(cb_r, []) is False
                  and check_capability(cb_r, ["host-call"]) is True)
        # opaque ref resolves back to the live object
        resolve_ok = resolve_opaque(cb_v) is len
        ref_id_ok = opaque_ref_id(cb_v) == cb_v.get("__hy_meta_opaque__", cb_v.get("__pnix_opaque__"))
        inspect_ok = inspect_opaque(cb_v).get("callable") is True
        text_ref = make_opaque_ref("abc", "host-object")
        methods = opaque_allowed_methods(text_ref)
        method_value, method_rec = call_host_method(text_ref, "upper")
        method_denied, denied_rec = call_host_method(text_ref, "upper", granted=())
        method_ok = (
            "upper" in methods
            and method_value == "ABC"
            and method_rec.effect_class == "host-call"
            and "denied" in method_denied
            and denied_rec.loss_status == "unsupported"
        )
        # IB3: pnix function callable from host (curried), host callable from pnix (gated)
        inc = pnix_callable("x: x + 1")
        add = pnix_callable("a: b: a + b")
        ib3_pnix = inc(41) == 42 and add(3, 4) == 7
        called, call_rec = call_host(len, ([1, 2, 3],))
        denied, _ = call_host(len, ([1, 2, 3],), granted=())
        ib3_host = called == 3 and call_rec.effect_class == "host-call" and "denied" in denied
        # IB4: pnix attrset of functions -> host dict of callables
        mod = pnix_module_to_host(rt.eval_source_raw("{ inc = x: x + 1; k = 7; }",
                                                     rt.runtime_context(None), realize=False))
        ib4 = callable(mod.get("inc")) and mod["inc"](10) == 11 and mod.get("k") == 7
        ready = bool(
            data_ok and attr_ok and fn_ok and fn_witness_ok and sentinel_str_ok and cap_ok
            and resolve_ok and ref_id_ok and inspect_ok and method_ok
            and ib3_pnix and ib3_host and ib4
        )
        return {"schema": "pnix-hy.interop.report.v0", "ready": ready, "available": True,
                "data_lossless": data_ok, "attrset_lossless": attr_ok,
                "sentinel_string_lossless": sentinel_str_ok,
                "function_opaque": fn_ok, "function_witness": fn_witness_ok,
                "host_adapter": _host_interop() is not None,
                "capability_gate": cap_ok, "opaque_resolves": resolve_ok,
                "opaque_ref_id": ref_id_ok, "inspect_opaque": inspect_ok,
                "method_call": method_ok,
                "ib3_pnix_callable": ib3_pnix, "ib3_call_host": ib3_host, "ib4_module": ib4}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def roundtrip_report() -> dict[str, Any]:
    """Self-check for A1-A6 (proposal 0001): lossless data round-trips equal; tuple/set/bytes/
    non-str-key/collision/path crossings are marked lossy -- fidelity is never silently claimed
    lossless."""
    try:
        lossless_ok = True
        for v in [None, True, 7, 3.5, "hi", [1, 2, [3]], {"a": 1, "b": [2, 3]}]:
            rr = roundtrip_host_value(v)
            lossless_ok = lossless_ok and rr["loss_status"] == "lossless" and rr["equal"] is True
        tup = roundtrip_host_value((1, 2, 3))
        tuple_ok = tup["from_host_loss"] == "lossy" and tup["loss_status"] == "lossy" and tup["equal"] is False
        st = roundtrip_host_value({1, 2, 3})
        set_pv, _ = from_host({1, 2, 3})
        set_ok = st["from_host_loss"] == "lossy" and set_pv == [1, 2, 3]
        by = roundtrip_host_value(b"abc")
        bytes_pv, _ = from_host(b"abc")
        bytes_ok = by["from_host_loss"] == "lossy" and bytes_pv == [97, 98, 99]
        _, nk_rec = from_host({1: "a"})
        nonstr_ok = nk_rec.loss_status == "lossy"
        coll_pv, coll_rec = from_host({1: "a", "1": "b"})
        collision_ok = coll_rec.loss_status == "lossy" and len(coll_pv) == 1
        praw = rt.eval_source_raw("./foo/bar", rt.runtime_context(None), realize=False)
        _, prec = to_host(praw)
        path_ok = prec.output_kind == "path" and prec.loss_status == "lossy"
        # A12 regression pin: a path NESTED in an attrset is still marked lossy
        nraw = rt.eval_source_raw("{ p = ./foo; }", rt.runtime_context(None), realize=False)
        _, nrec = to_host(nraw)
        nested_path_ok = nrec.loss_status == "lossy"
        cb = roundtrip_host_value(len)  # opaque round-trips by reference
        opaque_ok = cb["opaque"] is True and cb["equal"] is True and cb["loss_status"] == "opaque"
        # 0015 (I7): fitsIn*-style numeric predicates + boundary marking (values unchanged).
        big = roundtrip_host_value(2**53 + 1)
        huge = roundtrip_host_value(10**30)
        inf = roundtrip_host_value(float("inf"))
        numeric_ok = (
            not numeric_fits(2**53 + 1, "json-number") and numeric_fits(2**53 - 1, "json-number")
            and numeric_fits(7, "float") and not numeric_fits(2**53 + 1, "float")
            and numeric_fits(2.0, "int") and not numeric_fits(0.5, "int")
            and big["loss_status"] == "lossy" and big["equal"] is True      # marked, not mangled
            and huge["loss_status"] == "lossy" and huge["equal"] is True
            and inf["loss_status"] == "lossy"
            and roundtrip_host_value(0.1)["loss_status"] == "lossless"       # safe float untouched
        )
        ready = bool(lossless_ok and tuple_ok and set_ok and bytes_ok
                     and nonstr_ok and collision_ok and path_ok and nested_path_ok and opaque_ok
                     and numeric_ok)
        return {"schema": "pnix-hy.interop.roundtrip.report.v0", "ready": ready, "available": True,
                "lossless_roundtrip": lossless_ok, "tuple_lossy": tuple_ok, "set_lossy": set_ok,
                "bytes_lossy": bytes_ok, "nonstr_key_lossy": nonstr_ok,
                "collision_lossy": collision_ok, "path_provenance_lossy": path_ok,
                "nested_path_lossy": nested_path_ok, "opaque_by_ref": opaque_ok,
                "numeric_predicates": numeric_ok}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop.roundtrip.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def host_bridge_report() -> dict[str, Any]:
    """Self-check for proposal 0002 (B1/B2/B3/B5): pnix SOURCE can apply a host callable injected
    into the env (capability-gated); call_host threads kwargs; host arity projects to the pnix
    functionArgs shape; a host module's callables become applicable in pnix."""
    try:
        # B1: inject host callables into a pnix env; pnix SOURCE applies them (unary + curried)
        ctx = rt.runtime_context(None)
        ctx["env"] = {"pyLen": host_callable_to_pnix(len),
                      "pyAdd": host_callable_to_pnix(lambda a, b: a + b)}
        b1_unary = rt.eval_source_raw("pyLen [1 2 3]", ctx, realize=True) == 3
        b1_curried = rt.eval_source_raw("pyAdd 3 4", ctx, realize=True) == 7
        # capability gate: without host-call the injected callable denies at apply time
        ctx_denied = rt.runtime_context(None)
        ctx_denied["env"] = {"pyLen": host_callable_to_pnix(len, granted=())}
        try:
            rt.eval_source_raw("pyLen [1 2 3]", ctx_denied, realize=True)
            b1_gated = False
        except Exception:  # noqa: BLE001
            b1_gated = True
        # B2: call_host threads kwargs; record carries a witness
        def _kw(a: int, b: int = 0, *, c: int = 0) -> int:
            return a + b + c
        kv, krec = call_host(_kw, (1,), kwargs={"b": 2, "c": 3})
        b2 = kv == 6 and krec.effect_class == "host-call" and bool(krec.witness_id)
        # B3: arity projects into the pnix functionArgs {name: has_default} shape
        arity = host_callable_arity(_kw)
        b3 = arity.get("a") is False and arity.get("b") is True and arity.get("c") is True
        # B5: a host module's callables become applicable inside pnix
        import types as _types  # noqa: PLC0415
        module = _types.SimpleNamespace(add=lambda a, b: a + b, ten=10)
        mod = host_module_to_pnix(module, wrap_callables=True)
        ctxm = rt.runtime_context(None)
        ctxm["env"] = {"m": mod}
        b5 = (rt.eval_source_raw("m.add 3 4", ctxm, realize=True) == 7
              and rt.eval_source_raw("m.ten", ctxm, realize=True) == 10)
        ready = bool(b1_unary and b1_curried and b1_gated and b2 and b3 and b5)
        return {"schema": "pnix-hy.interop.host-bridge.report.v0", "ready": ready, "available": True,
                "b1_unary": b1_unary, "b1_curried": b1_curried, "b1_capability_gated": b1_gated,
                "b2_kwargs": b2, "b3_arity": b3, "b5_module_callables": b5}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop.host-bridge.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def no_mirror_report() -> dict[str, Any]:
    """C8 invariant guard (SCOPE_LOCK): interop works with the mirror OFF. Every crossing runs
    against a plain runtime_context that has NO `events`/mirror, so interop can never silently
    become mirror-dependent (a regression that would break standalone use)."""
    try:
        # host->pnix and pnix->host pure data, no mirror context anywhere
        pv, pr = from_host({"a": [1, 2]})
        from_ok = pv == {"a": [1, 2]} and pr.loss_status == "lossless"
        hv, _ = to_host(rt.eval_source("{ a = 1; b = [2 3]; }"))
        to_ok = hv == {"a": 1, "b": [2, 3]}
        # opaque ref lifecycle
        ref = make_opaque_ref(len)
        opaque_ok = resolve_opaque(ref) is len
        # host callable, capability-gated
        cv, _ = call_host(len, ([1, 2, 3],))
        call_ok = cv == 3
        # host callable applied from WITHIN a pnix eval whose ctx has no events/mirror
        ctx = rt.runtime_context(None)
        assert "events" not in ctx  # the eval path must not require a mirror event sink
        ctx["env"] = {"pyLen": host_callable_to_pnix(len)}
        bridge_ok = rt.eval_source_raw("pyLen [1 2 3]", ctx, realize=True) == 3
        ready = bool(from_ok and to_ok and opaque_ok and call_ok and bridge_ok)
        return {"schema": "pnix-hy.interop.no-mirror.report.v0", "ready": ready, "available": True,
                "from_host": from_ok, "to_host": to_ok, "opaque": opaque_ok,
                "call_host": call_ok, "host_bridge": bridge_ok, "mirror_independent": True}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop.no-mirror.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def error_contract_report() -> dict[str, Any]:
    """D1: host exceptions are UNAMBIGUOUS (not confusable with an attrset); try_call_host is
    tryEval-shaped; a legit pnix attrset with an `exception` key is not misread; and
    wrap_pnix_callable raises a typed InteropError instead of leaking a raw PnixError."""
    try:
        def _boom() -> Any:
            raise ValueError("boom")
        res, rec = call_host(_boom)
        exc_ok = (is_interop_error(res)
                  and (interop_error_of(res) or {}).get("type") == "ValueError"
                  and rec.output_kind == "exception")
        # a legit pnix attrset {exception = 1} must NOT be read as an interop error
        attr_host, _ = to_host(rt.eval_source("{ exception = 1; }"))
        not_misread = attr_host == {"exception": 1} and not is_interop_error(attr_host)
        ok = try_call_host(len, ([1, 2, 3],))
        fail = try_call_host(_boom)
        denied = try_call_host(len, ([1, 2, 3],), granted=())
        try_ok = (ok.get("success") is True and ok.get("value") == 3
                  and fail.get("success") is False and fail.get("error", {}).get("type") == "ValueError"
                  and denied.get("success") is False and denied.get("error", {}).get("kind") == "denied")
        # wrap_pnix_callable raises a typed InteropError on a pnix eval failure
        bad = pnix_callable("x: x + 1")
        try:
            bad("notanumber")
            wrap_ok = False
        except InteropError:
            wrap_ok = True
        except Exception:  # noqa: BLE001
            wrap_ok = False
        # A13 regression pin: an EAGER pnix error at pnix_callable build time is also typed
        try:
            pnix_callable('throw "boom"')
            eager_ok = False
        except InteropError:
            eager_ok = True
        except Exception:  # noqa: BLE001
            eager_ok = False
        ready = bool(exc_ok and not_misread and try_ok and wrap_ok and eager_ok)
        return {"schema": "pnix-hy.interop.error-contract.report.v0", "ready": ready, "available": True,
                "host_exception_unambiguous": exc_ok, "attrset_not_misread": not_misread,
                "try_call_host_shape": try_ok, "wrap_raises_interop_error": wrap_ok,
                "eager_eval_raises_interop_error": eager_ok}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop.error-contract.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def opaque_lifecycle_report() -> dict[str, Any]:
    """D2: the pnix fallback opaque-ref lifecycle is observable -- a local ref resolves, releasing
    it flips the lifecycle to released, and a made-not-released ref is countable as a leak signal.
    The opaque-ref DICT shape is unchanged (lifecycle is a lane-local sidecar)."""
    try:
        before = opaque_lifecycle()
        marker_a, marker_b = object(), object()
        ref_a = make_opaque_ref(marker_a, "probe", prefer_host=False)  # force the pnix lane
        ref_b = make_opaque_ref(marker_b, "probe", prefer_host=False)
        shape_ok = "__pnix_opaque__" in ref_a and set(ref_a) == {"__pnix_opaque__", "kind", "repr", "witness_id"}
        resolve_ok = resolve_opaque(ref_a) is marker_a
        mid = opaque_lifecycle()
        created_two = mid["live"] - before["live"] == 2
        release_opaque(ref_a)
        after = opaque_lifecycle()
        released_one = after["released"] - before["released"] == 1 and after["live"] - before["live"] == 1
        leak_signal = (after["live"] - before["live"]) >= 1  # ref_b created but not released (delta)
        invariant_ok = after["total"] == after["live"] + after["released"]  # id-reuse-safe accounting
        release_opaque(ref_b)  # cleanup so repeated runs are idempotent
        # 0016 (I2): own/borrow discipline -- releasing while lent is a typed trap; after all
        # lends return, release succeeds; nested lends require every lease to end.
        marker_c = object()
        ref_c = make_opaque_ref(marker_c, "probe", prefer_host=False)
        with lend_opaque(ref_c):
            try:
                release_opaque(ref_c)
                lend_guard = False
            except InteropError:
                lend_guard = True
            with lend_opaque(ref_c):  # nested borrow: 2 active leases
                nested_active = opaque_lifecycle()["lends_active"] >= 2
        release_opaque(ref_c)  # all leases returned -> owner release succeeds
        borrow_ok = bool(lend_guard and nested_active and opaque_lifecycle()["lends_active"] == 0)
        ready = bool(shape_ok and resolve_ok and created_two and released_one
                     and leak_signal and invariant_ok and borrow_ok)
        return {"schema": "pnix-hy.interop.opaque-lifecycle.report.v0", "ready": ready, "available": True,
                "ref_shape_unchanged": shape_ok, "resolves": resolve_ok,
                "created_tracked": created_two, "release_tracked": released_one,
                "leak_countable": leak_signal, "invariant_total": invariant_ok,
                "own_borrow_discipline": borrow_ok,
                "snapshot": opaque_lifecycle()}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop.opaque-lifecycle.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# --- 0020/I5+I6: hardened surfaces + substrate-enforced invariants ---
_OPAQUE_SURFACE: dict[int, str] = {}
_OPAQUE_INVARIANTS: dict[int, dict[str, Any]] = {}


def _surface_fingerprint(obj: Any) -> str:
    """A content fingerprint of the object's PUBLIC callable surface: method names + the code
    identity behind them (class attr code bytes) + whether the instance shadows them."""
    import hashlib as _hashlib  # noqa: PLC0415

    parts: list[str] = []
    for name in sorted(opaque_allowed_methods(obj)):
        fn = getattr(type(obj), name, None)
        code = getattr(fn, "__code__", None)
        # bytecode alone misses constant-only edits (co_code is identical for `return "a"` vs
        # `return "b"`); include the constant/name pools in the fingerprint.
        ident = (f"{code.co_code.hex()}|{code.co_consts!r}|{code.co_names!r}"
                 if code is not None else repr(fn))
        shadowed = isinstance(getattr(obj, "__dict__", None), dict) and name in obj.__dict__
        parts.append(f"{name}:{ident}:{int(bool(shadowed))}")
    return _hashlib.sha256("|".join(parts).encode("utf-8")).hexdigest()


def harden_opaque(ref: dict[str, Any]) -> dict[str, Any]:
    """0020/I5 (SES harden): freeze-witness the ref's public method surface. Every subsequent
    `call_host_method`/`opaque_call_method` re-fingerprints the surface first and refuses a
    tampered object (`surface-tampered`) -- guests get a tamper-EVIDENT shared surface."""
    obj = resolve_opaque(ref)
    fp = _surface_fingerprint(obj)
    _OPAQUE_SURFACE[opaque_ref_id(ref)] = fp
    witness = _local_witness("harden-opaque", {"surface_sha256": fp})
    return {"surface_sha256": fp, "witness_id": witness["witness_id"]}


def _surface_guard(ref: dict[str, Any], obj: Any) -> None:
    want = _OPAQUE_SURFACE.get(opaque_ref_id(ref))
    if want is not None and _surface_fingerprint(obj) != want:
        raise InteropError("surface-tampered: hardened opaque surface changed since harden_opaque",
                           blame="host")


def declare_opaque_invariants(ref: dict[str, Any], frozen_attrs: tuple[str, ...] | list[str]) -> dict[str, Any]:
    """0020/I6 (Trustworthy Proxies): declare attributes whose values must NEVER change for the
    life of the ref. Enforcement lives in the call entrypoint (the substrate), not in any
    wrapper the guest would have to trust."""
    obj = resolve_opaque(ref)
    snap = {a: getattr(obj, a) for a in frozen_attrs}
    _OPAQUE_INVARIANTS[opaque_ref_id(ref)] = snap
    witness = _local_witness("declare-invariants", {"frozen": sorted(snap)})
    return {"frozen_attrs": sorted(snap), "witness_id": witness["witness_id"]}


def _invariant_guard(ref: dict[str, Any], obj: Any) -> None:
    inv = _OPAQUE_INVARIANTS.get(opaque_ref_id(ref))
    if not inv:
        return
    for attr, want in inv.items():
        cur = getattr(obj, attr, _RELEASE_MISS)
        if cur is _RELEASE_MISS or cur != want:
            raise InteropError(f"invariant-violated: frozen attr {attr!r} changed", blame="host")


def interop_hardening_report() -> dict[str, Any]:
    """Self-check (proposal 0020): revocable capabilities, context-scoped refs, blame direction,
    hardened surfaces, and substrate-enforced invariants all behave as specified."""
    try:
        # I1: runtime-revocable capability handle
        cap = grant_capability("host-call")
        ok1, _ = call_host(len, ([1, 2],), granted=(cap,))
        cap.suspend()
        den1, _ = call_host(len, ([1, 2],), granted=(cap,))
        cap.resume()
        ok2, _ = call_host(len, ([1, 2],), granted=(cap,))
        cap.revoke()
        den2, _ = call_host(len, ([1, 2],), granted=(cap,))
        att = grant_capability("host-call", "file-read").attenuate("host-call")
        den3, _ = call_host(len, ([1, 2],), granted=(att,))
        cap_ok = (ok1 == 2 and ok2 == 2
                  and all(isinstance(d, dict) and "denied" in d for d in (den1, den2, den3)))

        # I3: context-scoped opaque refs
        with interop_context():
            ref = make_opaque_ref(object(), "probe", prefer_host=False)
            inside_ok = resolve_opaque(ref) is not None
        try:
            resolve_opaque(ref)
            ctx_ok = False
        except InteropError as exc:
            ctx_ok = inside_ok and "closed" in str(exc)
        lifecycle_ok = opaque_lifecycle()["total"] == (opaque_lifecycle()["live"]
                                                       + opaque_lifecycle()["released"])

        # I4: blame direction (pnix-side vs host-side contract violation)
        try:
            pnix_callable('throw "boom"')
            blame_pnix = None
        except InteropError as exc:
            blame_pnix = exc.blame
        def _boom() -> Any:
            raise ValueError("host boom")
        res, _ = call_host(_boom)
        blame_host = (interop_error_of(res) or {}).get("blame")
        blame_ok = blame_pnix == "pnix" and blame_host == "host"

        # I5: hardened surface tamper detection
        class _Probe:
            def ping(self) -> str:
                return "pong"
        probe = _Probe()
        pref = make_opaque_ref(probe, "probe", prefer_host=False)
        harden_opaque(pref)
        v1, _ = call_host_method(pref, "ping")
        _Probe.ping = lambda self: "tampered"  # type: ignore[method-assign]
        v2, r2 = call_host_method(pref, "ping")
        tamper_ok = v1 == "pong" and isinstance(v2, dict) and is_interop_error(v2) is False             and "denied" not in (v2 if isinstance(v2, dict) else {})
        # the guard raises InteropError -> call_host_method captures it as a D1 error value
        tamper_ok = v1 == "pong" and is_interop_error(v2)             and "surface-tampered" in (interop_error_of(v2) or {}).get("message", "")
        release_opaque(pref)

        # I6: substrate-enforced frozen attribute
        class _Inv:
            def __init__(self) -> None:
                self.k = 1
            def get(self) -> int:
                return self.k
        inv_obj = _Inv()
        iref = make_opaque_ref(inv_obj, "probe", prefer_host=False)
        declare_opaque_invariants(iref, ("k",))
        w1, _ = call_host_method(iref, "get")
        inv_obj.k = 2
        w2, _ = call_host_method(iref, "get")
        inv_ok = w1 == 1 and is_interop_error(w2)             and "invariant-violated" in (interop_error_of(w2) or {}).get("message", "")
        release_opaque(iref)

        ready = bool(cap_ok and ctx_ok and lifecycle_ok and blame_ok and tamper_ok and inv_ok)
        return {"schema": "pnix-hy.interop-hardening.report.v0", "ready": ready, "available": True,
                "revocable_capability": cap_ok, "context_scoped_refs": ctx_ok,
                "lifecycle_invariant": lifecycle_ok, "blame_direction": blame_ok,
                "harden_tamper_detected": tamper_ok, "invariant_enforced": inv_ok}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.interop-hardening.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}
