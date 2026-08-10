#!/usr/bin/env bash
set -euo pipefail
# ChEBI core OBO -> pnix attrset source for redb append.
# Host code is IO/transcription only. No math graph wiring, no normalization to pnix math.
# Excludes prose-heavy fields (`def`, `comment`, `synonym`, `xref`) and very large structure strings
# (`inchi_string`, `smiles_string`). Keeps OBO-like structure:
# id/name/namespace/subset/is_a/relationship/property_value(formula/charge/mass/inchi_key)/obsolete metadata.
# Compact projection note: OBO comments after `!` are labels only, so they are omitted to keep one redb row small.
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
SRC="${CHEBI_SOURCE:-$ROOT/ingest/science/chebi/chebi_core.obo}"
OUT="${CHEBI_OUT:-$ROOT/stdlib/lib/corpus/chebi.generated.px}"
LIMIT="${CHEBI_RECORD_LIMIT:-1500}"
if [ ! -f "$SRC" ]; then
  echo "missing ChEBI source: $SRC" >&2
  echo "run: scripts/update-science-chebi.sh" >&2
  exit 1
fi
mkdir -p "$(dirname "$OUT")"
python3 - "$SRC" "$OUT" "$LIMIT" <<'PY'
import hashlib, re, sys
from pathlib import Path
src = Path(sys.argv[1])
out = Path(sys.argv[2])
limit = int(sys.argv[3])
text = src.read_text(encoding='utf-8', errors='replace')
sha = hashlib.sha256(src.read_bytes()).hexdigest()

def q(s):
    if s is None:
        return 'null'
    s = str(s)
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n').replace('\r', '\\r').replace('\t', '\\t') + '"'

def lit(v, indent=0):
    sp = ' ' * indent
    if isinstance(v, bool):
        return 'true' if v else 'false'
    if isinstance(v, int):
        return str(v)
    if v is None:
        return 'null'
    if isinstance(v, str):
        return q(v)
    if isinstance(v, list):
        if not v:
            return '[ ]'
        return '[\n' + ''.join((' ' * (indent + 2)) + lit(x, indent + 2) + '\n' for x in v) + sp + ']'
    if isinstance(v, dict):
        if not v:
            return '{ }'
        parts = ['{']
        for k in v:
            parts.append((' ' * (indent + 2)) + k + ' = ' + lit(v[k], indent + 2) + ';')
        parts.append(sp + '}')
        return '\n'.join(parts)
    raise TypeError(type(v))

def split_comment(rest):
    if ' ! ' in rest:
        a, b = rest.split(' ! ', 1)
        return a.strip(), b.strip()
    return rest.strip(), None

def parse_property_value(rest):
    m = re.match(r'^(\S+)\s+"((?:[^"\\]|\\.)*)"(?:\s+(\S+))?', rest)
    if m:
        raw = m.group(2)
        try:
            raw = bytes(raw, 'utf-8').decode('unicode_escape')
        except Exception:
            pass
        return {"property": m.group(1), "value": raw, "datatype": m.group(3) or ""}
    bits = rest.split()
    if not bits:
        return None
    return {"property": bits[0], "value": bits[1] if len(bits) > 1 else "", "datatype": bits[2] if len(bits) > 2 else ""}

ALLOWED_PROPERTY_VALUES = {
    'chemrof:charge',
    'chemrof:generalized_empirical_formula',
    'chemrof:mass',
    'chemrof:monoisotopic_mass',
    'chemrof:inchi_key_string',
}
PROPERTY_KEY = {
    'chemrof:charge': 'charge',
    'chemrof:generalized_empirical_formula': 'generalized_empirical_formula',
    'chemrof:mass': 'mass',
    'chemrof:monoisotopic_mass': 'monoisotopic_mass',
    'chemrof:inchi_key_string': 'inchi_key_string',
}

def finish(block):
    rec = {"id":"", "name":"", "namespace":"", "subset":[], "alt_id":[], "is_a":[], "relationship":[], "property_value":{}, "is_obsolete": False, "replaced_by":[], "consider":[]}
    for line in block:
        if not line or line.startswith('!'):
            continue
        if line.startswith('id: '): rec["id"] = line[4:].strip()
        elif line.startswith('name: '): rec["name"] = line[6:].strip()
        elif line.startswith('namespace: '): rec["namespace"] = line[11:].strip()
        elif line.startswith('subset: '): rec["subset"].append(line[8:].strip())
        elif line.startswith('alt_id: '): rec["alt_id"].append(line[8:].strip())
        elif line.startswith('is_obsolete: '): rec["is_obsolete"] = line[13:].strip() == 'true'
        elif line.startswith('replaced_by: '): rec["replaced_by"].append(line[13:].strip())
        elif line.startswith('consider: '): rec["consider"].append(line[10:].strip())
        elif line.startswith('is_a: '):
            target, label = split_comment(line[6:])
            rec["is_a"].append(target)
        elif line.startswith('relationship: '):
            rest = line[14:].strip()
            main, label = split_comment(rest)
            bits = main.split()
            if len(bits) >= 2:
                rec["relationship"].append(bits[0] + " " + bits[1])
        elif line.startswith('property_value: '):
            pv = parse_property_value(line[16:].strip())
            if pv is not None and pv.get("property") in ALLOWED_PROPERTY_VALUES:
                rec["property_value"][PROPERTY_KEY[pv["property"]]] = pv["value"]
        # Intentionally ignore: def/comment/synonym/xref/created_by/creation_date.
    # Drop empty optional fields to keep redb row compact while preserving OBO field names used.
    compact = {"id": rec["id"], "name": rec["name"]}
    for k in ["namespace", "subset", "alt_id", "is_a", "relationship", "property_value", "is_obsolete", "replaced_by", "consider"]:
        v = rec[k]
        if v not in ("", [], {}, False):
            compact[k] = v
    return compact if compact["id"] else None

header = {}
terms = []
in_term = False
block = []
for line in text.splitlines():
    if line == '[Term]':
        if in_term:
            r = finish(block)
            if r: terms.append(r)
        in_term = True
        block = []
        continue
    if line.startswith('[') and line.endswith(']'):
        if in_term:
            r = finish(block)
            if r: terms.append(r)
        in_term = False
        block = []
        continue
    if in_term:
        block.append(line)
    else:
        if line.startswith('data-version: '): header['data_version'] = line.split(': ',1)[1]
        elif line.startswith('date: '): header['source_date'] = line.split(': ',1)[1]
        elif line.startswith('ontology: '): header['ontology'] = line.split(': ',1)[1]
if in_term:
    r = finish(block)
    if r: terms.append(r)
selected = terms[:limit]
value = {
    "schema": "chem.chebi.v1",
    "source": "ChEBI core OBO",
    "license": "CC-BY-4.0",
    "source_url": "https://purl.obolibrary.org/obo/chebi/chebi_core.obo",
    "source_sha256": sha,
    "data_version": header.get('data_version', ''),
    "source_date": header.get('source_date', ''),
    "record_limit": limit,
    "total_terms_seen": len(terms),
    "records": selected,
}
out.write_text('# GENERATED by scripts/gen-science-chebi.sh; do not commit.\n' + lit(value) + '\n', encoding='utf-8')
print(f"generated {out}: records={len(selected)} total_terms={len(terms)} data_version={header.get('data_version','')} sha256={sha}")
PY
