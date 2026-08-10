#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir/.." rev-parse --show-toplevel 2>/dev/null || (cd "$script_dir/.." && pwd))"
cd "$repo_root"

license_db="${PNIX_LICENSE_DENY_DB:-data/license-deny/license-deny.sqlite}"

fail() {
  printf 'FAILED: %s\n' "$*" >&2
  exit 1
}

require_text() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if ! grep -Fq "$needle" "$file"; then
    fail "$label"
  fi
}

require_no_matches_file() {
  local label="$1"
  local tmp="$2"
  if [ -s "$tmp" ]; then
    printf 'FAILED: %s\n' "$label" >&2
    sed -n '1,120p' "$tmp" >&2
    rm -f "$tmp"
    exit 1
  fi
  rm -f "$tmp"
}

require_text CLAUDE.md "Legal/provenance pre-ingest gate (2026-06-13)" \
  "CLAUDE.md missing legal/provenance pre-ingest gate"
require_text CLAUDE.md "license-deny.sqlite" \
  "CLAUDE.md missing SQLite deny DB authority note"
require_text CLAUDE.md "DB hit = 완전 차단" \
  "CLAUDE.md missing no-held/local ingest rule"
require_text CLAUDE.md "anti-transcript intelligence law" \
  "CLAUDE.md missing anti-transcript intelligence law"
require_text problem-todo.md "pnix-owned parser/generator/solver/evaluator" \
  "problem-todo.md missing no-transcript algorithmic closure rule"

require_text stdlib/lib/gate/license-provenance-constitution-policy.px "licenseDenyDatabasePath" \
  "license provenance owner missing SQLite DB path"
require_text stdlib/lib/gate/license-provenance-constitution-policy.px "pnix.license-deny-db.v1" \
  "license provenance owner missing SQLite DB schema"
require_text stdlib/lib/gate/license-provenance-constitution-policy.px "denyTextNeedles = [ ];" \
  "license provenance owner still embeds text needles"
require_text stdlib/lib/gate/license-provenance-constitution-policy.px "Text scanning is delegated to the SQLite-backed preingest gate" \
  "license provenance owner missing SQLite delegation note"

require_text scripts/require-legal-provenance-gate.sh "sqlite3" \
  "preingest legal gate is not SQLite-backed"
require_text scripts/require-legal-provenance-gate.sh "deny_marker_patterns" \
  "preingest legal gate does not query marker table"
require_text scripts/require-legal-provenance-gate.sh "deny_text_needles" \
  "preingest legal gate does not query text needle table"
require_text scripts/all_gates.sh "check-license-provenance-constitution.sh" \
  "all_gates missing license provenance constitution audit"
require_text scripts/all_gates.sh "license-deep-scan-gate.sh" \
  "all_gates missing license deep scan gate"

require_text .gitignore "*.sqlite" \
  ".gitignore missing generic sqlite artifact ban"
require_text .gitignore "!data/license-deny/license-deny.sqlite" \
  ".gitignore missing policy DB allow exception"

if [ ! -f "$license_db" ]; then
  fail "SQLite deny DB missing: $license_db"
fi

python3 - "$license_db" <<'PY'
import re
import sqlite3
import sys

db_path = sys.argv[1]
con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
con.row_factory = sqlite3.Row

schema = con.execute("select value from meta where key='schema'").fetchone()
if not schema or schema["value"] != "pnix.license-deny-db.v1":
    raise SystemExit(f"bad SQLite deny DB schema: {schema['value'] if schema else '<missing>'}")

minimums = {
    "deny_flags": 700,
    "deny_marker_patterns": 500,
    "deny_text_needles": 5000,
}
for table, minimum in minimums.items():
    count = con.execute(f"select count(*) from {table} where enabled=1").fetchone()[0]
    if count < minimum:
        raise SystemExit(f"{table} enabled row count too low: {count} < {minimum}")

audit_flag_schema = con.execute(
    "select value from meta where key='deny_audit_required_flags_table'"
).fetchone()
if not audit_flag_schema or audit_flag_schema["value"] != "deny_audit_required_flags.v1":
    raise SystemExit("SQLite deny DB missing deny_audit_required_flags.v1 meta")
audit_smoke_schema = con.execute(
    "select value from meta where key='deny_audit_smoke_table'"
).fetchone()
if not audit_smoke_schema or audit_smoke_schema["value"] != "deny_audit_smoke.v1":
    raise SystemExit("SQLite deny DB missing deny_audit_smoke.v1 meta")
file_retention_schema = con.execute(
    "select value from meta where key='deny_file_retention_needles_table'"
).fetchone()
if not file_retention_schema or file_retention_schema["value"] != "deny_file_retention_needles.v1":
    raise SystemExit("SQLite deny DB missing deny_file_retention_needles.v1 meta")

required_flags = [
    row["name"]
    for row in con.execute(
        "select name from deny_audit_required_flags where enabled=1 order by name"
    )
]
if len(required_flags) < 10:
    raise SystemExit(f"deny_audit_required_flags enabled row count too low: {len(required_flags)}")
for name in required_flags:
    if not con.execute("select 1 from deny_flags where name=? and enabled=1", (name,)).fetchone():
        raise SystemExit(f"required deny flag missing from SQLite DB: {name}")

audit_rows = list(con.execute(
    """
    select label, flag_name, evidence_url_like, sample_text
    from deny_audit_smoke
    where enabled=1
    order by label
    """
))
if len(audit_rows) < 10:
    raise SystemExit(f"deny_audit_smoke enabled row count too low: {len(audit_rows)}")
for row in audit_rows:
    label = row["label"]
    flag_name = row["flag_name"]
    if not con.execute("select 1 from deny_marker_patterns where label=? and enabled=1", (label,)).fetchone():
        raise SystemExit(f"required deny marker missing from SQLite DB: {label}")
    if not con.execute("select 1 from deny_flags where name=? and enabled=1", (flag_name,)).fetchone():
        raise SystemExit(f"required deny flag missing from SQLite DB: {flag_name}")

if not con.execute("select 1 from sqlite_master where type='table' and name='deny_evidence'").fetchone():
    raise SystemExit("SQLite deny DB missing deny_evidence table")
for row in audit_rows:
    label = row["label"]
    evidence_url_like = row["evidence_url_like"]
    if evidence_url_like and not con.execute(
        "select 1 from deny_evidence where label=? and source_urls like ?",
        (label, f"%{evidence_url_like}%"),
    ).fetchone():
        raise SystemExit(f"required deny evidence missing from SQLite DB: {label}")

rows = list(con.execute("select label, pattern from deny_marker_patterns where enabled=1"))
compiled = []
for row in rows:
    try:
        compiled.append((row["label"], re.compile(row["pattern"])))
    except re.error as exc:
        raise SystemExit(f"bad deny marker regex {row['label']}: {exc}")

for row in audit_rows:
    text = row["sample_text"]
    expected = row["label"]
    hits = [label for label, pattern in compiled if pattern.search(text)]
    if expected not in hits:
        raise SystemExit(f"SQLite deny DB failed sample {expected}: {text}")
PY

tmp_receipt="$(mktemp)"
tmp_out="$(mktemp)"
cleanup() {
  rm -f "$tmp_receipt" "$tmp_out"
}
trap cleanup EXIT

smoke_row="$(sqlite3 -separator "$(printf '\t')" "$license_db" \
  "select sample_text, label from deny_audit_smoke where enabled=1 order by label limit 1")"
if [ -z "$smoke_row" ]; then
  fail "SQLite deny DB has no audit smoke sample"
fi
audit_sample="${smoke_row%%$'\t'*}"
audit_expected="${smoke_row#*$'\t'}"
if [ "$audit_sample" = "$audit_expected" ]; then
  fail "SQLite deny DB audit smoke sample is malformed"
fi

python3 - "$tmp_receipt" "$audit_sample" <<'PY'
import json
import sys

clean_flags = [
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
receipt = {name: True for name in clean_flags}
receipt.update({
    "source": "synthetic",
    "source_id": "pnix-owned-policy-gate-smoke",
    "notes": sys.argv[2],
})
for name in [
    "local_only",
    "eval_only",
    "license_deny_db_missing",
    "license_deny_db_invalid",
    "license_deny_db_rejected",
]:
    receipt[name] = False
open(sys.argv[1], "w", encoding="utf-8").write(json.dumps(receipt, sort_keys=True))
PY

if PNIX_LICENSE_DENY_DB="$license_db" bash scripts/require-legal-provenance-gate.sh db-negative-smoke "$tmp_receipt" >"$tmp_out" 2>&1; then
  cat "$tmp_out" >&2
  fail "SQLite-backed preingest gate accepted a blocked sample"
fi
if ! grep -Fq "$audit_expected" "$tmp_out"; then
  cat "$tmp_out" >&2
  fail "SQLite-backed preingest gate did not report expected marker"
fi

line_count="$(wc -l < scripts/check-license-provenance-constitution.sh | tr -d ' ')"
if [ "$line_count" -gt 380 ]; then
  fail "license provenance constitution audit grew past compact DB-backed form"
fi
line_count="$(wc -l < stdlib/lib/gate/license-provenance-constitution-policy.px | tr -d ' ')"
if [ "$line_count" -gt 160 ]; then
  fail "license provenance owner grew past compact DB reference form"
fi
line_count="$(wc -l < scripts/require-legal-provenance-gate.sh | tr -d ' ')"
if [ "$line_count" -gt 220 ]; then
  fail "preingest gate grew past compact DB query form"
fi

seto_generated_tmp="$(mktemp)"
if [ -d data/seto ]; then
  find data/seto -type f \
    \( -name 'docset_*' -o -name 'treesitter_*' -o -name '*docset*' \) \
    -print >"$seto_generated_tmp"
fi
require_no_matches_file "docset/tree-sitter generated SETO/rule artifacts still exist locally" "$seto_generated_tmp"

pdf_tmp="$(mktemp)"
if [ -d docs/outreach/pdf ]; then
  find docs/outreach/pdf -type f -iname '*.pdf' -print >"$pdf_tmp"
fi
require_no_matches_file "personal/outreach PDFs still exist locally" "$pdf_tmp"

db_tmp="$(mktemp)"
find . \( -path './.git' -o -path './target' -o -path './.pnix' \) -prune -o -type f \
  \( -iname '*.db' -o -iname '*.sqlite' -o -iname '*.sqlite3' -o -iname '*.redb' -o -iname '*.duckdb' \) \
  ! -path './data/license-deny/license-deny.sqlite' \
  -print >"$db_tmp"
require_no_matches_file "DB-like ingest/index artifacts still exist locally outside the policy deny DB" "$db_tmp"

schema_tmp="$(mktemp)"
find . \( -path './.git' -o -path './target' \) -prune -o -type f \
  \( -iname '*.xsd' -o -iname '*.dtd' -o -iname '*.mod' \) \
  -print >"$schema_tmp"
require_no_matches_file ".xsd/.dtd/.mod schema file found; external schema materialization is denied" "$schema_tmp"

python3 - "$license_db" <<'PY'
import json, pathlib, sqlite3, subprocess, sys
db_path = sys.argv[1]
con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
con.row_factory = sqlite3.Row

def fetch_needles(sql):
    return [row["needle"].lower() for row in con.execute(sql) if row["needle"]]

bad_paths = []
dict_needles = [n for n in fetch_needles("""
    select needle from deny_text_needles where enabled=1 and label in (
      select label from deny_evidence
      where label like '%dictionary%' or summary like '%dictionary%'
    )
""") if len(n) >= 4]
for root in [pathlib.Path("corpus/dictionary"), pathlib.Path(".puncheetah/dict-shards")]:
    if not root.exists():
        continue
    for path in root.rglob("*"):
        if any(needle in str(path).lower() for needle in dict_needles):
            bad_paths.append(str(path))
bad_manifests = []
for path in pathlib.Path("corpus/dictionary/LICENSES").glob("*.license.json"):
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        raise SystemExit(f"bad dictionary license manifest {path}: {exc}")
    if data.get("license_clean") is False or data.get("workspace_ingest_allowed") is False:
        bad_manifests.append(str(path))

bad_text = []
retention_needles = set(fetch_needles("""
    select needle from deny_text_needles
    where enabled=1 and source in (
      'round142_dictionary_deny_manifest_file_compaction_2026_06_15',
      'round144_file_to_sqlite_cleanup_2026_06_15',
      'round145_biomedical_license_false_positive_official_2026_06_15',
      'round146_dataset_hub_repository_official_evidence_2026_06_15'
    )
"""))
retention_needles.update(fetch_needles("""
    select needle from deny_file_retention_needles
    where enabled=1
"""))
tracked = subprocess.run(
    ["git", "ls-files"],
    check=True,
    stdout=subprocess.PIPE,
    text=True,
).stdout.splitlines()
skip_suffixes = (".sqlite", ".db", ".redb", ".duckdb", ".png", ".jpg", ".jpeg", ".gif", ".webp", ".pdf")
for raw_path in tracked:
    path = pathlib.Path(raw_path)
    if path == pathlib.Path("data/license-deny/license-deny.sqlite"):
        continue
    if raw_path.endswith(skip_suffixes):
        continue
    try:
        text = path.read_text(encoding="utf-8", errors="ignore").lower()
    except Exception:
        continue
    for needle in retention_needles:
        if needle and len(needle) >= 4 and needle in text:
            bad_text.append(f"{path}: SQLite file-retention needle matched")
            break
if bad_paths:
    print("denied dictionary shard/cache paths must not exist; matched SQLite deny DB needles:", file=sys.stderr)
    print("\n".join(sorted(bad_paths)[:120]), file=sys.stderr)
    raise SystemExit(1)
if bad_manifests:
    print("deny-only dictionary license manifests must live in SQLite DB, not files:", file=sys.stderr)
    print("\n".join(bad_manifests), file=sys.stderr)
    raise SystemExit(1)
if bad_text:
    print("external problem/web-search retained text found via SQLite deny DB needles:", file=sys.stderr)
    print("\n".join(sorted(bad_text)[:120]), file=sys.stderr)
    raise SystemExit(1)
PY

printf 'SUCCESS ALL OK\n'
