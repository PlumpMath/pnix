#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
source "${REPO_ROOT}/scripts/require-legal-provenance-gate.sh"
pnix_require_legal_provenance_gate "nl-corpus-web-vocab-ingest" "${PNIX_LEGAL_PROVENANCE_RECEIPT:-}"
REDB_PATH="${PNIXC_META_REDB_PATH:-/tmp/pnix-nl-corpus-demo.redb}"
BASE_MANIFEST="${PNIXC_META_CORPUS_MANIFEST_PATH:-${REPO_ROOT}/corpus/ko/manifest.json}"
NL_WEB_VOCAB_TOPICS="${NL_WEB_VOCAB_TOPICS:-curiosity,understanding,learning,conversation,perspective,imagination,analysis,innovation}"
NL_WEB_VOCAB_WORDS_PER_TOPIC="${NL_WEB_VOCAB_WORDS_PER_TOPIC:-4}"
NL_WEB_VOCAB_TOTAL_WORDS="${NL_WEB_VOCAB_TOTAL_WORDS:-28}"
WARM_FTS="${NL_WEB_VOCAB_WARM_FTS:-0}"
NL_WEB_VOCAB_TIMEOUT_SECS="${NL_WEB_VOCAB_TIMEOUT_SECS:-6}"
SOURCE_ID="${NL_WEB_VOCAB_SOURCE_ID:-web-vocab-live}"
SOURCE_NAME="${NL_WEB_VOCAB_SOURCE_NAME:-Web vocabulary expansion (live search + translation)}"

TMP_DIR="$(mktemp -d -t pnixc-web-vocab.XXXXXX)"
SHARD_PATH="$TMP_DIR/web-vocab.jsonl"
MANIFEST_PATH="$TMP_DIR/manifest.web.json"

trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR"

cat > "$TMP_DIR/build_web_vocab.py" <<'PY'
import json
import urllib.parse
import pathlib
import subprocess
import sys

TOPICS = [t.strip() for t in (sys.argv[1] if len(sys.argv) > 1 else "").split(",") if t.strip()]
WORDS_PER_TOPIC = int(sys.argv[2]) if len(sys.argv) > 2 else 4
TOTAL_WORDS = int(sys.argv[3]) if len(sys.argv) > 3 else 28
TIMEOUT = float(sys.argv[4]) if len(sys.argv) > 4 else 6.0
REPO_ROOT = pathlib.Path(sys.argv[5]) if len(sys.argv) > 5 else pathlib.Path(".")
BASE_MANIFEST = pathlib.Path(sys.argv[6]) if len(sys.argv) > 6 else pathlib.Path("corpus/ko/manifest.json")
SHARD_PATH = pathlib.Path(sys.argv[7]) if len(sys.argv) > 7 else pathlib.Path("web-vocab.jsonl")
MANIFEST_PATH = pathlib.Path(sys.argv[8]) if len(sys.argv) > 8 else pathlib.Path("manifest.web.json")
SOURCE_ID = sys.argv[9] if len(sys.argv) > 9 else "web-vocab-live"
SOURCE_NAME = sys.argv[10] if len(sys.argv) > 10 else "Web vocabulary expansion (live search + translation)"

OFFLINE_FALLBACK = [
  "perspective",
  "imagination",
  "insight",
  "curiosity",
  "analysis",
  "understanding",
  "reasoning",
  "observation",
  "clarity",
  "empathy",
  "innovation",
  "cooperation",
  "dialogue",
  "conversation",
  "meaning",
  "metaphor",
]

def fetch_json(url, retries=2):
  curl_cmd = [
    "/usr/bin/curl",
    "--silent",
    "--show-error",
    "--location",
    f"--max-time",
    str(int(max(1, TIMEOUT))),
    "--header",
    "User-Agent: pnixc-meta-corpus-ingest/1.0 (+https://github.com/gp/pnix)",
    url,
  ]

  for _ in range(max(1, retries)):
    proc = subprocess.run(
      curl_cmd,
      stdout=subprocess.PIPE,
      stderr=subprocess.PIPE,
      text=True,
      timeout=max(1, TIMEOUT + 2),
    )
    if proc.returncode != 0:
      last_stderr = (proc.stderr or "").strip()
      if not last_stderr:
        last_stderr = "curl return code {} without stderr".format(proc.returncode)
      print(f"warn: fetch failed ({url}) - {last_stderr}", file=sys.stderr)
      return None

    raw = (proc.stdout or "").strip()
    if not raw:
      print(f"warn: empty HTTP body ({url})", file=sys.stderr)
      return None
    try:
      return json.loads(raw)
    except Exception as exc:
      print(f"warn: invalid JSON ({url}): {exc}", file=sys.stderr)
      return None

  return None


def datamuse_words(topic, max_words):
  url = (
      "https://api.datamuse.com/words"
      f"?ml={urllib.parse.quote(topic)}&max={max_words}"
  )
  data = fetch_json(url)
  if data is None:
    return []
  return [entry.get("word", "") for entry in data if isinstance(entry, dict) and entry.get("word")]


def translate_to_ko(word):
  url = (
      "https://api.mymemory.translated.net/get"
      f"?q={urllib.parse.quote(word)}&langpair=en|ko"
  )
  payload = fetch_json(url)
  if not payload:
    return word if word else ""
  translated = (
      payload.get("responseData", {})
      .get("translatedText")
      if isinstance(payload, dict)
      else ""
  )
  return (translated or "").strip()


def lookup_definition(word):
  url = (
      "https://api.dictionaryapi.dev/api/v2/entries/en/"
      + urllib.parse.quote(word)
  )
  payload = fetch_json(url)
  if not payload or not isinstance(payload, list):
    return []

  meanings = payload[0].get("meanings") or []
  defs = []
  for meaning in meanings:
    definitions = meaning.get("definitions") or []
    for definition in definitions[:2]:
      text = (definition.get("definition") or "").strip()
      if text:
        defs.append(text)
  return defs[:2]


def load_base_manifest():
  return json.loads(BASE_MANIFEST.read_text(encoding="utf-8"))


def build_rows(words):
  rows = []
  idx = 1
  for word in words:
    k = translate_to_ko(word)
    if not k:
      continue
    definitions = lookup_definition(word)
    definition = definitions[0] if definitions else ""
    extra = f" (meaning: {definition})" if definition else ""
    en_text = (
      f"Explain '{word}' in one natural English sentence with a richer, "
      "meaning-preserving expression."
    ) + extra
    ko_text = (
      f"'{k}'의 뜻을 살려 일상 대화에서 더 자연스럽고 풍부한 문장으로"
      f" 재구성해 주세요.{ '' if definition == '' else ' 단어의 뉘앙스를 반영해 주세요.'}"
    )
    rows.append(
      {
        "row_id": f"web-vocab-en-{idx:04d}",
        "source_id": SOURCE_ID,
        "text": en_text,
        "pattern_id": "web-vocab-seed-en",
        "tags": ["dialogue", "lang:en", "web-vocab"],
      }
    )
    rows.append(
      {
        "row_id": f"web-vocab-ko-{idx:04d}",
        "source_id": SOURCE_ID,
        "text": ko_text,
        "pattern_id": "web-vocab-seed-ko",
        "tags": ["dialogue", "lang:ko", "web-vocab"],
      }
    )
    idx += 1
  return rows


def ensure_source(manifest):
  existing = [s.get("source_id") for s in manifest.get("sources", [])]
  if SOURCE_ID in existing:
    return manifest

  manifest["sources"].append(
    {
      "source_id": SOURCE_ID,
      "display_name": SOURCE_NAME,
      "license_check_required": False,
      "license_unverified": False,
      "redistribution_allowed": True,
      "commercial_use_allowed": True,
      "derivative_allowed": True,
      "ai_training_allowed": True,
      "attribution_required": False,
      "raw_text_store_allowed": True,
      "derived_feature_store_allowed": True,
      "runtime_use": "allowed",
      "notes": "internet-sourced demo expansion for NL dialogue growth",
    }
  )
  return manifest


def append_shard(manifest, shard_path):
  seed_shards = manifest.get("seed_shards")
  if not isinstance(seed_shards, list):
    manifest["seed_shards"] = []
    seed_shards = manifest["seed_shards"]
  if str(shard_path) not in seed_shards:
    seed_shards.append(str(shard_path))


def write_rows(path, rows):
  with path.open("w", encoding="utf-8") as f:
    for row in rows:
      f.write(json.dumps(row, ensure_ascii=False))
      f.write("\n")


def main():
  words = []
  for topic in [t.strip() for t in TOPICS if t.strip()]:
    for candidate in datamuse_words(topic, WORDS_PER_TOPIC):
      if not candidate:
        continue
      candidate = candidate.strip().lower()
      if candidate not in words and len(words) < TOTAL_WORDS:
        words.append(candidate)
    if len(words) >= TOTAL_WORDS:
      break

  if not words:
    words = OFFLINE_FALLBACK[:TOTAL_WORDS]

  rows = build_rows(words)
  if not rows:
    raise SystemExit("no translation rows generated")

  SHARD_PATH.parent.mkdir(parents=True, exist_ok=True)
  write_rows(SHARD_PATH, rows)

  manifest = load_base_manifest()
  manifest = ensure_source(manifest)
  append_shard(manifest, SHARD_PATH)

  MANIFEST_PATH.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
  print(f"built {len(rows)} web-vocab rows from {len(words)} seed words")
  print(f"manifest: {MANIFEST_PATH}")
  print(f"shard: {SHARD_PATH}")

if __name__ == "__main__":
  main()
PY

python3 "$TMP_DIR/build_web_vocab.py" \
  "$NL_WEB_VOCAB_TOPICS" \
  "$NL_WEB_VOCAB_WORDS_PER_TOPIC" \
  "$NL_WEB_VOCAB_TOTAL_WORDS" \
  "$NL_WEB_VOCAB_TIMEOUT_SECS" \
  "$REPO_ROOT" \
  "$BASE_MANIFEST" \
  "$SHARD_PATH" \
  "$MANIFEST_PATH" \
  "$SOURCE_ID" \
  "$SOURCE_NAME"

cd "$REPO_ROOT"

if [[ ! -x target/debug/pnixc-meta ]]; then
  echo "building pnixc-meta..." >&2
  cargo build -p pnixc-meta
fi

echo "nl web vocab ingest:"
echo "  manifest: $BASE_MANIFEST"
echo "  topics: $NL_WEB_VOCAB_TOPICS"
echo "  words/topic: $NL_WEB_VOCAB_WORDS_PER_TOPIC"
echo "  total: $NL_WEB_VOCAB_TOTAL_WORDS"

INGEST_ARGS=(--corpus-ingest-redb "$REDB_PATH" --corpus-manifest "$MANIFEST_PATH")
if [[ "$WARM_FTS" == "1" ]]; then
  INGEST_ARGS+=(--warm-fts)
fi

PNIXC_META_CORPUS_MANIFEST_PATH="$MANIFEST_PATH" target/debug/pnixc-meta "${INGEST_ARGS[@]}"

printf '\n'
