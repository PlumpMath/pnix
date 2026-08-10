"""hy-meta host execution/artifact floor (SR2 / SEP2).

The canonical host (CPython) execution primitive: compile+exec emitted Python in a given
namespace. pnix-hy's compiler lane (pnix -> Python emitter) delegates the host EXECUTION
here, so "pnix-hy uses hy-meta as its host floor" holds for code execution too -- pnix-hy
owns the pnix->Python EMITTER, hy-meta owns running the result on the host.

Pure stdlib; no `hy`, no pnix. Semantics are byte-identical to a direct
`exec(compile(source, filename, "exec"), namespace)` so the pnix 4-lane compiler parity is
preserved when the compiler lane routes through here.

The artifact helpers below are lazy facades over the canonical stage8 artifact functions
already resident in bootstrap.py. Loading bootstrap is intentionally deferred so the hot
execution floor remains usable without paying the Hy bootstrap import cost.
"""

from __future__ import annotations

import ast
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

_HERE = str(Path(__file__).resolve().parent)
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

_BOOTSTRAP: Any = None
_ARTIFACT_CACHE: dict[str, dict[str, Any]] = {}
_ROUNDTRIP_STATUS_VOCAB: tuple[str, ...] | None = None


def _load_bootstrap() -> Any:
    global _BOOTSTRAP
    if _BOOTSTRAP is not None:
        return _BOOTSTRAP
    existing = sys.modules.get("bootstrap")
    if existing is not None and hasattr(existing, "artifact_from_ast"):
        _BOOTSTRAP = existing
        return _BOOTSTRAP
    root = Path(__file__).resolve().parents[1]
    bootstrap_path = root / "hy-meta" / "bootstrap.py"
    added: list[str] = []
    for path in (str(root), str(root / "hy-meta")):
        if path not in sys.path:
            sys.path.insert(0, path)
            added.append(path)
    try:
        spec = importlib.util.spec_from_file_location("bootstrap", str(bootstrap_path))
        if spec is None or spec.loader is None:
            raise ImportError(f"cannot load bootstrap from {bootstrap_path}")
        module = importlib.util.module_from_spec(spec)
        previous = sys.modules.get("bootstrap")
        sys.modules["bootstrap"] = module
        try:
            spec.loader.exec_module(module)
        except BaseException:
            if previous is None:
                sys.modules.pop("bootstrap", None)
            else:
                sys.modules["bootstrap"] = previous
            raise
        _BOOTSTRAP = module
        return module
    finally:
        for path in reversed(added):
            try:
                sys.path.remove(path)
            except ValueError:
                pass


def run_python_source(source: str, namespace: dict[str, Any], filename: str = "<pnix-px>") -> dict[str, Any]:
    """Compile `source` and execute it in `namespace` on the host. Returns the namespace."""
    exec(compile(source, filename, "exec"), namespace)  # noqa: S102 - host execution floor
    return namespace


def run_code_object(code: Any, namespace: dict[str, Any]) -> dict[str, Any]:
    """Execute an already-compiled code object in `namespace` on the host."""
    exec(code, namespace)  # noqa: S102 - host execution floor
    return namespace


def compile_python_ast(
    tree: ast.AST,
    filename: str = "<hy-meta:python-ast>",
    mode: str = "exec",
    *,
    location_stable: bool = True,
) -> Any:
    """Compile a Python AST through the same location-stable path used by stage8."""
    bootstrap = _load_bootstrap()
    code_tree = bootstrap.location_stable_ast(tree) if location_stable else tree
    return compile(code_tree, filename, mode)


def artifact_from_ast(
    *,
    name: str,
    source: str,
    filename: str,
    tree: ast.AST,
) -> dict[str, Any]:
    """Return the canonical hy-meta artifact record for a Python AST."""
    return _load_bootstrap().artifact_from_ast(
        name=name,
        source=source,
        filename=filename,
        tree=tree,
    )


def form_sha256(source: str) -> str:
    """0027/G2: the READ-stage identity of a source -- a content hash of its significant token
    stream (comment/blank-line insensitive, pre-parse), distinct from source_sha256 (raw bytes)
    and ast_sha256 (post-parse structure)."""
    import hashlib
    import io
    import tokenize

    parts: list[str] = []
    try:
        for tok in tokenize.generate_tokens(io.StringIO(source).readline):
            if tok.type in (tokenize.COMMENT, tokenize.NL, tokenize.NEWLINE, tokenize.INDENT,
                            tokenize.DEDENT, tokenize.ENCODING, tokenize.ENDMARKER):
                continue
            parts.append(f"{tok.type}:{tok.string}")
    except tokenize.TokenizeError:
        parts = [source]
    return hashlib.sha256("\x1f".join(parts).encode("utf-8")).hexdigest()


def artifact_from_source(
    source: str,
    *,
    name: str = "python-source",
    filename: str = "<hy-meta:python-source>",
    mode: str = "exec",
) -> dict[str, Any]:
    """Parse Python source and return the canonical artifact record (0027/G2: now including
    the read-stage `form_sha256` alongside the existing hashes; purely additive)."""
    if mode != "exec":
        raise ValueError("artifact_from_source currently exposes the stage8 exec artifact lane")
    record = artifact_from_ast(
        name=name,
        source=source,
        filename=filename,
        tree=ast.parse(source, filename=filename, mode=mode),
    )
    record.setdefault("form_sha256", form_sha256(source))
    return record


def artifact_summary(artifact: dict[str, Any]) -> dict[str, Any]:
    """Drop heavyweight AST/Python payload fields from an artifact record."""
    return _load_bootstrap().artifact_summary(artifact)


def pyc_bytes_for_code(code: Any, source: str, filename: str) -> bytes:
    """Return the canonical timestamp-pyc bytes used by the artifact lane."""
    return _load_bootstrap().pyc_bytes_for_code(code, source, filename)


def stable_code_payload(code: Any) -> dict[str, Any]:
    """Return the marshal-reference-free code-object payload used by stage8."""
    return _load_bootstrap().stable_code_payload(code)


def compare_artifacts(
    left: dict[str, Any],
    right: dict[str, Any],
) -> dict[str, Any]:
    """Compare canonical artifact records or artifact bundles via the stage8 comparator."""
    bootstrap = _load_bootstrap()
    keys = set(bootstrap.stage8_hash_keys())
    if keys <= set(left) and keys <= set(right):
        return bootstrap.compare_stage8_artifact_bundles({"artifact": left}, {"artifact": right})
    return bootstrap.compare_stage8_artifact_bundles(left, right)


def classify_drift(keys: list[str]) -> dict[str, Any]:
    """Classify host artifact/value/environment drift through bootstrap's canonical API."""
    return _load_bootstrap().classify_drift(keys)


def classify_drift_report() -> dict[str, Any]:
    """Smoke report for the host drift classifier facade."""
    return _load_bootstrap().classify_drift_report()


def roundtrip_status_vocabulary() -> tuple[str, ...]:
    """Read pnix's shared roundtrip status vocabulary without importing pnix runtime code."""
    global _ROUNDTRIP_STATUS_VOCAB
    if _ROUNDTRIP_STATUS_VOCAB is not None:
        return _ROUNDTRIP_STATUS_VOCAB
    root = Path(__file__).resolve().parents[1]
    pnix_mirror = root / "pnix-hy" / "pnix_hy" / "pnix_mirror.py"
    tree = ast.parse(pnix_mirror.read_text(), filename=str(pnix_mirror), mode="exec")
    for node in tree.body:
        if not isinstance(node, ast.Assign):
            continue
        if not any(isinstance(target, ast.Name) and target.id == "ROUNDTRIP_STATUS_VOCAB"
                   for target in node.targets):
            continue
        value = ast.literal_eval(node.value)
        if not isinstance(value, tuple) or not all(isinstance(item, str) for item in value):
            raise TypeError("ROUNDTRIP_STATUS_VOCAB must be a tuple[str, ...]")
        _ROUNDTRIP_STATUS_VOCAB = value
        return value
    raise LookupError(f"ROUNDTRIP_STATUS_VOCAB not found in {pnix_mirror}")


def _roundtrip_status(*, ok: bool, comparable: bool = True, lossy: bool = False) -> str:
    vocab = roundtrip_status_vocabulary()
    if not comparable:
        status = "held"
    elif not ok:
        status = "rejected"
    elif lossy:
        status = "lossy-ok"
    else:
        status = "lossless"
    if status not in vocab:
        raise ValueError(f"roundtrip status {status!r} is not in shared vocabulary")
    return status


def _namespace_result(namespace: dict[str, Any]) -> dict[str, str]:
    return {
        key: repr(value)
        for key, value in sorted(namespace.items())
        if key != "__builtins__" and not key.startswith("__")
    }


def roundtrip_python_ast(
    source: str,
    *,
    name: str = "host-python-ast-roundtrip",
    filename: str = "<hy-meta:roundtrip-python-ast>",
    mode: str = "exec",
) -> dict[str, Any]:
    """Roundtrip Python source through AST->source->AST using canonical host artifacts."""
    if mode != "exec":
        raise ValueError("roundtrip_python_ast currently exposes the stage8 exec artifact lane")
    original = artifact_from_source(source, name=name, filename=filename, mode=mode)
    regenerated_source = original["python_source"]
    regenerated = artifact_from_source(
        regenerated_source,
        name=f"{name}:regenerated",
        filename=filename,
        mode=mode,
    )
    stable_keys = ["ast_sha256", "python_sha256", "normalized_sha256", "code_sha256"]
    mismatches = [key for key in stable_keys if original[key] != regenerated[key]]
    source_lossless = original["source_sha256"] == regenerated["source_sha256"]
    status = _roundtrip_status(ok=not mismatches, lossy=not source_lossless)
    return {
        "schema": "hy-meta.roundtrip-python-ast.v0",
        "status": status,
        "vocabulary": roundtrip_status_vocabulary(),
        "source_lossless": source_lossless,
        "mismatches": mismatches,
        "original": artifact_summary(original),
        "regenerated": artifact_summary(regenerated),
        "regenerated_source": regenerated_source,
    }


def roundtrip_code_result(
    source: str,
    *,
    filename: str = "<hy-meta:roundtrip-code-result>",
    mode: str = "exec",
) -> dict[str, Any]:
    """Compile source, verify marshal roundtrip via host_introspect, and execute the code."""
    if mode != "exec":
        raise ValueError("roundtrip_code_result currently exposes exec code-result checks")
    import host_introspect  # noqa: PLC0415

    tree = ast.parse(source, filename=filename, mode=mode)
    code = compile_python_ast(tree, filename)
    marshal_info = host_introspect.marshal_code(code)
    namespace: dict[str, Any] = {}
    run_code_object(code, namespace)
    status = _roundtrip_status(ok=bool(marshal_info.get("roundtrip_ok")))
    return {
        "schema": "hy-meta.roundtrip-code-result.v0",
        "status": status,
        "vocabulary": roundtrip_status_vocabulary(),
        "marshal": {
            "marshal_version": marshal_info.get("marshal_version"),
            "blob_len": marshal_info.get("blob_len"),
            "roundtrip_ok": marshal_info.get("roundtrip_ok"),
        },
        "result": _namespace_result(namespace),
    }


def roundtrip_stage(debug_dir: str | None = None) -> dict[str, Any]:
    """Wrap the canonical stage8 fresh-stage reproduction check in the shared status vocab."""
    stage8 = _load_bootstrap().run_stage8_check(debug_dir)
    stage_status = stage8.get("stage8_status")
    drift_class = stage8.get("stage8_drift_class")
    ok = stage_status == "reproduced"
    status = _roundtrip_status(ok=ok, comparable=True, lossy=ok and drift_class != "none")
    return {
        "schema": "hy-meta.roundtrip-stage.v0",
        "status": status,
        "vocabulary": roundtrip_status_vocabulary(),
        "stage8_status": stage_status,
        "stage8_drift_class": drift_class,
        "stage8_drift_kinds": stage8.get("stage8_drift_kinds"),
        "artifact_count": stage8.get("artifact_count"),
    }


def roundtrip_report() -> dict[str, Any]:
    try:
        source = "answer = 40 + 2\n"
        ast_result = roundtrip_python_ast(source)
        code_result = roundtrip_code_result(source)
        stage_result = roundtrip_stage()
        vocab = roundtrip_status_vocabulary()
        statuses = [ast_result["status"], code_result["status"], stage_result["status"]]
        ready = (
            set(vocab) == {"lossless", "lossy-ok", "held", "rejected"}
            and all(status in vocab for status in statuses)
            and ast_result["status"] in {"lossless", "lossy-ok"}
            and code_result["status"] == "lossless"
            and stage_result["status"] in {"lossless", "lossy-ok"}
        )
        return {
            "schema": "hy-meta.roundtrip.report.v0",
            "ready": ready,
            "available": True,
            "vocabulary": vocab,
            "statuses": {
                "python_ast": ast_result["status"],
                "code_result": code_result["status"],
                "stage": stage_result["status"],
            },
        }
    except Exception as exc:  # noqa: BLE001
        return {
            "schema": "hy-meta.roundtrip.report.v0",
            "ready": False,
            "available": False,
            "error": f"{type(exc).__name__}: {exc}",
        }


def _host_hy_version() -> str:
    try:
        import hy  # noqa: PLC0415
    except Exception:  # noqa: BLE001
        return "unknown"
    return str(getattr(hy, "__version__", "unknown"))


def cache_key(
    source: str,
    *,
    compiler: str = "host_exec.artifact_from_source",
    stage: str = "host",
    env: dict[str, Any] | None = None,
    python_version: str | None = None,
    hy_version: str | None = None,
) -> str:
    """Content-addressed host artifact cache key."""
    payload = {
        "source_sha256": _load_bootstrap().sha256_text(source),
        "compiler": compiler,
        "stage": stage,
        "env": dict(sorted((env or {}).items())),
        "python": python_version or sys.version.split()[0],
        "hy_version": hy_version or _host_hy_version(),
    }
    body = json.dumps(payload, sort_keys=True, separators=(",", ":"), default=repr)
    return "host-artifact-" + _load_bootstrap().sha256_text(body)


def cache_get(key: str) -> dict[str, Any] | None:
    """Return a cached host artifact by key, or None."""
    return _ARTIFACT_CACHE.get(key)


def cache_put(key: str, artifact: dict[str, Any]) -> str:
    """Store a host artifact by key and return the key."""
    _ARTIFACT_CACHE[key] = artifact
    return key


def cache_clear() -> None:
    _ARTIFACT_CACHE.clear()


def artifact_cache_report() -> dict[str, Any]:
    try:
        cache_clear()
        source = "answer = 40 + 2\n"
        key = cache_key(source, env={"PYTHONHASHSEED": "0"})
        same_key = cache_key(source, env={"PYTHONHASHSEED": "0"})
        different_env_key = cache_key(source, env={"PYTHONHASHSEED": "random"})
        artifact = artifact_from_source(source, name="artifact-cache-report",
                                        filename="<hy-meta:artifact-cache-report>")
        cache_put(key, artifact)
        hit = cache_get(same_key)
        ready = (
            key == same_key
            and key != different_env_key
            and hit is artifact
            and len((hit or {}).get("code_sha256", "")) == 64
        )
        return {"schema": "hy-meta.artifact-cache.report.v0", "ready": bool(ready),
                "available": True, "cache_key": key[:28], "hit": hit is artifact,
                "env_separates": key != different_env_key, "size": len(_ARTIFACT_CACHE)}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "hy-meta.artifact-cache.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def compile_artifact_witness(
    source: str,
    *,
    name: str = "host-compile-artifact",
    filename: str = "<hy-meta:compile-artifact-witness>",
    mode: str = "exec",
) -> dict[str, Any]:
    """Emit a witness for the host compile/artifact lane using the canonical artifact summary."""
    if mode != "exec":
        raise ValueError("compile_artifact_witness currently exposes the stage8 exec artifact lane")
    from witness import record_witness  # noqa: PLC0415

    artifact = artifact_from_source(source, name=name, filename=filename, mode=mode)
    summary = artifact_summary(artifact)
    witness_payload = {key: value for key, value in summary.items() if key != "raw_code_sha256"}
    witness = record_witness("host-compile-artifact", witness_payload)
    return {
        "schema": "hy-meta.compile-artifact-witness.v0",
        "artifact": summary,
        "witness": witness,
        "witness_id": witness["witness_id"],
    }


def compile_artifact_witness_report() -> dict[str, Any]:
    try:
        first = compile_artifact_witness("answer = 40 + 2\n")
        second = compile_artifact_witness("answer = 40 + 2\n")
        artifact = first.get("artifact") or {}
        ready = (
            len(artifact.get("source_sha256", "")) == 64
            and len(artifact.get("code_sha256", "")) == 64
            and first.get("witness_id") == second.get("witness_id")
            and (first.get("witness") or {}).get("kind") == "host-compile-artifact"
        )
        return {"schema": "hy-meta.compile-artifact-witness.report.v0", "ready": bool(ready),
                "available": True, "witness_id": first.get("witness_id"),
                "deterministic": first.get("witness_id") == second.get("witness_id")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "hy-meta.compile-artifact-witness.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def reify_host(
    source: str,
    *,
    name: str = "host-reify",
    filename: str = "<hy-meta:reify-host>",
    mode: str = "exec",
) -> dict[str, Any]:
    """Uniform host reification over Python source, AST/code artifacts, introspection, and witness.

    This is a facade over the existing stage8 artifact and host_introspect surfaces; it does
    not add a second compiler or artifact implementation.
    """
    if mode != "exec":
        raise ValueError("reify_host currently exposes the stage8 exec artifact lane")
    import host_introspect  # noqa: PLC0415
    from witness import record_witness  # noqa: PLC0415

    artifact = artifact_from_source(source, name=name, filename=filename, mode=mode)
    summary = artifact_summary(artifact)
    introspection = host_introspect.full_introspection(source, mode=mode)
    witness = record_witness("host-reify", {
        "name": name,
        "filename": filename,
        "source_sha256": summary.get("source_sha256"),
        "ast_sha256": summary.get("ast_sha256"),
        "code_sha256": summary.get("code_sha256"),
        "pyc_sha256": summary.get("pyc_sha256"),
    })
    return {
        "schema": "hy-meta.reify-host.v0",
        "ok": True,
        "source": {"text": source, "sha256": summary.get("source_sha256")},
        "artifact": summary,
        "introspection": {
            "code_object": introspection.get("code_object"),
            "bytecode_op_count": len(introspection.get("bytecode") or []),
            "ast": introspection.get("ast"),
            "symtable": introspection.get("symtable"),
        },
        "witness": witness,
    }


def reify_host_report() -> dict[str, Any]:
    try:
        r = reify_host("answer = 40 + 2\n", name="host-reify-report",
                       filename="<hy-meta:reify-host-report>")
        artifact = r.get("artifact") or {}
        ready = (
            r.get("ok") is True
            and len(artifact.get("source_sha256", "")) == 64
            and len(artifact.get("code_sha256", "")) == 64
            and (r.get("introspection") or {}).get("bytecode_op_count", 0) > 0
            and len((r.get("witness") or {}).get("witness_id", "")) == 16
        )
        return {"schema": "hy-meta.reify-host.report.v0", "ready": bool(ready),
                "available": True, "source_sha256": artifact.get("source_sha256", "")[:12],
                "code_sha256": artifact.get("code_sha256", "")[:12],
                "witness_id": (r.get("witness") or {}).get("witness_id")}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "hy-meta.reify-host.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


def host_exec_report() -> dict[str, Any]:
    """Lightweight SR2 smoke report for host exec + artifact API exposure."""
    try:
        source = "answer = 40 + 2\n"
        tree = ast.parse(source, filename="<hy-meta:host-exec-report>", mode="exec")
        code = compile_python_ast(tree, "<hy-meta:host-exec-report>")
        namespace: dict[str, Any] = {}
        run_code_object(code, namespace)
        artifact = artifact_from_ast(
            name="host-exec-report",
            source=source,
            filename="<hy-meta:host-exec-report>",
            tree=tree,
        )
        summary = artifact_summary(artifact)
        diff = compare_artifacts({"probe": artifact}, {"probe": artifact})
        drift = classify_drift(["code_sha256", "pyc_sha256"])
        reified = reify_host(source, name="host-exec-report", filename="<hy-meta:host-exec-report>")
        ready = (
            namespace.get("answer") == 42
            and "ast_dump" not in summary
            and "python_source" not in summary
            and diff["missing_from_stage8"] == []
            and diff["new_in_stage8"] == []
            and diff["changed"] == {}
            and drift["classification"] == "mixed:bytecode,pyc"
            and reified.get("ok") is True
        )
        return {
            "schema": "hy-meta.host-exec.report.v0",
            "ready": ready,
            "available": True,
            "answer": namespace.get("answer"),
            "artifact_keys": sorted(summary),
            "drift": diff,
            "drift_classification": {"classification": drift["classification"], "kinds": drift["kinds"]},
            "reify_host": {"witness_id": (reified.get("witness") or {}).get("witness_id")},
        }
    except Exception as exc:  # noqa: BLE001
        return {
            "schema": "hy-meta.host-exec.report.v0",
            "ready": False,
            "available": False,
            "error": f"{type(exc).__name__}: {exc}",
        }


__all__ = [
    "artifact_from_ast",
    "artifact_from_source",
    "artifact_summary",
    "artifact_cache_report",
    "cache_clear",
    "cache_get",
    "cache_key",
    "cache_put",
    "classify_drift",
    "classify_drift_report",
    "compare_artifacts",
    "compile_artifact_witness",
    "compile_artifact_witness_report",
    "compile_python_ast",
    "host_exec_report",
    "pyc_bytes_for_code",
    "reify_host",
    "reify_host_report",
    "roundtrip_code_result",
    "roundtrip_python_ast",
    "roundtrip_report",
    "roundtrip_stage",
    "roundtrip_status_vocabulary",
    "run_code_object",
    "run_python_source",
    "stable_code_payload",
]
