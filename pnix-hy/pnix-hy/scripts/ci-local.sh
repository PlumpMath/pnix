#!/usr/bin/env bash
# pnix-hy LOCAL CI — mirrors .github/workflows/pnix-hy-gate.yml, but solves the two things that
# broke it in GitHub Actions:
#   1) it runs from the pnix-hy PACKAGE ROOT (the sacred runtime self-tests use relative paths
#      like `readFile "todo.md"` / `readDir "."`, and todo.md lives here), and
#   2) it finds an ABSOLUTE Python that actually has Hy 1.3.0 for the projection / proof tiers.
# Run from anywhere:  bash pnix-hy/scripts/ci-local.sh   (or: make -C pnix-hy ci)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # .../pnix-hy (package root)
cd "$HERE"
export PYTHONPATH="$HERE"

find_hy_python() {
  local c
  if [ -n "${PNIX_HY_PYTHON:-}" ]; then
    c="${PNIX_HY_PYTHON}"
    if command -v "$c" >/dev/null 2>&1 && (cd "$HERE/.."; "$c" -c 'import hy; print(hy.__version__)') >/dev/null 2>&1; then
      command -v "$c"
      return 0
    fi
    echo "ERROR: PNIX_HY_PYTHON=$c does not provide working hy import" >&2
    return 1
  fi
  for c in "${PY:-}" python3 python; do
    [ -n "$c" ] || continue
    if command -v "$c" >/dev/null 2>&1 && (cd "$HERE/.."; "$c" -c 'import hy; print(hy.__version__)') >/dev/null 2>&1; then
      command -v "$c"; return 0
    fi
  done
  return 1
}

if ! PY="$(find_hy_python)"; then
  echo "ERROR: no Python with Hy 1.3.0 found." >&2
  echo "  set PNIX_HY_PYTHON=/path/to/python  (after: python -m pip install 'hy==1.3.0')" >&2
  echo "  or run inside 'nix develop' (which puts Hy 1.3.0 on PATH)." >&2
  exit 2
fi
export PNIX_HY_PYTHON="$PY"
echo "proof Python: $PY (Hy $("$PY" -c 'import hy;print(hy.__version__)'))"
echo "working dir : $HERE"

echo "### 1/4 syntax"
"$PY" -m py_compile pnix_hy/capabilities.py pnix_hy/cli.py

echo "### 2/4 --check (57 toolkit self-checks, incl docs_drift)"
"$PY" -m pnix_hy.cli --check || { echo "--- --check --json (diagnostics) ---"; "$PY" -m pnix_hy.cli --check --json; exit 1; }

echo "### 3/4 capability index is current (regenerate + no diff)"
"$PY" -m pnix_hy.cli --capabilities > docs/CAPABILITIES.md
git diff --exit-code docs/CAPABILITIES.md

echo "### 4/4 --gate (sacred lanes + toolkit)"
"$PY" -m pnix_hy.cli --gate

echo "LOCAL CI: PASS"
