#!/usr/bin/env bash
# 외부 사전(Open English WordNet 2025 JSON) → facts+structure .px 모듈 생성 (영영 사전).
#
# ★라이선스: OEWN = CC BY 4.0 (share-alike/copyleft 아님 → 상업·비공개 임베드 OK, 출처표시만).
#   그래도 ★헌법(anti-transcript): 외부 *prose* 저장 금지 → gloss(정의문)/용례(example)는 추출 안 함.
#   추출 대상 = *구조*: lemma·품사(n/v/a/r/s)·synset id·synset 구성원(동의어집합)·의미관계(hypernym 등).
#   생성물(*.generated.px)·원본 덤프는 gitignore(미배포). repo 는 *코드(이 스크립트)*만.
#
# ★헌법: host(이 스크립트)=IO/전사만, 의미 0. pos→kind 분류 등 의미는 .px 소유(여기서 안 함).
#   ★redb 적재까지가 끝 — 거울/그래프 연결(wiring)은 하지 않는다(데이터로 저장만).
#
# OEWN JSON 구조:
#   entries-*.json: { lemma: { "<pos>": { sense:[{id,synset}], ... }, ... } }  (pos = n/v/a/r/s, 동형이의 n-1 접미)
#   noun.*/verb.*/adj.*/adv.*.json: { "<synset-id>": { partOfSpeech, members:[...], hypernym:[...], definition:[...], ... } }
#
# 사용:  bash scripts/gen-dictionary-oewn-facts.sh [--limit N] [--predicate-limit N] [--synset-limit N] [--full] [--src DIR] [--out FILE]
#   --limit N           : 비-동사(noun/adj/adv/...) facts 최대치 (기본 20000; 0=전체)
#   --predicate-limit N : 동사(v) facts 최대치 (기본 0=전체)
#   --synset-limit N    : synset 최대치 (기본 20000; 0=전체)
#   --full              : 위 3개 cap 전부 해제(전체 추출)
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
LIMIT=20000
PREDICATE_LIMIT=0
SYNSET_LIMIT=20000
SRC="$ROOT/ingest/dictionary/oewn"
OUT="$ROOT/stdlib/lib/nl/dictionary-oewn-facts.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --limit) LIMIT="$2"; shift 2;;
    --predicate-limit) PREDICATE_LIMIT="$2"; shift 2;;
    --synset-limit) SYNSET_LIMIT="$2"; shift 2;;
    --full) LIMIT=0; PREDICATE_LIMIT=0; SYNSET_LIMIT=0; shift;;
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$OUT" "$LIMIT" "$PREDICATE_LIMIT" "$SYNSET_LIMIT" <<'PY'
import json, glob, os, sys, re
from collections import Counter
src, out = sys.argv[1], sys.argv[2]
limit, pred_limit, syn_limit = int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])

# prose/비구조 필드 = 추출 제외(anti-transcript). 나머지 list 키 = 의미관계(synset-id 목록)로 통과.
SYNSET_DROP = {"definition", "example", "ili", "syntacticBehaviour", "members", "partOfSpeech"}

def esc(s):
    return s.replace("\\", "\\\\").replace('"', '\\"')

def base_pos(k):
    return k.split("-", 1)[0]   # n-1 → n

# ── facts: lemma + pos (동사=predicate 우선, 한글 overlay 와 동형) ──
seen, verb_rows, rest_rows = set(), [], []
for f in sorted(glob.glob(os.path.join(src, "entries-*.json"))):
    try:
        d = json.load(open(f, encoding="utf-8"))
    except Exception:
        continue
    if not isinstance(d, dict):
        continue
    for lemma, rec in d.items():
        lemma = (lemma or "").strip()
        if not lemma or not isinstance(rec, dict):
            continue
        for pos_key in rec.keys():
            pos = base_pos(pos_key)
            if pos not in ("n", "v", "a", "r", "s"):
                continue
            if (lemma, pos) in seen:
                continue
            seen.add((lemma, pos))
            if pos == "v":
                verb_rows.append((lemma, pos))
            elif limit == 0 or len(rest_rows) < limit:
                rest_rows.append((lemma, pos))
verb_rows = verb_rows if pred_limit == 0 else verb_rows[:pred_limit]
fact_rows = verb_rows + rest_rows

# ── synsets: id + pos + members + 의미관계 (gloss/example 제외) ──
syn_rows = []
syn_pos_files = (glob.glob(os.path.join(src, "noun.*.json")) +
                 glob.glob(os.path.join(src, "verb.*.json")) +
                 glob.glob(os.path.join(src, "adj.*.json")) +
                 glob.glob(os.path.join(src, "adv.*.json")))
for f in sorted(syn_pos_files):
    if syn_limit and len(syn_rows) >= syn_limit:
        break
    try:
        d = json.load(open(f, encoding="utf-8"))
    except Exception:
        continue
    if not isinstance(d, dict):
        continue
    for sid, rec in d.items():
        if syn_limit and len(syn_rows) >= syn_limit:
            break
        if not isinstance(rec, dict):
            continue
        pos = (rec.get("partOfSpeech") or "").strip()
        members = [m for m in rec.get("members", []) if isinstance(m, str)]
        rels = {}
        for k, v in rec.items():
            if k in SYNSET_DROP:
                continue
            if isinstance(v, list) and v and all(isinstance(x, str) for x in v):
                rels[k] = v
        syn_rows.append((sid, pos, members, rels))

# ── pnix attrset 소스 직렬화 (NOT JSON) ──
def emit_facts(rows):
    return "\n".join('    { lemma = "%s"; pos = "%s"; }' % (esc(l), esc(p)) for l, p in rows)

def emit_synsets(rows):
    out_lines = []
    for sid, pos, members, rels in rows:
        mem = " ".join('"%s"' % esc(m) for m in members)
        parts = ['id = "%s";' % esc(sid), 'pos = "%s";' % esc(pos),
                 "members = [ %s ];" % mem]
        for k in sorted(rels):
            ids = " ".join('"%s"' % esc(x) for x in rels[k])
            parts.append("%s = [ %s ];" % (k, ids))
        out_lines.append("    { %s }" % " ".join(parts))
    return "\n".join(out_lines)

hdr = ("# stdlib/lib/nl/dictionary-oewn-facts.generated.px — GENERATED (gitignored), do not commit.\n"
       "# 생성: bash scripts/gen-dictionary-oewn-facts.sh\n"
       "# 출처: Open English WordNet 2025, CC BY 4.0 (https://en-word.net/), Princeton WordNet 파생. 출처표시 필수.\n"
       "# 구조만(lemma/pos/synset/members/관계). gloss(정의문)/example=제외(anti-transcript, 비저작권 사실만).\n")
body = ('{ schema = "dictionary.oewn-facts.v1";\n'
        '  attribution = "Open English WordNet 2025 (CC BY 4.0, en-word.net); derived from Princeton WordNet 3.0.";\n'
        '  facts = [\n%s\n  ];\n'
        '  synsets = [\n%s\n  ];\n}\n' % (emit_facts(fact_rows), emit_synsets(syn_rows)))
open(out, "w", encoding="utf-8").write(hdr + body)
print("generated %s: facts=%d (verbs=%d, others=%d), synsets=%d. pos dist: %s" % (
    out, len(fact_rows), len(verb_rows), len(rest_rows), len(syn_rows),
    dict(Counter(p for _, p in fact_rows))))
PY
