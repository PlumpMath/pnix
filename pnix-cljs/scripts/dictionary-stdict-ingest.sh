#!/usr/bin/env bash
# Standard Korean dictionary ingest (표준국어대사전) from stdict JSON dumps.
# Source: ingest/dictionary/stdict/*.json (JSON dumps from 국립국어원 standard dictionary portal).
set -euo pipefail

REPO_ROOT="${PNIX_WORKSPACE_ROOT:-$(git -C "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/.." rev-parse --show-toplevel 2>/dev/null || pwd)}"
source "${REPO_ROOT}/scripts/require-legal-provenance-gate.sh"

export PNIX_DICTIONARY_FACTS_ONLY_SOURCE="stdict"
export PNIX_DICTIONARY_ALLOW_SHARE_ALIKE=1
FACTS_ONLY="${PNIX_DICTIONARY_FACTS_ONLY:-0}"

for arg in "$@"; do
  case "$arg" in
    --facts-only)
      FACTS_ONLY=1
      ;;
    --full)
      FACTS_ONLY=0
      ;;
    --help|-h)
      echo "usage: $(basename "$0") [--facts-only|--full]" >&2
      echo "  --facts-only: definitions omitted, facts-only ingest mode" >&2
      echo "  --full:       keep definition text (not facts-only)" >&2
      echo "  env PNIX_STDICT_ORIGINAL_LANGUAGE_FACTS=0 disables original_language_info rec{} materialization" >&2
      echo "  env PNIX_STDICT_ORIGINAL_LANGUAGE_SCOPE=all stores all origin clues; default foreign-core" >&2
      echo "  env PNIX_STDICT_ORIGINAL_LANGUAGE_LIMIT=0 removes the default 500-record safety cap; full sets need optimized bulk ingest" >&2
      echo "  examples:" >&2
      echo "    $(basename "$0") --facts-only" >&2
      echo "    $(basename "$0") --full" >&2
      exit 0
      ;;
    *)
      echo "warning: ignoring unknown arg: $arg" >&2
      ;;
  esac
done

export PNIX_DICTIONARY_FACTS_ONLY="$FACTS_ONLY"

pnix_require_legal_provenance_gate "dictionary-stdict-ingest" "${PNIX_LEGAL_PROVENANCE_RECEIPT:-}"

PNIXC_META_BIN="${PNIXC_META_BIN:-pnixc-meta}"
DICTIONARY_DIR="${DICTIONARY_STDICT_DIR:-${REPO_ROOT}/ingest/dictionary/stdict}"
MANIFEST_PATH="${DICTIONARY_STDICT_LICENSE_MANIFEST:-${REPO_ROOT}/corpus/dictionary/LICENSES/stdict.license.json}"
REDB_PATH="${PNIXC_META_REDB_PATH:-/tmp/pnix-nl-corpus-demo.redb}"
SHARDS_DIR="${DICTIONARY_STDICT_SHARDS_DIR:-/tmp/pnix-dictionary-stdict-shards}"
SOURCE_ID="${DICTIONARY_STDICT_SOURCE_ID:-stdict}"
DOMAIN="${DICTIONARY_STDICT_DOMAIN:-}"
MAX_ROWS_PER_SHARD="${PNIX_DICTIONARY_MAX_ROWS_PER_SHARD:-${DICTIONARY_MAX_ROWS_PER_SHARD:-2000}}"

if [[ ! -d "$DICTIONARY_DIR" ]]; then
  echo "preingest-legal-provenance-reject-dictionary-dir-missing: DICTIONARY_STDICT_DIR is not set or not a directory ($DICTIONARY_DIR)" >&2
  exit 2
fi

if [[ ! -f "$MANIFEST_PATH" ]]; then
  echo "preingest-legal-provenance-reject-license-manifest-missing: $MANIFEST_PATH" >&2
  exit 2
fi

if [[ ! -x "$(command -v "$PNIXC_META_BIN")" ]]; then
  echo "error: pnixc-meta not found: $PNIXC_META_BIN" >&2
  exit 2
fi

if ! [[ "$MAX_ROWS_PER_SHARD" =~ ^[0-9]+$ ]] || [[ "$MAX_ROWS_PER_SHARD" -le 0 ]]; then
  echo "preingest-stdict-invalid-rows-per-shard: MAX_ROWS_PER_SHARD must be a positive integer, got '$MAX_ROWS_PER_SHARD'" >&2
  exit 2
fi

mkdir -p "$SHARDS_DIR"
rm -rf "$SHARDS_DIR"
mkdir -p "$SHARDS_DIR"

python3 - "$DICTIONARY_DIR" "$MANIFEST_PATH" "$SHARDS_DIR" "$SOURCE_ID" "$DOMAIN" "$MAX_ROWS_PER_SHARD" <<'PY'
import hashlib
import json
import os
import sys
from collections import defaultdict
from pathlib import Path


def normalize(value):
  return (value or "").strip()


def parse_manifest(path):
  raw = Path(path).read_text(encoding="utf-8")
  manifest = json.loads(raw)
  if manifest.get("schema") != "pnixc-meta.dictionary-license-manifest.v0":
    raise RuntimeError(f"license manifest schema mismatch: {manifest.get('schema')}")
  required_missing = [
    key
    for key in ("license_id", "attribution", "retrieved_at", "dump_file_digest_sha256")
    if not normalize(manifest.get(key))
  ]
  if required_missing:
    raise RuntimeError(f"license manifest missing required fields: {', '.join(required_missing)}")
  fill = "FILL-ME"
  for key in ("license_id", "attribution", "retrieved_at", "dump_file_digest_sha256"):
    if fill in normalize(manifest.get(key)):
      raise RuntimeError(f"license manifest field still placeholder: {key}")
  if str(manifest.get("operator_confirmed", False)).lower() not in ("true", "1"):
    raise RuntimeError("license manifest operator_confirmed must be true")
  return manifest["license_id"]


def chosung_bucket(lemma):
  if not lemma:
    return "etc"
  first = lemma[0]
  code = ord(first)
  if 0xAC00 <= code <= 0xD7A3:
    return f"cho-{((code - 0xAC00) // 588):02d}"
  return "etc"


def row_bucket(lemma, domain):
  domain = normalize(domain) or "general"
  return f"{domain}/{chosung_bucket(lemma)}"


def chunk(iterable, size):
  for i in range(0, len(iterable), size):
    yield iterable[i : i + size]


def sharded_item_paths(shards_dir, bucket, page):
  rel = f"{bucket}/{page:05}.jsonl"
  return rel, Path(shards_dir) / rel


def build_shards(dictionary_dir, manifest_path, shards_dir, source_id, domain, max_rows):
  license_ref = parse_manifest(manifest_path)

  bucket_rows = defaultdict(list)
  json_files = sorted([p for p in Path(dictionary_dir).glob("*.json") if p.is_file()])
  if not json_files:
    raise RuntimeError(f"no json files found in {dictionary_dir}")

  entries_seen = 0
  held_missing_lemma = 0
  held_missing_definition = 0
  total_rows = 0

  for path in json_files:
    raw = json.loads(path.read_text(encoding="utf-8"))
    channel = raw.get("channel") if isinstance(raw, dict) else None
    items = []
    if isinstance(channel, dict):
      items = channel.get("item", [])
    elif isinstance(raw, dict):
      items = raw.get("item", [])
    if not isinstance(items, list):
      continue

    for item in items:
      entries_seen += 1
      word_info = item.get("word_info") if isinstance(item, dict) else None
      if not isinstance(word_info, dict):
        held_missing_lemma += 1
        continue

      lemma = normalize(word_info.get("word"))
      if not lemma:
        held_missing_lemma += 1
        continue

      pos_infos = word_info.get("pos_info", [])
      if isinstance(pos_infos, dict):
        pos_infos = [pos_infos]
      if not isinstance(pos_infos, list):
        pos_infos = []

      if not pos_infos:
        held_missing_definition += 1
        continue

      for pos_info in pos_infos:
        if not isinstance(pos_info, dict):
          continue
        pos = normalize(pos_info.get("pos"))
        sense_infos = pos_info.get("sense_info", [])
        if isinstance(sense_infos, dict):
          sense_infos = [sense_infos]
        if not isinstance(sense_infos, list):
          continue

        for sense in sense_infos:
          if not isinstance(sense, dict):
            continue
          definition = normalize(sense.get("definition"))
          if not definition:
            definition = normalize(sense.get("definition_original"))
          if not definition:
            held_missing_definition += 1
            continue
          bucket_rows[row_bucket(lemma, domain)].append(
            {
              "lemma": lemma,
              "pos": pos,
              "definition_surface": definition,
              "source_id": source_id,
              "license_ref": license_ref,
              "domain": normalize(domain),
            }
          )
          total_rows += 1

  if total_rows == 0:
    raise RuntimeError(f"no usable rows from stdict corpus at {dictionary_dir}; entries_seen={entries_seen}, held_missing_lemma={held_missing_lemma}, held_missing_definition={held_missing_definition}")

  shards = []
  for bucket in sorted(bucket_rows):
    rows = bucket_rows[bucket]
    for page, page_rows in enumerate(chunk(rows, max_rows)):
      rel_path, abs_path = sharded_item_paths(shards_dir, bucket, page)
      abs_path.parent.mkdir(parents=True, exist_ok=True)
      body_lines = [json.dumps(row, ensure_ascii=False) for row in page_rows]
      body = "\n".join(body_lines)
      if body:
        body += "\n"
      abs_path.write_text(body, encoding="utf-8")
      digest = hashlib.sha256(body.encode("utf-8")).hexdigest()
      shards.append(
        {
          "shard_id": f"{bucket}/{page:05}",
          "path": rel_path,
          "bucket": bucket,
          "page": page,
          "row_count": len(page_rows),
          "content_digest": digest,
          "license_ref": "",
        }
      )

  manifest = {
    "schema": "pnixc-meta.dictionary-shard-manifest.v0",
    "manifest_id": f"shards/{source_id}",
    "source_id": source_id,
    "license_ref": license_ref,
    "max_rows_per_shard": max_rows,
    "shards": shards,
  }
  (Path(shards_dir) / "manifest.json").write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2),
    encoding="utf-8",
  )

  print(
    f"STDICT report: dictionary_dir={dictionary_dir} entries_seen={entries_seen} rows={total_rows} "
    f"shards={len(shards)} held_missing_lemma={held_missing_lemma} "
    f"held_missing_definition={held_missing_definition} max_rows_per_shard={max_rows}",
    file=sys.stderr,
  )


if __name__ == "__main__":
  dictionary_dir = sys.argv[1]
  manifest_path = sys.argv[2]
  shards_dir = sys.argv[3]
  source_id = sys.argv[4]
  domain = sys.argv[5]
  max_rows = int(sys.argv[6])
  try:
    build_shards(dictionary_dir, manifest_path, shards_dir, source_id, domain, max_rows)
  except Exception as exc:
    raise SystemExit(f"preingest-stdict-shards-build-failed: {exc}")
PY

INGEST_ARGS=(--dict-shards-ingest "$REDB_PATH" "$SHARDS_DIR")
if [[ "${PNIX_DICTIONARY_FACTS_ONLY:-0}" == "1" ]]; then
  INGEST_ARGS+=(--facts-only)
fi

echo "=== stdict full ingest to redb 시작 ==="
echo "  Dictionary dir: $DICTIONARY_DIR"
echo "  Manifest:      $MANIFEST_PATH"
echo "  Source:        $SOURCE_ID"
echo "  Domain:        ${DOMAIN:-<unset>}"
echo "  Shards:        $SHARDS_DIR"
echo "  Facts only:    ${PNIX_DICTIONARY_FACTS_ONLY:-0}"

echo "pnixc-meta ${INGEST_ARGS[*]}"
"$PNIXC_META_BIN" "${INGEST_ARGS[@]}"

if [[ "${PNIX_STDICT_ORIGINAL_LANGUAGE_FACTS:-1}" == "1" ]]; then
  ORIGIN_SCOPE="${PNIX_STDICT_ORIGINAL_LANGUAGE_SCOPE:-foreign-core}"
  ORIGIN_LIMIT="${PNIX_STDICT_ORIGINAL_LANGUAGE_LIMIT:-500}"
  echo "=== stdict original_language_info rec{} facts 생성/적재 ==="
  "$REPO_ROOT/scripts/gen-dictionary-stdict-original-language-facts.sh" \
    --src "$DICTIONARY_DIR" \
    --scope "$ORIGIN_SCOPE" \
    --limit "$ORIGIN_LIMIT" \
    --out "$REPO_ROOT/stdlib/lib/nl/dictionary-stdict-original-language-facts.generated.px"
  PNIXC_META_CORPUS_REDB_PATH="$REDB_PATH" \
    "$PNIXC_META_BIN" --morph-rules-build "$REPO_ROOT/stdlib/lib/nl/dictionary-stdict-original-language-store-plan.px"
else
  echo "=== stdict original_language_info rec{} facts 적재 생략(PNIX_STDICT_ORIGINAL_LANGUAGE_FACTS=0) ==="
fi

echo "=== 완료 ==="
