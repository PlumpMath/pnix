#!/usr/bin/env bash
# 표준국어대사전 stdict JSON `original_language_info` -> pnix rec{} facts.
#
# This is not a translation table. It preserves origin-language lexical clues:
#   Korean lemma -> original surface + language_type
# They are proposer evidence only. They must not be promoted to ontology truth
# without a downstream gate/receipt.
set -euo pipefail

ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/dictionary/stdict"
OUT="$ROOT/stdlib/lib/nl/dictionary-stdict-original-language-facts.generated.px"
LIMIT="${PNIX_STDICT_ORIGINAL_LANGUAGE_LIMIT:-500}"
SHARD_SIZE=10000
SINGLE=0
SCOPE="${PNIX_STDICT_ORIGINAL_LANGUAGE_SCOPE:-foreign-core}"

while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --limit) LIMIT="$2"; shift 2;;
    --shard-size) SHARD_SIZE="$2"; shift 2;;
    --scope) SCOPE="$2"; shift 2;;
    --single) SINGLE=1; shift;;
    --help|-h)
      echo "usage: $(basename "$0") [--src DIR] [--out FILE] [--limit N] [--shard-size N] [--scope foreign-core|all] [--single]" >&2
      echo "  default: --scope ${SCOPE} --limit ${LIMIT}. Use --limit 0 for all records; generic pnix store is slow for large sets." >&2
      exit 0
      ;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done

python3 - "$SRC" "$OUT" "$LIMIT" "$SHARD_SIZE" "$SINGLE" "$SCOPE" <<'PY'
import glob
import json
import os
import re
import shutil
import sys
from collections import Counter

src, out, limit, shard_size, single, scope = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]), sys.argv[6]
if scope not in ("foreign-core", "all"):
    raise SystemExit(f"unknown scope: {scope}")

def clean_word(w):
    w = re.sub(r"\d+", "", str(w or ""))
    w = w.replace("-", "")
    w = w.replace("^", " ")
    return w.strip()

def clean(v):
    return str(v or "").strip()

def esc(s):
    return s.replace("\\", "\\\\").replace('"', '\\"')

records = []
seen = set()
lang_counts = Counter()
word_type_counts = Counter()
entries_seen = 0
held_missing_origin = 0
held_out_of_scope = 0
foreign_core_exclude = {"한자", "고유어", "안 밝힘", "/(병기)"}

for f in sorted(glob.glob(os.path.join(src, "*.json"))):
    try:
        raw = json.load(open(f, encoding="utf-8"))
    except Exception:
        continue
    channel = raw.get("channel") if isinstance(raw, dict) else None
    items = channel.get("item", []) if isinstance(channel, dict) else []
    if not isinstance(items, list):
        continue
    for item in items:
        entries_seen += 1
        wi = item.get("word_info") if isinstance(item, dict) else None
        if not isinstance(wi, dict):
            continue
        lemma = clean_word(wi.get("word"))
        if not lemma:
            continue
        word_type = clean(wi.get("word_type"))
        target_code = item.get("target_code")
        origins = wi.get("original_language_info", [])
        if isinstance(origins, dict):
            origins = [origins]
        if not isinstance(origins, list) or not origins:
            held_missing_origin += 1
            continue
        for idx, origin in enumerate(origins):
            if not isinstance(origin, dict):
                continue
            surface = clean(origin.get("original_language"))
            lang = clean(origin.get("language_type"))
            if not surface or not lang:
                continue
            if scope == "foreign-core" and lang in foreign_core_exclude:
                held_out_of_scope += 1
                continue
            key = (lemma, surface, lang, word_type, str(target_code), idx)
            if key in seen:
                continue
            seen.add(key)
            records.append({
                "lemma": lemma,
                "target_code": target_code,
                "word_type": word_type,
                "origin_surface": surface,
                "origin_language_type": lang,
                "origin_index": idx,
                "evidence_kind": "stdict.original_language_info",
                "proposer_only": True,
                "durable_truth": False,
                "ontology_assert_allowed": False,
                "note": "origin-language lexical clue, not translation truth",
            })
            lang_counts[lang] += 1
            if word_type:
                word_type_counts[word_type] += 1
            if limit > 0 and len(records) >= limit:
                break
        if limit > 0 and len(records) >= limit:
            break
    if limit > 0 and len(records) >= limit:
        break

def fmt_value(v):
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    return '"' + esc(v) + '"'

def fmt_record(r):
    fields = [
        "lemma", "target_code", "word_type", "origin_surface", "origin_language_type",
        "origin_index", "evidence_kind", "proposer_only", "durable_truth",
        "ontology_assert_allowed", "note",
    ]
    body = " ".join(f"{k} = {fmt_value(r[k])};" for k in fields)
    return "    { " + body + " }"

def lang_rows(counter, indent="    "):
    return "\n".join(
        f'{indent}{{ language_type = "{esc(lang)}"; count = {count}; }}'
        for lang, count in sorted(counter.items(), key=lambda x: (-x[1], x[0]))
    )

def word_type_rows(counter, indent="    "):
    return "\n".join(
        f'{indent}{{ word_type = "{esc(wt)}"; count = {count}; }}'
        for wt, count in sorted(counter.items(), key=lambda x: (-x[1], x[0]))
    )

hdr = (
    "# stdlib/lib/nl/dictionary-stdict-original-language-facts.generated.px — GENERATED (gitignored), do not commit.\n"
    "# 생성: bash scripts/gen-dictionary-stdict-original-language-facts.sh\n"
    "# stdict original_language_info facts only: 원어/언어태그 lexical clue, 번역 truth 아님, ontology assertion 금지.\n"
)

def write_file(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w", encoding="utf-8").write(text)

def render_single():
    body = "\n".join(fmt_record(r) for r in records)
    return (
        hdr
        + '{ schema = "dictionary.stdict-original-language-facts.v1";\n'
        + '  source = "국립국어원 표준국어대사전 original_language_info";\n'
        + f"  entries_seen = {entries_seen};\n"
        + f"  record_count = {len(records)};\n"
        + f"  held_missing_origin = {held_missing_origin};\n"
        + f"  held_out_of_scope = {held_out_of_scope};\n"
        + f'  scope = "{esc(scope)}";\n'
        + "  language_counts = [\n" + lang_rows(lang_counts) + "\n  ];\n"
        + "  word_type_counts = [\n" + word_type_rows(word_type_counts) + "\n  ];\n"
        + "  records = [\n" + body + "\n  ];\n"
        + "}\n"
    )

os.makedirs(os.path.dirname(out), exist_ok=True)
if single or shard_size <= 0:
    write_file(out, render_single())
    print(f"generated {out} ({len(records)} original-language records; languages={len(lang_counts)}; scope={scope}; single=true)")
else:
    shard_dir = out[:-3] + ".d" if out.endswith(".px") else out + ".d"
    if os.path.isdir(shard_dir):
        shutil.rmtree(shard_dir)
    os.makedirs(shard_dir, exist_ok=True)

    shards = [records[i:i + shard_size] for i in range(0, len(records), shard_size)]
    shard_imports = []
    for idx, shard in enumerate(shards):
        sc = Counter(r["origin_language_type"] for r in shard)
        wc = Counter(r["word_type"] for r in shard if r["word_type"])
        body = "\n".join(fmt_record(r) for r in shard)
        shard_name = f"shard-{idx:04d}.px"
        shard_path = os.path.join(shard_dir, shard_name)
        shard_text = (
            "# GENERATED shard: stdict original_language_info facts. Do not commit.\n"
            + '{ schema = "dictionary.stdict-original-language-facts.v1";\n'
            + '  source = "국립국어원 표준국어대사전 original_language_info";\n'
            + f"  shard_index = {idx};\n"
            + f"  shard_count = {len(shards)};\n"
            + f"  shard_size = {shard_size};\n"
            + f"  record_count = {len(shard)};\n"
            + f'  scope = "{esc(scope)}";\n'
            + "  language_counts = [\n" + lang_rows(sc) + "\n  ];\n"
            + "  word_type_counts = [\n" + word_type_rows(wc) + "\n  ];\n"
            + "  records = [\n" + body + "\n  ];\n"
            + "}\n"
        )
        write_file(shard_path, shard_text)
        shard_imports.append(f"    (import ./{os.path.basename(shard_dir)}/{shard_name})")

    manifest = (
        hdr
        + '{ schema = "dictionary.stdict-original-language-facts.v1";\n'
        + '  source = "국립국어원 표준국어대사전 original_language_info";\n'
        + "  sharded = true;\n"
        + f"  shard_size = {shard_size};\n"
        + f"  shard_count = {len(shards)};\n"
        + f"  entries_seen = {entries_seen};\n"
        + f"  record_count = {len(records)};\n"
        + f"  held_missing_origin = {held_missing_origin};\n"
        + f"  held_out_of_scope = {held_out_of_scope};\n"
        + f'  scope = "{esc(scope)}";\n'
        + "  language_counts = [\n" + lang_rows(lang_counts) + "\n  ];\n"
        + "  word_type_counts = [\n" + word_type_rows(word_type_counts) + "\n  ];\n"
        + "  shards = [\n" + "\n".join(shard_imports) + "\n  ];\n"
        + "}\n"
    )
    write_file(out, manifest)
    print(f"generated {out} ({len(records)} original-language records; languages={len(lang_counts)}; scope={scope}; shards={len(shards)}; shard_size={shard_size})")
PY
