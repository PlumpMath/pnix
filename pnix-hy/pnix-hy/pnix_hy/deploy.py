"""pnix_hy.deploy -- deployment introspection (additive, diagnostic-only).

Answers "where is pnix_hy installed, and which capability tiers work HERE?" without changing any
behavior. Core (pnix eval / safe_eval / gate / action / ir / witness / explain) always works as a
plain `import pnix_hy`. Projection / proof tiers additionally need Hy 1.3.0 + the hy-meta tree at
`HY_ROOT` (in-repo/flake by default; `PNIX_HY_HOME` points an off-tree install at a checkout).
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Any

from . import hy_mirror as hm


def _probe_projection_python(py: str) -> bool:
    try:
        proc = subprocess.run(
            [py, "-c", "import hy"],  # noqa: S603 - subprocess is internal, no shell injection
            cwd=str(hm.HY_ROOT),
            text=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=15,
        )
    except Exception:  # noqa: BLE001
        return False
    return proc.returncode == 0


def deployment_info() -> dict[str, Any]:
    """Report the resolved paths + which tiers (core / projection / full_gate) are available."""
    hy_root = hm.HY_ROOT
    hy_meta_ok = (hy_root / "hy-meta" / "host_introspect.py").exists()
    vendored_hy_ok = (hy_root / "hy").exists() or (hy_root / "hy-1.3.0").exists()
    proof_python: str | None = None
    proof_ok = False
    try:
        proof_python = str(hm.hy_python())
        proof_ok = _probe_projection_python(proof_python)
    except Exception:  # noqa: BLE001 - no Hy proof Python available here
        proof_ok = False
    projection_ok = bool(hy_meta_ok and proof_ok)
    return {
        "schema": "pnix-hy.deployment.v0",
        "package_path": str(Path(__file__).resolve().parent),
        "pnix_hy_home_env": os.environ.get("PNIX_HY_HOME"),
        "hy_root": str(hy_root),
        "hy_meta_found": hy_meta_ok,
        "vendored_hy_found": vendored_hy_ok,
        "proof_python": proof_python,
        "proof_python_found": proof_ok,
        "tiers": {
            "core": True,                 # pnix eval/safe_eval/gate/action/ir/witness/explain — always
            "projection": projection_ok,  # Hy<->pnix projection, mirror-over-hy, reports needing Hy
            "full_gate": projection_ok,   # --gate (stage7 kernel) needs Hy + the tree
        },
        "hint": None if projection_ok else (
            "core works as-is. For projection/proof tiers: install 'pnix-hy[projection]' (Hy 1.3.0) "
            "and set PNIX_HY_HOME=/path/to/pnix-hy checkout (with hy-meta/ + hy), or use `nix develop`."
        ),
    }
