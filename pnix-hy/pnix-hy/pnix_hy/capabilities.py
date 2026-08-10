"""pnix_hy.capabilities -- docs-as-code capability index + doc<->code drift gate (proposal 0011).

Single source of truth = the CODE. This module DERIVES a capability index from the code
(`pnix_hy.__all__` public API + the `_toolkit_reports()` self-check registry + the proposal
files' status) so "what exists / who owns it / where / which proposal" cannot drift from reality.
`docs/CAPABILITIES.md` is GENERATED from `render_capabilities_md()` (do not hand-edit); the
`docs_drift_report()` self-check fails if the committed file is stale, if a public symbol does not
import, or if a `[[wikilink]]` in the docs does not resolve. Use `--capabilities | grep X` before
building anything -- that is the anti-duplicate-development lookup.

Additive/diagnostic only: no evaluator/mirror/gate, no runtime behavior, `pnix_runtime.py`
untouched. All heavy imports are lazy so `import pnix_hy` stays dependency-free.
"""

from __future__ import annotations

import inspect
import os
import re
import types
from pathlib import Path
from typing import Any

_CWD = Path(__file__).resolve().parents[1]               # fallback: installed package root


def _resolve_pkg_root() -> Path:
    """Resolve package/docs root from installation, honoring PNIX_HY_HOME when set."""
    env_home = os.environ.get("PNIX_HY_HOME")
    if env_home:
        root = Path(env_home).expanduser().resolve()
        for candidate in (root / "pnix-hy", root):
            if candidate.is_dir() and (candidate / "pnix_hy").is_dir() and (candidate / "docs").is_dir():
                return candidate
    return _CWD


_PKG_ROOT = _resolve_pkg_root()
_DOCS_DIR = _PKG_ROOT / "docs"
_PROPOSALS_DIR = _DOCS_DIR / "proposals"
_CAPABILITIES_MD = _DOCS_DIR / "CAPABILITIES.md"
_REPO_ROOT = _PKG_ROOT.parent if _PKG_ROOT.name == "pnix-hy" else _PKG_ROOT

_LANE = {
    "pnix_hy.interop": "interop (boundary)",
    "pnix_hy.hy_mirror": "pnix-hy <-> hy-meta (projection)",
    "pnix_hy.pnix_runtime": "pnix-hy (runtime, sacred)",
}
_WIKILINK = re.compile(r"\[\[([^\]|]+)(?:\|[^\]]*)?\]\]")


def _lane_for(module: str) -> str:
    return _LANE.get(module, "pnix-hy")


def _kind_of(obj: Any) -> str:
    if isinstance(obj, types.ModuleType):
        return "module"
    if inspect.isclass(obj):
        return "class"
    if callable(obj):
        return "function"
    return "value"


def _summary_of(obj: Any) -> str:
    doc = inspect.getdoc(obj) if not isinstance(obj, types.ModuleType) else (obj.__doc__ or "")
    type_doc = inspect.getdoc(type(obj)) if not isinstance(obj, types.ModuleType) else ""
    if not doc or doc == type_doc:
        return ""
    return doc.strip().splitlines()[0].strip()


def _owning_module(obj: Any, name: str) -> str:
    # First, trust explicit module metadata when it points into pnix_hy.
    module = getattr(obj, "__module__", "") or ""
    if module.startswith("pnix_hy"):
        return module

    # Fall back to identity scan across pnix_hy SUBmodules; the package itself re-exports every
    # public symbol, so including bare "pnix_hy" would always match first and defeat the scan.
    pkg = Path(__file__).resolve().parent
    modules: list[str] = []
    if pkg.is_dir():
        for child in pkg.iterdir():
            if child.name.startswith("_"):
                continue
            if child.is_file() and child.suffix == ".py":
                modules.append(f"pnix_hy.{child.stem}")
            elif child.is_dir() and (child / "__init__.py").is_file():
                modules.append(f"pnix_hy.{child.name}")
    for mod_name in sorted(set(modules)):
        try:
            mod = __import__(mod_name, fromlist=["*"])
        except Exception:
            continue
        if getattr(mod, name, None) is obj:
            return mod_name
    return module or "pnix_hy"


def _public_symbols() -> list[dict[str, Any]]:
    import pnix_hy as ph  # noqa: PLC0415 - lazy to avoid import cycle

    rows: list[dict[str, Any]] = []
    for name in ph.__all__:
        if name == "__version__":
            rows.append({"name": name, "kind": "value", "module": "pnix_hy",
                         "lane": "pnix-hy", "summary": f"package version ({ph.__version__})"})
            continue
        obj = getattr(ph, name, None)
        if obj is None:
            rows.append({"name": name, "kind": "MISSING", "module": "", "lane": "",
                         "summary": "__all__ entry does not resolve"})
            continue
        module = _owning_module(obj, name)
        rows.append({"name": name, "kind": _kind_of(obj), "module": module,
                     "lane": _lane_for(module), "summary": _summary_of(obj)})
    rows.sort(key=lambda r: r["name"])
    return rows


def _reports() -> list[dict[str, Any]]:
    from pnix_hy import cli  # noqa: PLC0415 - lazy to avoid import cycle

    rows: list[dict[str, Any]] = []
    for name, fn in cli._toolkit_reports().items():
        rows.append({"name": name,
                     "module": getattr(fn, "__module__", ""),
                     "symbol": getattr(fn, "__qualname__", getattr(fn, "__name__", ""))})
    rows.sort(key=lambda r: r["name"])
    return rows


def _proposals() -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not _PROPOSALS_DIR.is_dir():
        return rows
    for path in sorted(_PROPOSALS_DIR.glob("[0-9]*.md")):
        head = path.read_text(encoding="utf-8").splitlines()
        title = next((ln.lstrip("# ").strip() for ln in head if ln.startswith("#")), path.stem)
        status = "?"
        for ln in head[:8]:
            m = re.search(r"\b(SHIPPED|ACCEPTED|DECLINED|candidate|후보)\b", ln)
            if m:
                status = m.group(1)
                break
        rows.append({"id": path.stem.split("-")[0], "title": title,
                     "status": status, "path": f"docs/proposals/{path.name}"})
    return rows


def capability_index() -> dict[str, Any]:
    """Derive the capability index from code (public API + report registry + proposals)."""
    symbols = _public_symbols()
    reports = _reports()
    proposals = _proposals()
    return {
        "schema": "pnix-hy.capabilities.v0",
        "symbols": symbols,
        "reports": reports,
        "proposals": proposals,
        "counts": {"symbols": len(symbols), "reports": len(reports), "proposals": len(proposals)},
    }


def render_capabilities_md(index: dict[str, Any] | None = None) -> str:
    """Render the index as the canonical `docs/CAPABILITIES.md` (deterministic; do not hand-edit)."""
    idx = index or capability_index()
    out: list[str] = []
    out.append("# CAPABILITIES — 생성물 (하드편집 금지 / GENERATED — do not hand-edit)")
    out.append("")
    out.append("`pnix-hy-project --capabilities`(=`pnix_hy.capabilities.render_capabilities_md`)로")
    out.append("코드에서 파생됩니다. 진실의 원천 = 코드. 중복개발 전에 여기서 `grep` 하세요.")
    out.append("재생성: `pnix-hy-project --capabilities > docs/CAPABILITIES.md` (proposal 0011).")
    out.append("핵심 basic 예: [[safe_eval]] · [[explain_pnix]].")
    out.append("명시적 proof/service API: `pnix_hy.proof.check_action` · "
               "`pnix_hy.deploy.deployment_info` (basic top-level export 아님).")
    out.append("")
    c = idx["counts"]
    out.append(f"counts: symbols={c['symbols']} · reports={c['reports']} · proposals={c['proposals']}")
    out.append("")
    out.append("## Public API (`pnix_hy.__all__`)")
    out.append("")
    out.append("| name | kind | lane | module | summary |")
    out.append("|---|---|---|---|---|")
    for r in idx["symbols"]:
        summ = (r["summary"] or "").replace("|", "\\|")
        out.append(f"| `{r['name']}` | {r['kind']} | {r['lane']} | `{r['module']}` | {summ} |")
    out.append("")
    out.append("## Self-check reports (`--check`)")
    out.append("")
    out.append("| report | module | symbol |")
    out.append("|---|---|---|")
    for r in idx["reports"]:
        out.append(f"| `{r['name']}` | `{r['module']}` | `{r['symbol']}` |")
    out.append("")
    out.append("## Proposals (`docs/proposals/`)")
    out.append("")
    out.append("| id | status | title | path |")
    out.append("|---|---|---|---|")
    for r in idx["proposals"]:
        title = (r["title"] or "").replace("|", "\\|")
        out.append(f"| {r['id']} | {r['status']} | {title} | `{r['path']}` |")
    out.append("")
    return "\n".join(out) + "\n"


def write_capabilities_md() -> Path:
    """Regenerate the committed `docs/CAPABILITIES.md` from code."""
    _CAPABILITIES_MD.write_text(render_capabilities_md(), encoding="utf-8")
    return _CAPABILITIES_MD


def _known_names(idx: dict[str, Any]) -> set[str]:
    names = {r["name"] for r in idx["symbols"]}
    names |= {r["name"] for r in idx["reports"]}
    names |= {r["id"] for r in idx["proposals"]}
    return names


def _scan_docs() -> list[Path]:
    docs: list[Path] = []
    if _DOCS_DIR.is_dir():
        docs += sorted(_DOCS_DIR.rglob("*.md"))
    for extra in (_PKG_ROOT / "README.md", _PKG_ROOT / "todo.md",
                  _REPO_ROOT / "README.md", _REPO_ROOT / "SCOPE_LOCK.md"):
        if extra.is_file():
            docs.append(extra)
    return docs


def docs_drift_report() -> dict[str, Any]:
    """Doc<->code drift gate: generated index is current, public API imports, wikilinks resolve."""
    if not _DOCS_DIR.is_dir():
        return {"schema": "pnix-hy.docs-drift.v0", "ready": True, "available": False,
                "note": "docs tree absent (off-tree/core install) -- drift gate not applicable"}
    try:
        idx = capability_index()
        missing_api = [r["name"] for r in idx["symbols"] if r["kind"] == "MISSING"]

        rendered = render_capabilities_md(idx)
        md_exists = _CAPABILITIES_MD.is_file()
        md_current = md_exists and _CAPABILITIES_MD.read_text(encoding="utf-8") == rendered

        known = _known_names(idx)
        doc_stems = {p.stem for p in _scan_docs()}
        unresolved: list[str] = []
        link_count = 0
        for path in _scan_docs():
            for tok in _WIKILINK.findall(path.read_text(encoding="utf-8")):
                target = tok.strip()
                # only symbol-SHAPED tokens are wikilinks; prose examples ([[...]], [[<name>]]),
                # POSIX classes ([[:space:]]), and non-ASCII placeholders are ignored, not flagged.
                if not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_-]*", target):
                    continue
                link_count += 1
                if target in known or target in doc_stems:
                    continue
                unresolved.append(f"{path.name}:[[{target}]]")

        ready = (not missing_api) and md_current and (not unresolved)
        return {
            "schema": "pnix-hy.docs-drift.v0",
            "ready": bool(ready),
            "available": True,
            "counts": idx["counts"],
            "public_api_importable": not missing_api,
            "missing_api": missing_api,
            "capabilities_md_exists": md_exists,
            "capabilities_md_current": md_current,
            "wikilinks_seen": link_count,
            "wikilinks_unresolved": unresolved,
            "hint": None if ready else
                    "run `pnix-hy-project --capabilities > docs/CAPABILITIES.md`; fix MISSING api / [[links]].",
        }
    except Exception as exc:  # noqa: BLE001
        return {"schema": "pnix-hy.docs-drift.v0", "ready": False, "available": False,
                "error": f"{type(exc).__name__}: {exc}"}


__all__ = [
    "capability_index",
    "render_capabilities_md",
    "write_capabilities_md",
    "docs_drift_report",
]
