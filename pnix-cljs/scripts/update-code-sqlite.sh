#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/code/sqlite"
mkdir -p "$OUT_DIR"
python3 - "$OUT_DIR" <<'PY'
import json, sqlite3, subprocess, sys
from pathlib import Path
out=Path(sys.argv[1])
con=sqlite3.connect(':memory:')
version=sqlite3.sqlite_version
compile_options=[r[0] for r in con.execute('pragma compile_options')]
functions=[]
try:
    for row in con.execute('pragma function_list'):
        functions.append({'name':row[0], 'builtin':row[1], 'type':row[2], 'encoding':row[3], 'narg':row[4], 'flags':row[5]})
except Exception:
    pass
keywords=[]
try:
    p=subprocess.run(['sqlite3','-batch',':memory:','.help'], text=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
except Exception:
    p=None
# Static keyword list from SQLite docs/public-domain grammar tokens; kept here as structure, not prose.
keywords = '''ABORT ACTION ADD AFTER ALL ALTER ALWAYS ANALYZE AND AS ASC ATTACH AUTOINCREMENT BEFORE BEGIN BETWEEN BY CASCADE CASE CAST CHECK COLLATE COLUMN COMMIT CONFLICT CONSTRAINT CREATE CROSS CURRENT CURRENT_DATE CURRENT_TIME CURRENT_TIMESTAMP DATABASE DEFAULT DEFERRABLE DEFERRED DELETE DESC DETACH DISTINCT DO DROP EACH ELSE END ESCAPE EXCEPT EXCLUDE EXCLUSIVE EXISTS EXPLAIN FAIL FILTER FIRST FOLLOWING FOR FOREIGN FROM FULL GENERATED GLOB GROUP GROUPS HAVING IF IGNORE IMMEDIATE IN INDEX INDEXED INITIALLY INNER INSERT INSTEAD INTERSECT INTO IS ISNULL JOIN KEY LAST LEFT LIKE LIMIT MATCH MATERIALIZED NATURAL NO NOT NOTHING NOTNULL NULL NULLS OF OFFSET ON OR ORDER OTHERS OUTER OVER PARTITION PLAN PRAGMA PRECEDING PRIMARY QUERY RAISE RANGE RECURSIVE REFERENCES REGEXP REINDEX RELEASE RENAME REPLACE RESTRICT RETURNING RIGHT ROLLBACK ROW ROWS SAVEPOINT SELECT SET TABLE TEMP TEMPORARY THEN TIES TO TRANSACTION TRIGGER UNBOUNDED UNION UNIQUE UPDATE USING VACUUM VALUES VIEW VIRTUAL WHEN WHERE WINDOW WITH WITHOUT'''.split()
value={'sqlite_version':version, 'compile_options':compile_options, 'functions':functions, 'keywords':keywords}
(out/'sqlite-runtime-metadata.json').write_text(json.dumps(value, indent=2, sort_keys=True), encoding='utf-8')
print('wrote', out/'sqlite-runtime-metadata.json')
PY
shasum -a 256 "$OUT_DIR/sqlite-runtime-metadata.json" > "$OUT_DIR/sqlite-runtime-metadata.json.sha256"
{
  echo "source_url=https://sqlite.org/"
  echo "runtime_sqlite_version=$(python3 - <<'PY'
import sqlite3; print(sqlite3.sqlite_version)
PY
)"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$OUT_DIR/sqlite-runtime-metadata.json.sha256")"
} > "$OUT_DIR/sqlite-runtime-metadata.json.meta"
cat "$OUT_DIR/sqlite-runtime-metadata.json.sha256"
