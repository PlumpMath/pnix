#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${PNIX_WORKSPACE_ROOT:-$(git -C "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/.." rev-parse --show-toplevel 2>/dev/null || pwd)}"
source "${REPO_ROOT}/scripts/require-legal-provenance-gate.sh"

export PNIX_DICTIONARY_FACTS_ONLY_SOURCE="woorimalsaem"
export PNIX_DICTIONARY_ALLOW_SHARE_ALIKE=1

pnix_require_legal_provenance_gate "dictionary-woorimalsaem-ingest" "${PNIX_LEGAL_PROVENANCE_RECEIPT:-}"

PNIXC_META_BIN="${PNIXC_META_BIN:-pnixc-meta}"
DUMP_PATH="${DICTIONARY_WOORIMALSAEM_DUMP_PATH:-${REPO_ROOT}/ingest/dictionary/woorimalsaem}"
MANIFEST_PATH="${DICTIONARY_WOORIMALSAEM_LICENSE_MANIFEST:-${REPO_ROOT}/corpus/dictionary/LICENSES/woorimalsaem.license.json}"
REDB_PATH="${PNIXC_META_REDB_PATH:-/tmp/pnix-nl-corpus-demo.redb}"
SHARDS_DIR="${DICTIONARY_WOORIMALSAEM_SHARDS_DIR:-/tmp/pnix-dictionary-woorimalsaem-shards}"
SOURCE_ID="${DICTIONARY_WOORIMALSAEM_SOURCE_ID:-woorimalsaem}"
MAX_ROWS_PER_SHARD="${PNIX_DICTIONARY_MAX_ROWS_PER_SHARD:-${DICTIONARY_MAX_ROWS_PER_SHARD:-}}"
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

if [[ ! -f "$MANIFEST_PATH" ]]; then
  echo "preingest-legal-provenance-reject-license-manifest-missing: $MANIFEST_PATH" >&2
  exit 2
fi

if [[ ! -f "$DUMP_PATH" && ! -d "$DUMP_PATH" ]]; then
  echo "preingest-legal-provenance-reject-dictionary-dump-missing: $DUMP_PATH" >&2
  exit 2
fi

if [[ ! -x "$(command -v "$PNIXC_META_BIN")" ]]; then
  echo "error: pnixc-meta not found: $PNIXC_META_BIN" >&2
  exit 2
fi

mkdir -p "$SHARDS_DIR"
mkdir -p "$(dirname "$REDB_PATH")"

DICT_TO_SHARDS_ARGS=(--dict-lmf-to-shards "$DUMP_PATH" "$MANIFEST_PATH" "$SHARDS_DIR" "$SOURCE_ID")
if [[ "$FACTS_ONLY" == "1" ]]; then
  DICT_TO_SHARDS_ARGS+=(--facts-only)
fi
if [[ -n "$MAX_ROWS_PER_SHARD" ]]; then
  DICT_TO_SHARDS_ARGS+=(--max-rows-per-shard "$MAX_ROWS_PER_SHARD")
fi
INGEST_ARGS=(--dict-shards-ingest "$REDB_PATH" "$SHARDS_DIR")
if [[ "$FACTS_ONLY" == "1" ]]; then
  INGEST_ARGS+=(--facts-only)
fi

echo "=== 우리말샘 full ingest 시작 ==="
echo "  Dump: $DUMP_PATH"
echo "  Manifest: $MANIFEST_PATH"
echo "  Source: $SOURCE_ID"
echo "  Shards: $SHARDS_DIR"
echo "  Facts-only: $FACTS_ONLY"

echo "pnixc-meta ${DICT_TO_SHARDS_ARGS[*]}"
"$PNIXC_META_BIN" "${DICT_TO_SHARDS_ARGS[@]}"

echo "=== 우리말샘 full redb ingest ==="
"$PNIXC_META_BIN" "${INGEST_ARGS[@]}"

echo "=== 완료 ==="
