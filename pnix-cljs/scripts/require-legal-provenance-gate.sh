#!/usr/bin/env bash
set -euo pipefail

pnix_require_legal_provenance_gate() {
  local caller="${1:-unknown-ingest}"
  local receipt="${2:-${PNIX_LEGAL_PROVENANCE_RECEIPT:-}}"
  local script_dir repo_root deny_db
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  repo_root="$(git -C "$script_dir/.." rev-parse --show-toplevel 2>/dev/null || (cd "$script_dir/.." && pwd))"
  deny_db="${PNIX_LICENSE_DENY_DB:-$repo_root/data/license-deny/license-deny.sqlite}"

  if [[ -z "$receipt" ]]; then
    echo "preingest-legal-provenance-reject-missing: $caller requires PNIX_LEGAL_PROVENANCE_RECEIPT" >&2
    return 2
  fi
  if [[ ! -f "$receipt" ]]; then
    echo "preingest-legal-provenance-reject-receipt-not-found: $receipt" >&2
    return 2
  fi
  if [[ ! -f "$deny_db" ]]; then
    echo "preingest-legal-provenance-reject-deny-db-missing: $deny_db" >&2
    return 2
  fi

  python3 - "$caller" "$receipt" "$deny_db" <<'PY'
import json
import re
import sqlite3
import sys
from pathlib import Path

import os

caller = sys.argv[1]
receipt_path = Path(sys.argv[2])
db_path = Path(sys.argv[3])

try:
    data = json.loads(receipt_path.read_text(encoding="utf-8"))
except Exception as exc:
    print(f"preingest-legal-provenance-reject-invalid-json: {receipt_path}: {exc}", file=sys.stderr)
    sys.exit(2)

required_true = [
    "license_clean",
    "commercial_use_allowed",
    "redistribution_allowed",
    "derivative_allowed",
    "modification_allowed",
    "ai_training_allowed",
    "workspace_ingest_allowed",
    "local_retention_allowed",
    "eval_use_allowed",
    "grounding_use_allowed",
    "packager_license_ok",
    "underlying_content_rights_ok",
    "access_path_terms_ok",
    "jurisdiction_ok",
    "no_third_party_rights_gap",
    "no_privacy_or_publicity_risk",
    "provenance_deep_audited",
]


def _is_truthy(value: str) -> bool:
    return value.strip().lower() in {"1", "true", "yes", "on"}


def _normalize_dictionary_source(value: str) -> str:
    return re.sub(r"[^0-9a-z가-힣]+", "", value.strip().lower())


DICTIONARY_FACTS_ONLY_CALLERS = {
    "dictionary-woorimalsaem-ingest",
    "dictionary-stardict-ingest",
    "dictionary-stdict-ingest",
}
DICTIONARY_FACTS_ONLY_ALLOWED_SOURCES = {
    _normalize_dictionary_source(v)
    for v in {
        "woorimalsaem",
        "우리말샘",
        "stdict",
        "stardict",
        "krdict",
        "표준국어대사전",
        "표준국어사전",
        "우리말사전",
    }
}
DICTIONARY_DENY_FLAG = "korean_cc_by_sa_dictionary_source"
DICTIONARY_EXCEPTION_ENV = "PNIX_DICTIONARY_ALLOW_SHARE_ALIKE"


def _is_dictionary_facts_only_allowed(caller: str) -> bool:
    if caller not in DICTIONARY_FACTS_ONLY_CALLERS:
        return False
    if not _is_truthy(os.environ.get("PNIX_DICTIONARY_FACTS_ONLY", "0")):
        return False
    source = _normalize_dictionary_source(
        os.environ.get("PNIX_DICTIONARY_FACTS_ONLY_SOURCE", "")
    )
    return source in DICTIONARY_FACTS_ONLY_ALLOWED_SOURCES


def _is_dictionary_full_allowed(caller: str) -> bool:
    if caller not in DICTIONARY_FACTS_ONLY_CALLERS:
        return False
    if not _is_truthy(os.environ.get(DICTIONARY_EXCEPTION_ENV, "0")):
        return False
    source = _normalize_dictionary_source(
        os.environ.get("PNIX_DICTIONARY_FACTS_ONLY_SOURCE", "")
    )
    return source in DICTIONARY_FACTS_ONLY_ALLOWED_SOURCES


def _is_dictionary_marker(label: str) -> bool:
    lowered = str(label).lower()
    return (
        "korean" in lowered
        and "cc-by-sa" in lowered
        and "dictionary" in lowered
    )


def walk_strings(value, path="$"):
    if isinstance(value, str):
        yield path, value
    elif isinstance(value, dict):
        for key, child in value.items():
            yield from walk_strings(child, f"{path}.{key}")
    elif isinstance(value, list):
        for idx, child in enumerate(value):
            yield from walk_strings(child, f"{path}[{idx}]")

try:
    con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    schema = con.execute("select value from meta where key='schema'").fetchone()
    if not schema or schema["value"] != "pnix.license-deny-db.v1":
        raise RuntimeError(f"unexpected schema {schema['value'] if schema else '<missing>'}")
    bad_flags = [
        row["name"]
        for row in con.execute("select name from deny_flags where enabled=1 order by name")
    ]
    marker_rows = list(con.execute(
        "select label, pattern from deny_marker_patterns where enabled=1 order by label"
    ))
    needle_rows = list(con.execute(
        "select label, needle from deny_text_needles where enabled=1 order by needle"
    ))
except Exception as exc:
    print(f"preingest-legal-provenance-reject-deny-db-invalid({caller}): {db_path}: {exc}", file=sys.stderr)
    sys.exit(2)
finally:
    try:
        con.close()
    except Exception:
        pass

dictionary_facts_only = _is_dictionary_facts_only_allowed(caller)
dictionary_full_allowed = _is_dictionary_full_allowed(caller)
dictionary_gate_exception = dictionary_facts_only or dictionary_full_allowed

missing = [name for name in required_true if data.get(name) is not True]
if dictionary_gate_exception:
    bad = [
        name
        for name in bad_flags
        if name != DICTIONARY_DENY_FLAG and data.get(name) not in (False, None)
    ]
else:
    bad = [name for name in bad_flags if data.get(name) not in (False, None)]

compiled = []
try:
    for row in marker_rows:
        label = row["label"]
        if dictionary_gate_exception and _is_dictionary_marker(label):
            continue
        compiled.append((label, re.compile(row["pattern"])))
except Exception as exc:
    print(f"preingest-legal-provenance-reject-deny-db-invalid({caller}): regex compile failed: {exc}", file=sys.stderr)
    sys.exit(2)

marker_hits = []
strings = list(walk_strings(data))
for path_label, text in strings:
    for label, pattern in compiled:
        if pattern.search(text):
            marker_hits.append(f"{label}@{path_label}")

for path_label, text in strings:
    folded = text.lower()
    for row in needle_rows:
        if dictionary_gate_exception and _is_dictionary_marker(row["label"]):
            continue
        needle = row["needle"]
        if needle and needle.lower() in folded:
            marker_hits.append(f"{row['label']}:{needle}@{path_label}")

if missing or bad or marker_hits:
    if missing:
        print(f"preingest-legal-provenance-reject-missing-clean-flags({caller}): {', '.join(missing)}", file=sys.stderr)
    if bad:
        print(f"preingest-legal-provenance-reject-deny-flags({caller}): {', '.join(bad)}", file=sys.stderr)
    if marker_hits:
        print(f"preingest-legal-provenance-reject-deny-markers({caller}): {', '.join(marker_hits[:24])}", file=sys.stderr)
    sys.exit(2)

print(f"legal-provenance-ok: {caller}: {receipt_path}", file=sys.stderr)
PY
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  pnix_require_legal_provenance_gate "${1:-manual-ingest}" "${2:-${PNIX_LEGAL_PROVENANCE_RECEIPT:-}}"
fi
