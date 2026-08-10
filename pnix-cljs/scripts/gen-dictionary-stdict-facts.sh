#!/usr/bin/env bash
# 외부 사전 덤프(표준국어대사전 stdict JSON) → facts-only(lemma/pos) .px 모듈 생성.
#
# ★라이선스 회피(clean-room): 저작권법은 *표현*(정의문/용례)만 보호하고 *사실*(lemma·품사)엔 권리 없음.
#   word_info.word + pos_info[].pos 만 뽑고 comm_pattern_info/sense_info(정의문·용례)를 전부 버린다 → 비저작권
#   사실 → CC BY-SA 의 SA(copyleft) 가 안 붙음 → 상업 사용 자유. ★생성물(*.generated.px)은 gitignore —
#   repo 는 *코드(이 스크립트)만* 배포, CC BY-SA *데이터*는 배포하지 않는다(사용자가 덤프 받아 로컬 생성).
#   ⚠ 변호사 아님: 상업제품이면 "facts-only 상업 추출" 한 줄은 사람 변호사 확인 권장.
#
# ★헌법: host(이 스크립트)=IO/전사만, 의미 0. pos→술어/개체 분류는 dictionary-facts.px(.px) 단일소스(인터프리터-우선).
#   생성 모듈 → korean-taxonomy-lift.px 의 dictFacts overlay 가 seed 에 merge(pathExists-guard, gate-safe).
#
# stdict JSON 구조: { channel: { item: [ { word_info: { word, pos_info: [ { pos, ... } ], original_language_info } } ] } }
#   lemma 정제: 동음이의 상첨자(01) 제거 · 형태소경계 '-' 제거(금지-하다→금지하다) · 띄어쓰기 '^'→공백(금지^어업→금지 어업).
#   ★original_language_info(영어/프랑스어/한자 등)는 여기서 섞지 않는다. 별도
#   gen-dictionary-stdict-original-language-facts.sh + store-plan 으로 cross-language lexical clue 만 보존한다
#   (번역 truth / ontology assertion 금지).
#
# 사용:  bash scripts/gen-dictionary-stdict-facts.sh [--limit N] [--src DIR] [--out FILE]
#   --limit N : non-predicate facts 최대치(기본 1000; 0=전체).
#   --predicate-limit N : predicate facts 최대치(기본 3000; 0=전체). codec 핵심 회귀 lemma 는 cap 밖이어도 보존.
#   --src : stdict JSON 디렉터리.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
LIMIT=1000
PREDICATE_LIMIT=3000
SRC="$ROOT/ingest/dictionary/stdict"
OUT="$ROOT/stdlib/lib/nl/dictionary-stdict-facts.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --limit) LIMIT="$2"; shift 2;;
    --predicate-limit) PREDICATE_LIMIT="$2"; shift 2;;
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$OUT" "$LIMIT" "$PREDICATE_LIMIT" <<'PY'
import json, glob, os, sys, re
from collections import Counter
src, out, limit, pred_limit = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
PRED_POS = {"동사", "형용사", "보조 동사", "보조 형용사"}
PRIORITY_PRED_LEMMAS = {
    "있다", "하다", "쓰다", "보다", "주다", "피다", "서다", "사다", "먹다", "받다", "가다", "오다",
    "알다", "만들다", "열다", "먹이다", "죽이다", "끓이다",
    "바쁘다", "나쁘다", "아프다", "고프다", "예쁘다", "기쁘다", "슬프다", "크다", "끄다", "뜨다", "트다",
    "잠그다", "담그다", "들르다", "살다",
    "노랗다", "파랗다", "빨갛다", "까맣다", "하얗다", "허옇다", "뿌옇다",
    "그렇다", "이렇다", "저렇다", "어떻다", "동그랗다", "커다랗다"
}
AEOS = ("아", "어", "와", "워", "라", "래", "해")

def decomp(ch):
    o = ord(ch)
    if o < 0xAC00 or o > 0xD7A3:
        return None
    x = o - 0xAC00
    return x // 588, (x % 588) // 28, x % 28

def regular_aeo(stem):
    if not stem:
        return ""
    if stem.endswith("하"):
        return stem[:-1] + "해"
    d = decomp(stem[-1])
    if d is None:
        return stem + "어"
    _, jung, jong = d
    if jong != 0:
        return stem + ("아" if jung in (0, 8) else "어")
    # generator/lift already handles common vowel contractions; this is only a safety
    # comparator so regular C-stems like 잡아/좋아 do not become exact-dict overrides.
    return stem + ("아" if jung in (0, 8) else "어")

def compose(cho, jung, jong):
    return chr(0xAC00 + ((cho * 21) + jung) * 28 + jong)

def h_irregular_aeo(stem):
    if not stem:
        return None
    d = decomp(stem[-1])
    if d is None or d[2] != 27:
        return None
    cho, jung, _ = d
    j = 1 if jung in (0, 4) else 3 if jung == 2 else 7 if jung == 6 else jung
    return stem[:-1] + compose(cho, j, 0)

def is_aeo_candidate(stem, c):
    if c.endswith(AEOS):
        return True
    # ㅎ불규칙 아/어형은 빨개·까매·하얘·허예·어때처럼 마지막 음절이 고정 tuple 밖에 온다.
    return c == h_irregular_aeo(stem)

def select_aeo(stem, conjs):
    reg = regular_aeo(stem)
    for c in conjs:
        c = clean(c)
        if c and is_aeo_candidate(stem, c) and c != reg:
            return c
    return None

def reverse_safe(stem):
    if stem.endswith("르"):
        return True
    d = decomp(stem[-1]) if stem else None
    return d is not None and d[2] != 0

def clean(w):
    w = re.sub(r'\d+', '', w)   # 동음이의 상첨자 제거
    w = w.replace('-', '')      # 형태소 경계 제거 (금지-하다 → 금지하다)
    w = w.replace('^', ' ')     # 띄어쓰기 마커 → 공백 (금지^어업 → 금지 어업)
    return w.strip()

seen, all_pred_rows, rest_rows = set(), [], []
for f in sorted(glob.glob(os.path.join(src, "*.json"))):
    try:
        d = json.load(open(f, encoding='utf-8'))
    except Exception:
        continue
    items = d.get("channel", {}).get("item", [])
    if not isinstance(items, list):
        continue
    for it in items:
        wi = it.get("word_info", {})
        lemma = clean(wi.get("word", ""))
        conjs = []
        for ci in wi.get("conju_info", []) or []:
            x = (ci.get("conjugation_info") or {}).get("conjugation")
            if x:
                conjs.append(x)
        for pi in wi.get("pos_info", []):
            pos = (pi.get("pos") or "").strip()
            # ★facts-only: lemma+pos+활용표면(aeo) 만. sense_info(정의문)/comm_pattern_info 무시(clean-room).
            if lemma and pos and (lemma, pos) not in seen:
                seen.add((lemma, pos))
                stem = lemma[:-1] if pos in PRED_POS and lemma.endswith("다") else lemma
                aeo = select_aeo(stem, conjs) if pos in PRED_POS else None
                aeo_rev = bool(aeo) and reverse_safe(stem)
                if pos in PRED_POS:
                    all_pred_rows.append((lemma, pos, aeo, aeo_rev))
                elif limit == 0 or len(rest_rows) < limit:
                    rest_rows.append((lemma, pos, None, False))

pred_rows = all_pred_rows if pred_limit == 0 else all_pred_rows[:pred_limit]
selected_pred = set((l, p) for l, p, _, _ in pred_rows)
for row in all_pred_rows:
    if row[0] in PRIORITY_PRED_LEMMAS and (row[0], row[1]) not in selected_pred:
        pred_rows.append(row); selected_pred.add((row[0], row[1]))
rows = pred_rows + rest_rows
def esc(s): return s.replace("\\", "\\\\").replace('"', '\\"')
def fmt(l, p, aeo, aeo_rev):
    extra = ' aeo = "%s";' % esc(aeo) if aeo else ""
    extra += " aeo_rev = true;" if aeo_rev else ""
    return '    { lemma = "%s"; pos = "%s";%s }' % (esc(l), esc(p), extra)
body = "\n".join(fmt(l, p, aeo, aeo_rev) for l, p, aeo, aeo_rev in rows)
hdr = ("# stdlib/lib/nl/dictionary-stdict-facts.generated.px — GENERATED (gitignored), do not commit.\n"
       "# 생성: bash scripts/gen-dictionary-stdict-facts.sh  (출처: 국립국어원 표준국어대사전, CC BY-SA — facts-only 추출)\n"
       "# lemma/pos/활용 aeo 만(정의문/용례 제외=clean-room, 비저작권 사실). predicate facts 는 전체, non-predicate 는 limit 적용.\n")
open(out, "w", encoding="utf-8").write(hdr + '{ schema = "dictionary.stdict-facts.v1";\n  facts = [\n%s\n  ];\n}\n' % body)
print("generated %s (%d facts, facts-only; predicates=%d/%d, non_predicates=%d). pos dist: %s" % (
    out, len(rows), len(pred_rows), len(all_pred_rows), len(rest_rows), dict(Counter(p for _, p, _, _ in rows))))
PY
