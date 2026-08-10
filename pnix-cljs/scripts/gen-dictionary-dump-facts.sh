#!/usr/bin/env bash
# 외부 사전 덤프(우리말샘 LMF XML) → facts-only(lemma/pos) .px 모듈 생성.
#
# ★라이선스 회피(clean-room): 저작권법은 *표현*(정의문/용례)만 보호하고 *사실*(lemma·품사)엔 권리 없음.
#   이 스크립트는 lemma+pos 만 뽑고 definition/example/DB배열을 전부 버린다 → 비저작권 사실 → CC BY-SA 의
#   SA(copyleft) 가 안 붙음 → 상업 사용 자유. ★생성물(*.generated.px)은 gitignore — repo 는 *코드(이 스크립트)만*
#   배포하고, CC BY-SA *데이터*는 배포하지 않는다(사용자가 덤프 받아 로컬 생성). raw 덤프도 git 에 올리지 말 것.
#   ⚠ 변호사 아님: 상업제품이면 "facts-only 상업 추출" 한 줄은 사람 변호사 확인 권장.
#
# ★헌법: host(이 스크립트)=IO/전사만, 의미 0. pos→술어/개체 분류는 dictionary-facts.px(.px) 단일소스(인터프리터-우선).
#   생성 모듈 → dictionary-dump-facts-store-plan.px 가 rec{}(pnix attrset, JSON 아님)로 redb 적재.
#
# 사용:  bash scripts/gen-dictionary-dump-facts.sh [--limit N] [--src DIR]
#   --limit N : non-predicate facts 최대치(기본 1000; 0=전체).
#   --predicate-limit N : predicate facts 최대치(기본 3000; 0=전체). codec 핵심 회귀 lemma 는 cap 밖이어도 보존.
#   --src : 우리말샘 XML 디렉터리.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
LIMIT=1000
PREDICATE_LIMIT=3000
SRC="$ROOT/ingest/dictionary/woorimalsaem"
OUT="$ROOT/stdlib/lib/nl/dictionary-dump-facts.generated.px"
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
import xml.etree.ElementTree as ET
import glob, os, sys
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
        c = c.strip()
        if c and is_aeo_candidate(stem, c) and c != reg:
            return c
    return None

def reverse_safe(stem):
    if stem.endswith("르"):
        return True
    d = decomp(stem[-1]) if stem else None
    return d is not None and d[2] != 0

seen, all_pred_rows, rest_rows = set(), [], []
for f in sorted(glob.glob(os.path.join(src, "*.xml"))):
    try:
        for ev, el in ET.iterparse(f, events=("end",)):
            if el.tag == "LexicalEntry":
                pos = None; lemma = None
                for feat in el.findall("feat"):
                    if feat.get("att") == "partOfSpeech": pos = feat.get("val")
                lem = el.find("Lemma")
                if lem is not None:
                    for feat in lem.findall("feat"):
                        if feat.get("att") == "writtenForm": lemma = feat.get("val")
                conjs = []
                for wf in el.findall("WordForm"):
                    fs = {ft.get("att"): ft.get("val") for ft in wf.findall("feat")}
                    if fs.get("type") == "활용" and fs.get("writtenForm"):
                        conjs.append(fs["writtenForm"])
                # ★facts-only: lemma+pos+활용표면(aeo) 만. definition/SenseExample/pronunciation 등은 무시(clean-room).
                if lemma and pos and pos != "" and (lemma, pos) not in seen:
                    seen.add((lemma, pos))
                    stem = lemma[:-1] if pos in PRED_POS and lemma.endswith("다") else lemma
                    aeo = select_aeo(stem, conjs) if pos in PRED_POS else None
                    aeo_rev = bool(aeo) and reverse_safe(stem)
                    if pos in PRED_POS:
                        all_pred_rows.append((lemma, pos, aeo, aeo_rev))
                    elif limit == 0 or len(rest_rows) < limit:
                        rest_rows.append((lemma, pos, None, False))
                el.clear()
    except ET.ParseError as e:
        print("skip malformed XML %s: %s" % (f, e), file=sys.stderr)
        continue
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
hdr = ("# stdlib/lib/nl/dictionary-dump-facts.generated.px — GENERATED (gitignored), do not commit.\n"
       "# 생성: bash scripts/gen-dictionary-dump-facts.sh  (출처: 국립국어원 우리말샘, CC BY-SA 2.0 KR — facts-only 추출)\n"
       "# lemma/pos/활용 aeo 만(정의문/용례 제외=clean-room, 비저작권 사실). predicate facts 는 전체, non-predicate 는 limit 적용.\n")
open(out, "w", encoding="utf-8").write(hdr + '{ schema = "dictionary.woorimalsaem-facts.v1";\n  facts = [\n%s\n  ];\n}\n' % body)
print("generated %s (%d facts, facts-only; predicates=%d/%d, non_predicates=%d). pos dist: %s" % (
    out, len(rows), len(pred_rows), len(all_pred_rows), len(rest_rows), dict(Counter(p for _, p, _, _ in rows))))
PY
