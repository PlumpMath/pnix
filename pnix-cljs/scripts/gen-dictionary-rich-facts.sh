#!/usr/bin/env bash
# Bootstrap extractor: stdict JSON + woorimalsaem XML -> richer pnix facts modules.
#
# Goal:
# - facts-only redb is the narrow consumer layer.
# - bootstrap layer keeps richer generic records from raw dictionary sources.
# - generated outputs remain gitignored pnix modules, usable by later cache/build steps.
#
# Outputs (gitignored — regenerate locally):
# - stdlib/lib/nl/dictionary-stdict-rich-facts.generated.px  (manifest when sharded)
# - stdlib/lib/nl/dictionary-stdict-rich-facts.generated.d/  (shard modules)
# - stdlib/lib/nl/dictionary-woorimalsaem-rich-facts.generated.px (+ .d/)
# - stdlib/lib/nl/dictionary-rich-aeo.generated.px
#
# Large outputs shard under 50 MiB per file (GitHub size gate). Use --single for
# monolithic debug output only.
# Fields are intentionally broader than facts-only:
#   lemma, written_form, pos, pronunciation, definition, definition_original,
#   example, origin_language_info, sense_no, sense_code, word_type, word_unit,
#   conjugation, variant_forms, derived_forms, valency, derives_hada,
#   mood, level, honorific, aspect, modality, particles, quote_markers,
#   vocabulary_level, semantic_category, source_file
set -euo pipefail

ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
STDICT_SRC="$ROOT/ingest/dictionary/stdict"
WOORI_SRC="$ROOT/ingest/dictionary/woorimalsaem"
OUT_DIR="$ROOT/stdlib/lib/nl"
LIMIT="${PNIX_DICTIONARY_RICH_LIMIT:-0}"
SHARD_SIZE="${PNIX_DICTIONARY_RICH_SHARD_SIZE:-8000}"
SINGLE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --stdict-src) STDICT_SRC="$2"; shift 2;;
    --woori-src) WOORI_SRC="$2"; shift 2;;
    --out-dir) OUT_DIR="$2"; shift 2;;
    --limit) LIMIT="$2"; shift 2;;
    --shard-size) SHARD_SIZE="$2"; shift 2;;
    --single) SINGLE=1; shift;;
    --help|-h)
      echo "usage: $(basename "$0") [--stdict-src DIR] [--woori-src DIR] [--out-dir DIR] [--limit N] [--shard-size N] [--single]" >&2
      echo "  default shard_size=${SHARD_SIZE}. Use --single for monolithic debug output (may exceed GitHub 50MiB gate)." >&2
      exit 0
      ;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done

python3 - "$STDICT_SRC" "$WOORI_SRC" "$OUT_DIR" "$LIMIT" "$SHARD_SIZE" "$SINGLE" <<'PY'
import glob, json, os, re, shutil, sys
import xml.etree.ElementTree as ET

stdict_src, woori_src, out_dir, limit, shard_size, single = (
    sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5]), int(sys.argv[6])
)

def esc(s):
    return str(s).replace("\\", "\\\\").replace('"', '\\"')

def clean_word(w):
    w = re.sub(r"\d+", "", str(w or ""))
    w = w.replace("-", "").replace("^", " ")
    return w.strip()

def norm_list(xs):
    out = []
    seen = set()
    for x in xs or []:
        if isinstance(x, dict):
            key = tuple(sorted((str(k), repr(v)) for k, v in x.items()))
            if key in seen:
                continue
            seen.add(key)
            out.append(x)
            continue
        x = str(x or "").strip()
        if not x or x in seen:
            continue
        seen.add(x)
        out.append(x)
    return out

def norm_dict_list(xs):
    out = []
    seen = set()
    for x in xs or []:
        if not isinstance(x, dict):
            continue
        key = tuple(sorted((str(k), repr(v)) for k, v in x.items()))
        if key in seen:
            continue
        seen.add(key)
        out.append(x)
    return out

def fmt_value(v):
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        return "[ " + " ".join(fmt_value(x) for x in v) + " ]"
    if isinstance(v, dict):
        return "{ " + " ".join(f'{k} = {fmt_value(v[k])};' for k in v) + " }"
    return '"' + esc(v) + '"'

def fmt_record(r):
    fields = {
        "lemma": r.get("lemma"),
        "written_form": r.get("written_form"),
        "pos": r.get("pos"),
        "pronunciation": norm_list(r.get("pronunciation", [])),
        "origin_language_info": norm_list(r.get("origin_language_info", [])),
        "sense_no": r.get("sense_no"),
        "sense_code": r.get("sense_code"),
        "word_type": r.get("word_type"),
        "word_unit": r.get("word_unit"),
        "conjugation": norm_list(r.get("conjugation", [])),
        "variant_forms": norm_list(r.get("variant_forms", [])),
        "derived_forms": norm_list(r.get("derived_forms", [])),
        "valency": r.get("valency"),
        "derives_hada": r.get("derives_hada", False),
        "mood": r.get("mood"),
        "level": r.get("level"),
        "honorific": r.get("honorific"),
        "aspect": r.get("aspect"),
        "modality": r.get("modality"),
        "particles": norm_list(r.get("particles", [])),
        "quote_markers": norm_list(r.get("quote_markers", [])),
        "vocabulary_level": r.get("vocabulary_level"),
        "semantic_category": r.get("semantic_category"),
        "lexical_info": norm_dict_list(r.get("lexical_info", [])),
        "cat_info": norm_dict_list(r.get("cat_info", [])),
        "sense_pattern_info": norm_dict_list(r.get("sense_pattern_info", [])),
        "source_file": r.get("source_file"),
    }
    if not fields["lemma"] or not fields["pos"]:
        return None
    return "    { " + " ".join(f"{k} = {fmt_value(v)};" for k, v in fields.items()) + " }"

def write_px(path, schema, source, records, extra=None):
    extra = extra or {}
    lines = [
        "# GENERATED (gitignored), do not commit.",
        f"# source: {source}",
        f"# generate: bash scripts/gen-dictionary-rich-facts.sh",
        '{ schema = "%s";' % schema,
    ]
    for k, v in extra.items():
        lines.append(f"  {k} = {fmt_value(v)};")
    lines.append("  records = [")
    for r in records:
        line = fmt_record(r)
        if line:
            lines.append(line)
    lines.append("  ];")
    lines.append("}")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w", encoding="utf-8").write("\n".join(lines) + "\n")

def write_px_sharded(path, schema, source, records, extra=None):
    extra = dict(extra or {})
    if single or shard_size <= 0 or not records:
        write_px(path, schema, source, records, extra)
        print(f"generated {path} ({len(records)} records; single=true)")
        return

    shard_dir = path[:-3] + ".d" if path.endswith(".px") else path + ".d"
    if os.path.isdir(shard_dir):
        shutil.rmtree(shard_dir)
    os.makedirs(shard_dir, exist_ok=True)

    shards = [records[i:i + shard_size] for i in range(0, len(records), shard_size)]
    shard_dir_name = os.path.basename(shard_dir)
    shard_imports = []
    for idx, shard in enumerate(shards):
        shard_name = f"shard-{idx:04d}.px"
        shard_path = os.path.join(shard_dir, shard_name)
        shard_extra = dict(extra)
        shard_extra.update({
            "shard_index": idx,
            "shard_count": len(shards),
            "shard_size": shard_size,
            "record_count": len(shard),
        })
        write_px(shard_path, schema, source, shard, shard_extra)
        shard_imports.append(f"    (import ./{shard_dir_name}/{shard_name})")

    lines = [
        "# GENERATED (gitignored), do not commit.",
        f"# source: {source}",
        f"# generate: bash scripts/gen-dictionary-rich-facts.sh",
        '{ schema = "%s";' % schema,
    ]
    for k, v in extra.items():
        lines.append(f"  {k} = {fmt_value(v)};")
    lines.extend([
        "  sharded = true;",
        f"  shard_size = {shard_size};",
        f"  shard_count = {len(shards)};",
        f"  record_count = {len(records)};",
        "  shards = [",
    ])
    lines.extend(shard_imports)
    lines.extend(["  ];", "}"])
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print(f"generated {path} ({len(records)} records; shards={len(shards)}; shard_size={shard_size})")

stdict_records = []
for f in sorted(glob.glob(os.path.join(stdict_src, "*.json"))):
    try:
        raw = json.load(open(f, encoding="utf-8"))
    except Exception:
        continue
    channel = raw.get("channel", {})
    items = channel.get("item", []) if isinstance(channel, dict) else []
    for it in items if isinstance(items, list) else []:
        wi = it.get("word_info", {}) or {}
        lemma = clean_word(wi.get("word"))
        written_form = wi.get("word")
        pos_infos = wi.get("pos_info", []) or []
        origin_infos = wi.get("original_language_info", []) or []
        if isinstance(origin_infos, dict):
            origin_infos = [origin_infos]
        origin_info_rows = []
        for oi in origin_infos:
            if isinstance(oi, dict):
                origin_info_rows.append({
                    "surface": oi.get("original_language"),
                    "language_type": oi.get("language_type"),
                })
        pron = [p.get("pronunciation") for p in wi.get("pronunciation_info", []) or [] if isinstance(p, dict)]
        word_unit = wi.get("word_unit")
        word_type = wi.get("word_type")
        for pi in pos_infos if isinstance(pos_infos, list) else []:
            pos = str(pi.get("pos") or "").strip()
            senses = pi.get("sense_info", []) or []
            conj = []
            defs = []
            defs_orig = []
            exs = []
            lexical_infos = []
            cat_infos = []
            sense_patterns = []
            sense_codes = []
            vocab_levels = []
            sem_cats = []
            for s in senses if isinstance(senses, list) else []:
                if not isinstance(s, dict):
                    continue
                sn = s.get("sense_no")
                if s.get("definition"):
                    defs.append(s.get("definition"))
                if s.get("definition_original"):
                    defs_orig.append(s.get("definition_original"))
                if s.get("sense_code") is not None:
                    sense_codes.append(s.get("sense_code"))
                if s.get("vocabularyLevel"):
                    vocab_levels.append(s.get("vocabularyLevel"))
                if s.get("semanticCategory"):
                    sem_cats.append(s.get("semanticCategory"))
                if isinstance(s.get("lexical_info"), list):
                    lexical_infos.extend(s.get("lexical_info"))
                if isinstance(s.get("cat_info"), list):
                    cat_infos.extend(s.get("cat_info"))
                if isinstance(s.get("sense_pattern_info"), list):
                    sense_patterns.extend(s.get("sense_pattern_info"))
                for e in s.get("example_info", []) or []:
                    if isinstance(e, dict) and e.get("example"):
                        exs.append(e.get("example"))
                if sn is not None:
                    pass
            for wf in wi.get("conju_info", []) or []:
                if isinstance(wf, dict):
                    c = (wf.get("conjugation_info") or {}).get("conjugation")
                    if c:
                        conj.append(c)
            # ★license: definition/definition_original/example(프로즈)는 record 에 *담지 않는다* — write_px durable 저장 금지와
            #   동일 둘레(transient 읽기는 위 senses 루프서만, durable record 엔 미진입). conjugation/valency 등 문법 facts만.
            stdict_records.append({
                "lemma": lemma,
                "written_form": written_form,
                "pos": pos,
                "pronunciation": pron,
                "origin_language_info": origin_info_rows,
                "sense_no": None,
                "sense_code": sense_codes[0] if sense_codes else None,
                "word_type": word_type,
                "word_unit": word_unit,
                "conjugation": conj,
                "variant_forms": [wi.get("word")] if wi.get("word") else [],
                "derived_forms": [],
                "valency": None,
                "derives_hada": False,
                "mood": None,
                "level": None,
                "honorific": None,
                "aspect": None,
                "modality": None,
                "particles": [],
                "quote_markers": [],
                "vocabulary_level": vocab_levels[0] if vocab_levels else None,
                "semantic_category": sem_cats[0] if sem_cats else None,
                "lexical_info": lexical_infos,
                "cat_info": cat_infos,
                "sense_pattern_info": sense_patterns,
                "source_file": os.path.basename(f),
            })

woori_records = []
for f in sorted(glob.glob(os.path.join(woori_src, "*.xml"))):
    try:
        for ev, el in ET.iterparse(f, events=("end",)):
            if el.tag != "LexicalEntry":
                continue
            lemma = ""
            written_form = ""
            pos = ""
            pron = []
            defs = []
            defs_orig = []
            exs = []
            variants = []
            origin_info_rows = []
            lexical_infos = []
            cat_infos = []
            sense_patterns = []
            sense_no = None
            sense_code = None
            vocabulary_level = None
            semantic_category = None
            lexical_unit = None
            for feat in el.findall("feat"):
                att = feat.get("att")
                val = feat.get("val")
                if att == "partOfSpeech":
                    pos = val or ""
                elif att in ("homonym_number", "homonymNumber"):
                    origin_info_rows.append({"kind": "homonym_number", "value": val})
                elif att == "lexicalUnit":
                    lexical_unit = val
                elif att == "id":
                    sense_no = val
            lem = el.find("Lemma")
            if lem is not None:
                for feat in lem.findall("feat"):
                    att = feat.get("att")
                    val = feat.get("val")
                    if att == "writtenForm":
                        written_form = val or ""
                        lemma = clean_word(val)
                    elif att == "origin":
                        origin_info_rows.append({"kind": "origin", "value": val})
            for wf in el.findall(".//WordForm"):
                fs = {ft.get("att"): ft.get("val") for ft in wf.findall("feat")}
                if fs.get("writtenForm"):
                    variants.append(fs["writtenForm"])
                if fs.get("pronunciation"):
                    pron.append(fs["pronunciation"])
                if fs.get("vocabularyLevel"):
                    vocabulary_level = fs["vocabularyLevel"]
                if fs.get("semanticCategory"):
                    semantic_category = fs["semanticCategory"]
                if fs.get("type") == "활용" and fs.get("writtenForm"):
                    variants.append(fs["writtenForm"])
            for sense in el.findall(".//Sense"):
                sense_defs = []
                for feat in sense.findall("feat"):
                    att = feat.get("att")
                    val = feat.get("val")
                    if att == "definition" and val:
                        sense_defs.append(val)
                    if att == "definition_original" and val:
                        defs_orig.append(val)
                    if att == "example" and val:
                        exs.append(val)
                    if att == "pronunciation" and val:
                        pron.append(val)
                    if att == "id" and val:
                        sense_code = val
                    if att == "lexical_info" and val:
                        lexical_infos.append({"value": val})
                    if att == "cat_info" and val:
                        cat_infos.append({"value": val})
                    if att == "sense_pattern_info" and val:
                        sense_patterns.append({"value": val})
                defs.extend(sense_defs)
            for feat in el.findall(".//feat"):
                if feat.get("att") in ("origin_language", "original_language", "original_language_info"):
                    val = feat.get("val")
                    if val:
                        origin_info_rows.append({"value": val})
            if lemma and pos:
                woori_records.append({
                    # ★license: definition/definition_original/example(프로즈) record 미진입(durable 금지 둘레).
                    "lemma": lemma,
                    "written_form": written_form or lemma,
                    "pos": pos,
                    "pronunciation": norm_list(pron),
                    "origin_language_info": norm_list(origin_info_rows),
                    "sense_no": sense_no,
                    "sense_code": sense_code,
                    "word_type": lexical_unit,
                    "word_unit": lexical_unit,
                    "conjugation": [],
                    "variant_forms": norm_list(variants),
                    "derived_forms": [],
                    "valency": None,
                    "derives_hada": False,
                    "mood": None,
                    "level": None,
                    "honorific": None,
                    "aspect": None,
                    "modality": None,
                    "particles": [],
                    "quote_markers": [],
                    "vocabulary_level": vocabulary_level,
                    "semantic_category": semantic_category,
                    "lexical_info": lexical_infos,
                    "cat_info": cat_infos,
                    "sense_pattern_info": sense_patterns,
                    "source_file": os.path.basename(f),
                })
            el.clear()
    except ET.ParseError:
        continue

# ── thin aeo overlay (consumer 경로, additive) ──────────────────────────────
#   rich conjugation → {stem, aeo} 도출 = dictIrregularClassOf(불규칙클래스) 커버리지 확장.
#   ★아/어형 선택은 *위치추정 아님*: 음절 중성이 아/어-class 인 conjugation 형태(Hangul 검증).
#   ★homonym 충돌(굽→구워[ㅂ불규칙]/굽어[규칙]) = 같은 stem 다른 aeo → DROP(held, fail-closed; 임의선택 금지).
#   ★license: conjugation 활용형은 문법 fact(프로즈 아님) — durable 안전.
def is_aeo_form(s):
    if not s:
        return False
    code = ord(s[-1]) - 0xAC00
    if code < 0 or code >= 11172:        # 완성형 음절 아님
        return False
    jung = (code // 28) % 21             # 중성 인덱스
    return jung in (0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 14, 15)   # 아/어-class(ㅏㅐㅑㅒㅓㅔㅕㅖㅘㅙㅝㅞ)

def aeo_stem_of(lemma, pos):
    if pos in ("동사", "형용사") and lemma.endswith("다"):
        return lemma[:-1]
    return lemma

aeo_map = {}        # stem -> { aeo: pos }  (stdict conju_info; woori conjugation 비어있음)
for r in stdict_records:
    pos = r.get("pos")
    if pos not in ("동사", "형용사"):
        continue
    aeo = None
    for c in r.get("conjugation") or []:
        if isinstance(c, str) and is_aeo_form(c):
            aeo = c
            break
    if not aeo:
        continue
    stem = aeo_stem_of(r.get("lemma", ""), pos)
    if not stem:
        continue
    aeo_map.setdefault(stem, {})[aeo] = pos   # ★ALL 수집(필터 전) — 충돌 검출이 전체 aeo 집합을 봐야 함

aeo_facts = []
aeo_conflicts = 0
for stem, m in aeo_map.items():
    if len(m) > 1:                       # ★충돌(굽→구워[ㅂ불규칙]/굽어[규칙] 등) → drop(held, fail-closed)
        aeo_conflicts += 1
        continue
    aeo, pos = next(iter(m.items()))
    # ★thin 필터(무손실, 단일 aeo 에만): aeo 가 stem 으로 시작 = 규칙 자음어간(먹→먹어·잡→잡아) = 코덱 default 처리 →
    #   emit 불요. 안 시작 = stem 변형(불규칙 돕→도와·짓→지어, 축약 쓰→써, 르 흐르→흘러) = dictIrregularClassOf 필요.
    if aeo.startswith(stem):
        continue
    aeo_facts.append({"lemma": stem + "다", "pos": pos, "aeo": aeo})

def write_aeo_px(path, facts, conflicts):
    lines = [
        "# GENERATED (gitignored), do not commit.",
        "# source: ingest/dictionary/stdict/*.json conju_info (아/어형 → aeo).",
        "# thin consumer overlay: dictIrregularClassOf 커버리지. homonym 충돌 stem 은 DROP(held).",
        '{ schema = "dictionary.rich-aeo.v1";',
        "  rich_aeo = true;",
        f"  conflict_count = {conflicts};",
        "  facts = [",
    ]
    for fct in facts:
        lines.append(
            '    { lemma = "%s"; pos = "%s"; aeo = "%s"; }'
            % (esc(fct["lemma"]), esc(fct["pos"]), esc(fct["aeo"]))
        )
    lines.append("  ];")
    lines.append("}")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w", encoding="utf-8").write("\n".join(lines) + "\n")

write_aeo_px(os.path.join(out_dir, "dictionary-rich-aeo.generated.px"), aeo_facts, aeo_conflicts)
print(f"thin aeo overlay: facts={len(aeo_facts)} conflicts(held)={aeo_conflicts}")

if limit > 0:
    stdict_records = stdict_records[:limit]
    woori_records = woori_records[:limit]

write_px_sharded(
    os.path.join(out_dir, "dictionary-stdict-rich-facts.generated.px"),
    "dictionary.stdict-rich-facts.v1",
    "ingest/dictionary/stdict/*.json",
    stdict_records,
    {"rich": True, "bootstrap": True},
)
write_px_sharded(
    os.path.join(out_dir, "dictionary-woorimalsaem-rich-facts.generated.px"),
    "dictionary.woorimalsaem-rich-facts.v1",
    "ingest/dictionary/woorimalsaem/*.xml",
    woori_records,
    {"rich": True, "bootstrap": True},
)

print(f"generated rich facts: stdict={len(stdict_records)} woorimalsaem={len(woori_records)}")
PY
