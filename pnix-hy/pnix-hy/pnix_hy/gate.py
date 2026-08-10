"""pnix_hy.gate -- pnix runtime gates + witnesses (SEP5).

Builds on PP2's static_purity_check (all-or-nothing pure/impure) to give a CAPABILITY-AWARE
gate: each impure builtin is mapped to an effect class (file-read / file-write / host-call /
import / network), and a program is admitted only when every effect it requires is granted.
Plus deterministic content-hashed witnesses so eval/stage/mirror/interop conversions leave a
verifiable record. Pure-side only; the host-call enforcement point is the interop boundary
(interop.check_capability) and safe_eval.
"""

from __future__ import annotations

from typing import Any

from . import interop as _interop
from .pnix_mirror import _sha256, static_purity_check

# impure pnix builtin -> effect class. Canonical table lives beside interop.EFFECT_CLASSES.
EFFECT_OF: dict[str, str] = dict(_interop.IMPURE_BUILTIN_EFFECTS)
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


def gate_check(source: str, *, granted: tuple[str, ...] | list[str] = ()) -> dict[str, Any]:
    """Capability-aware admission: classify a program's required effects and admit it only
    if every required effect is granted. Schema `pnix-hy.gate-check.v0`."""
    granted_set = set(granted)
    purity = static_purity_check(source)
    if purity.get("parse_error"):
        return {"schema": "pnix-hy.gate-check.v0", "source": source, "allowed": False,
                "parse_error": purity["parse_error"]}
    required: list[dict[str, Any]] = []
    for use in purity.get("impure_uses") or []:
        name = use.get("name")
        effect = "import" if use.get("kind") == "import" else EFFECT_OF.get(name, "unknown")
        required.append({"name": name, "kind": use.get("kind"), "effect": effect})
    uncertain = purity.get("uncertain") or []
    required_effects = sorted({r["effect"] for r in required})
    denials = [r for r in required if r["effect"] not in granted_set]
    allowed = not denials and not uncertain
    return {
        "schema": "pnix-hy.gate-check.v0",
        "source": source,
        "pure": purity.get("pure"),
        "required_effects": required_effects,
        "granted": sorted(granted_set),
        "denials": denials,
        "uncertain": uncertain,
        "allowed": allowed,
    }


def _witness_fields(payload: Any) -> dict[str, Any]:
    if not isinstance(payload, dict):
        return {field: None for field in WITNESS_FIELD_SCHEMA}
    in_hash = payload.get("in_hash") or payload.get("input_sha256") or payload.get("source_sha256")
    out_hash = (
        payload.get("out_hash")
        or payload.get("output_sha256")
        or payload.get("result_sha256")
        or payload.get("code_sha256")
    )
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
        "env_hash": payload.get("env_hash") or payload.get("environment_sha256"),
        "status": payload.get("status"),
        "loss": payload.get("loss") or loss_status,
    }


def make_witness(kind: str, payload: Any) -> dict[str, Any]:
    """A deterministic content-hashed witness record.

    Prefer hy-meta's canonical host witness emitter when available; retain the pnix-native
    schema only as a standalone fallback.
    """
    host = _interop._host_interop()
    if host is not None and hasattr(host, "make_witness"):
        try:
            return host.make_witness(kind, payload)
        except Exception:  # noqa: BLE001 - standalone pnix fallback below
            pass
    import json as _json  # noqa: PLC0415
    body = _json.dumps(payload, sort_keys=True, default=str)
    digest = _sha256(kind + "::" + body)
    return {"schema": "pnix-hy.witness.v0", "kind": kind, **_witness_fields(payload), "payload": payload,
            "sha256": digest, "witness_id": digest[:16]}


def gate_report() -> dict[str, Any]:
    """Self-check: pure admits with no caps; readFile needs file-read; import needs import;
    a denied program lists its denial; witnesses are deterministic."""
    try:
        pure = gate_check("let a = 1; in a + a")
        rf_denied = gate_check('builtins.readFile "/etc/passwd"')
        rf_ok = gate_check('builtins.readFile "/etc/passwd"', granted=("file-read",))
        imp = gate_check("import ./x.px")
        ex = gate_check("builtins.exec")
        flake = gate_check("builtins.getFlake")
        w1 = make_witness("eval", {"value": 21, "source": "x"})
        w2 = make_witness("eval", {"source": "x", "value": 21})  # key order irrelevant
        vocab_ok = set(EFFECT_OF) == set(_interop.IMPURE_BUILTINS) and all(
            effect in _interop.EFFECT_CLASSES for effect in EFFECT_OF.values()
        )
        # witness-schema drift-guard: the pnix fallback's shared §14 field set must all
        # appear in the canonical host emitter's output, so the two shared schemas cannot
        # silently diverge (the pnix WITNESS_FIELD_SCHEMA is a deliberate fallback copy).
        _host = _interop._host_interop()
        _local_fields = set(_witness_fields({}))
        if _host is not None and hasattr(_host, "make_witness"):
            witness_schema_ok = _local_fields <= set(_host.make_witness("eval", {"value": 1}))
        else:
            witness_schema_ok = True
        ready = (
            pure.get("allowed") is True and pure.get("required_effects") == []
            and rf_denied.get("allowed") is False and rf_denied.get("required_effects") == ["file-read"]
            and rf_ok.get("allowed") is True
            and imp.get("allowed") is False and imp.get("required_effects") == ["import"]
            and ex.get("required_effects") == ["subprocess"]
            and flake.get("required_effects") == ["network"]
            and vocab_ok
            and witness_schema_ok
            and w1.get("schema") in ("hy-meta.witness.v0", "pnix-hy.witness.v0")
            and w1["sha256"] == w2["sha256"] and len(w1["sha256"]) == 64
        )
        return {"schema": "pnix-hy.gate-check.report.v0", "ready": bool(ready), "available": True,
                "pure_allowed": pure.get("allowed"), "readfile_required": rf_denied.get("required_effects"),
                "readfile_granted_allowed": rf_ok.get("allowed"),
                "import_required": imp.get("required_effects"),
                "exec_required": ex.get("required_effects"),
                "getflake_required": flake.get("required_effects"),
                "vocab_shared": vocab_ok,
                "witness_schema": w1.get("schema"),
                "witness_schema_ok": witness_schema_ok,
                "witness_deterministic": w1["sha256"] == w2["sha256"]}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.gate-check.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}


# --- 0024 (R4, in-toto/SLSA style): predicate-typed witnesses -- PAYLOAD-level, envelope UNCHANGED ---
# The predicate type URI lives INSIDE the witness payload (make_witness hashes payload verbatim),
# so the shared §14 witness FIELD SCHEMA is untouched -- no both-lane/drift-guard change needed.
PREDICATE_TYPES: dict[str, str] = {
    "action": "https://pnix-hy.dev/attestation/action/v1",
    "realisation": "https://pnix-hy.dev/attestation/realisation/v1",
    "interop": "https://pnix-hy.dev/attestation/interop/v1",
    "eval": "https://pnix-hy.dev/attestation/eval/v1",
}
# schema evolution by deprecating + renaming type URIs (never mutating a schema in place)
DEPRECATED_PREDICATES: dict[str, str] = {
    "https://pnix-hy.dev/attestation/action/v0": "https://pnix-hy.dev/attestation/action/v1",
}


def migrate_predicate(uri: str) -> str:
    """Follow the deprecation chain to the CURRENT predicate type URI."""
    seen = set()
    while uri in DEPRECATED_PREDICATES and uri not in seen:
        seen.add(uri)
        uri = DEPRECATED_PREDICATES[uri]
    return uri


def is_known_predicate(uri: str) -> bool:
    return migrate_predicate(uri) in set(PREDICATE_TYPES.values())


def typed_witness(predicate_uri: str, payload: dict[str, Any], kind: str = "typed") -> dict[str, Any]:
    """A witness whose payload is TYPED by a versioned predicate URI (deprecated URIs are
    migrated on the way in). Envelope fields are exactly `make_witness`'s."""
    uri = migrate_predicate(predicate_uri)
    return make_witness(kind, {"_predicate_type": uri, **payload})


def predicate_of(witness: dict[str, Any]) -> str | None:
    payload = witness.get("payload")
    return payload.get("_predicate_type") if isinstance(payload, dict) else None


def typed_witness_report() -> dict[str, Any]:
    """Self-check (proposal 0024): typed witnesses are deterministic, carry a recoverable
    predicate URI, migrate deprecated URIs, flag unknown ones -- and the shared witness
    ENVELOPE is byte-for-byte the same field set as an untyped witness."""
    try:
        uri = PREDICATE_TYPES["realisation"]
        w1 = typed_witness(uri, {"drv": "abc", "out": "def"})
        w2 = typed_witness(uri, {"drv": "abc", "out": "def"})
        deterministic = w1["witness_id"] == w2["witness_id"]
        typed_ok = predicate_of(w1) == uri

        old = "https://pnix-hy.dev/attestation/action/v0"
        w3 = typed_witness(old, {"x": 1})
        migrated = predicate_of(w3) == PREDICATE_TYPES["action"] and is_known_predicate(old)
        unknown_flagged = not is_known_predicate("https://example.invalid/predicate/v9")

        plain = make_witness("typed", {"x": 1})
        envelope_same = set(plain.keys()) == set(w1.keys())
        schema_guard = bool(gate_report().get("witness_schema_ok"))

        ready = bool(deterministic and typed_ok and migrated and unknown_flagged
                     and envelope_same and schema_guard)
        return {"schema": "pnix-hy.typed-witness.report.v0", "ready": ready, "available": True,
                "deterministic": deterministic, "predicate_recoverable": typed_ok,
                "deprecated_migrates": migrated, "unknown_flagged": unknown_flagged,
                "envelope_unchanged": envelope_same, "witness_schema_guard": schema_guard}
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.typed-witness.report.v0", "ready": False,
                "available": False, "error": f"{type(exc).__name__}: {exc}"}
