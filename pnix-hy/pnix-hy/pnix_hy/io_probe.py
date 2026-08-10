"""Cross-host read-only effect adapter probe."""

from __future__ import annotations

import json
from pathlib import Path
import sys

from .interop import apply_effect_request

CAPABILITY_CLASS = {
    "capability_id": "pnix.io.file-read.v1",
    "entry_point": "host-meta-io-v1",
    "input_signature": "{path:string}",
    "output_shape": "value+receipt",
    "effect_scope": "read-only-filesystem",
    "risk_tier": "bounded-read",
    "discovery_source": "pnix-meta.effect-request.v1",
}


def request(effect: str, path: Path) -> dict[str, object]:
    return {
        "operation_id": effect,
        "args": {"path": str(path)},
        "capability_class": CAPABILITY_CLASS,
    }


def probe(root: Path) -> dict[str, object]:
    note = root / "note.txt"
    grants = ("file-read",)
    exists = apply_effect_request(request("fs.path-exists", note), grants)
    missing = apply_effect_request(request("fs.path-exists", root / "missing.txt"), grants)
    opened = apply_effect_request(request("fs.open", note), grants)
    typed = apply_effect_request(request("fs.file-type", note), grants)
    listed = apply_effect_request(request("fs.read-dir", root), grants)
    denied = apply_effect_request(request("fs.open", note), ())
    adapter_error = apply_effect_request(
        request("fs.open", root / "missing.txt"), grants
    )
    invalid_request = request("fs.open", note)
    invalid_request["args"] = {"path": None}
    invalid = apply_effect_request(invalid_request, grants)
    unsupported = apply_effect_request(request("fs.unknown", note), grants)
    report: dict[str, object] = {
        "schema": "pnix-meta.host-io-probe.v1",
        "adapter_error": adapter_error.get("error"),
        "path_exists": exists.get("value"),
        "missing_exists": missing.get("value"),
        "open": opened.get("value"),
        "file_type": typed.get("value"),
        "read_dir": listed.get("value"),
        "denied": denied.get("error"),
        "invalid": invalid.get("error"),
        "unsupported": unsupported.get("error"),
        "receipt_adapter": (opened.get("receipt") or {}).get("adapter"),
    }
    report["all_ok"] = (
        report["adapter_error"]
        == {"phase": "effect", "class": "effect-adapter-error"}
        and report["path_exists"] is True
        and report["missing_exists"] is False
        and report["open"] == "hello"
        and report["file_type"] == "regular"
        and report["read_dir"] == {"note.txt": "regular", "subdir": "directory"}
        and report["denied"] == {"phase": "effect", "class": "effect-denied"}
        and report["invalid"]
        == {"phase": "effect-contract", "class": "invalid-effect-args"}
        and report["unsupported"]
        == {"phase": "effect-contract", "class": "unknown-effect-operation"}
        and report["receipt_adapter"] == "host-meta-io-v1"
    )
    return report


def main() -> int:
    result = probe(Path(sys.argv[1]))
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0 if result["all_ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
