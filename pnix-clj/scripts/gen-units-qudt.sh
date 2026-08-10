#!/usr/bin/env bash
# QUDT native TTL -> pnix attrset source.
# Host responsibility only: TTL transcription/extraction. No graph/math wiring.
# Excludes prose fields and UCUM fields; preserves source predicates as field rows otherwise.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/units/qudt"
OUT="$ROOT/stdlib/lib/corpus/qudt.generated.px"
QUDT_UNIT_LIMIT="${QUDT_UNIT_LIMIT:-200}"
QUDT_QUANTITY_KIND_LIMIT="${QUDT_QUANTITY_KIND_LIMIT:-100}"
QUDT_CONSTANT_LIMIT="${QUDT_CONSTANT_LIMIT:-50}"
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --unit-limit) QUDT_UNIT_LIMIT="$2"; shift 2;;
    --quantity-kind-limit) QUDT_QUANTITY_KIND_LIMIT="$2"; shift 2;;
    --constant-limit) QUDT_CONSTANT_LIMIT="$2"; shift 2;;
    --full) QUDT_UNIT_LIMIT=0; QUDT_QUANTITY_KIND_LIMIT=0; QUDT_CONSTANT_LIMIT=0; shift;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$OUT" "$QUDT_UNIT_LIMIT" "$QUDT_QUANTITY_KIND_LIMIT" "$QUDT_CONSTANT_LIMIT" <<'PY'
import hashlib, os, re, sys
from collections import Counter
src, out = sys.argv[1], sys.argv[2]
unit_limit, qk_limit, constant_limit = map(int, sys.argv[3:6])
SOURCES = [
    ("schema", "schema/SCHEMA_QUDT.ttl", "schema_term", "qudt:"),
    ("units", "vocab/unit/VOCAB_QUDT-UNITS-ALL.ttl", "unit", "unit:"),
    ("quantity_kinds", "vocab/quantitykinds/VOCAB_QUDT-QUANTITY-KINDS-ALL.ttl", "quantity_kind", "quantitykind:"),
    ("dimension_vectors", "vocab/dimensionvectors/VOCAB_QUDT-DIMENSION-VECTORS.ttl", "dimension_vector", "qkdv:"),
    ("prefixes", "vocab/prefixes/VOCAB_QUDT-PREFIXES.ttl", "prefix", "prefix:"),
    ("constants", "vocab/constants/VOCAB_QUDT-CONSTANTS.ttl", "constant", "constant:"),
    ("systems_of_units", "vocab/systems/VOCAB_QUDT-SYSTEM-OF-UNITS-ALL.ttl", "system_of_units", "sou:"),
    ("systems_of_quantity_kinds", "vocab/systems/VOCAB_QUDT-SYSTEM-OF-QUANTITY-KINDS-ALL.ttl", "system_of_quantity_kinds", "soqk:"),
]
PROSE_PRED = {
    "dcterms:description", "dc:description", "rdfs:comment", "skos:definition",
    "qudt:plainTextDescription", "qudt:latexDefinition", "qudt:latexSymbol",
    "qudt:expression", "vaem:description", "vaem:intent", "qudt:informativeReference",
    "qudt:dbpediaMatch", "qudt:wikidataMatch"
}
DROP_VALUE_NEEDLES = ("ucum", "description", "comment", "definition", "latexsymbol", "latexdefinition")
KEEP_PRED = {
    "rdf:type", "rdfs:label", "skos:prefLabel", "skos:broader", "rdfs:subClassOf",
    "rdfs:subPropertyOf", "rdfs:domain", "rdfs:range", "rdfs:isDefinedBy",
    "qudt:abbreviation", "qudt:symbol", "qudt:conversionMultiplier", "qudt:conversionMultiplierSN",
    "qudt:conversionOffset", "qudt:conversionOffsetSN", "qudt:hasDimensionVector",
    "qudt:hasQuantityKind", "qudt:applicableUnit", "qudt:applicableSystem", "qudt:isUnitOfSystem",
    "qudt:prefix", "qudt:scalingOf", "qudt:hasUnit", "qudt:hasFactorUnit", "qudt:factorUnitScalar",
    "qudt:qkdvNumerator", "qudt:qkdvDenominator", "qudt:hasReferenceQuantityKind",
    "qudt:dimensionExponentForAmountOfSubstance", "qudt:dimensionExponentForElectricCurrent",
    "qudt:dimensionExponentForLength", "qudt:dimensionExponentForLuminousIntensity",
    "qudt:dimensionExponentForMass", "qudt:dimensionExponentForThermodynamicTemperature",
    "qudt:dimensionExponentForTime", "qudt:dimensionlessExponent",
    "qudt:prefixMultiplier", "qudt:prefixMultiplierSN", "qudt:quantityValue",
    "qudt:numericValue", "qudt:unit", "qudt:value", "qudt:standardUncertainty",
    "qudt:standardUncertaintySN", "qudt:constantValue", "qudt:systemAllowedUnit",
    "qudt:coherentUnitOfSystem", "qudt:baseUnitOfSystem", "qudt:derivedUnitOfSystem",
    "qudt:definedUnitOfSystem", "qudt:exactMatch", "qudt:siExactMatch"
}

def read(rel):
    with open(os.path.join(src, rel), encoding="utf-8") as f:
        return f.read()

def sha(rel):
    with open(os.path.join(src, rel), "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()

def esc(s):
    return str(s).replace("\\", "\\\\").replace('"', '\\"').replace("${", "\\${")

def pstr(s): return '"' + esc(s) + '"'
def plist(xs): return "[ " + " ".join(pstr(x) for x in xs if x != "") + " ]"

def pattrs(d):
    parts=[]
    for k in sorted(d):
        v=d[k]
        if v is None or v=="" or v==[]: continue
        if isinstance(v, bool): parts.append(f"{k} = {'true' if v else 'false'};")
        elif isinstance(v, list): parts.append(f"{k} = {plist(v)};")
        else: parts.append(f"{k} = {pstr(v)};")
    return "{ " + " ".join(parts) + " }"

def field_attrs(pred, values):
    return "{ pred = " + pstr(pred) + "; values = " + plist(values) + "; }"

def clean_ttl(text):
    out=[]; i=0; n=len(text); quote=0; escaped=False
    while i<n:
        if quote==0 and text.startswith('"""', i): quote=3; out.append('"""'); i+=3; continue
        if quote==3 and text.startswith('"""', i): quote=0; out.append('"""'); i+=3; continue
        ch=text[i]
        if quote==0 and ch=='"': quote=1; escaped=False; out.append(ch); i+=1; continue
        if quote==1:
            out.append(ch)
            if ch=='"' and not escaped: quote=0
            escaped=(ch=='\\' and not escaped)
            if ch!='\\': escaped=False
            i+=1; continue
        if quote==0 and ch=='#':
            while i<n and text[i]!='\n': i+=1
            continue
        out.append(ch); i+=1
    return ''.join(out)

def blocks(text):
    text=clean_ttl(text)
    acc=[]; depth=0; quote=0; escaped=False; i=0; n=len(text)
    while i<n:
        if quote==0 and text.startswith('"""', i): quote=3; acc.append('"""'); i+=3; continue
        if quote==3 and text.startswith('"""', i): quote=0; acc.append('"""'); i+=3; continue
        ch=text[i]
        if quote==0 and ch=='"': quote=1; escaped=False; acc.append(ch); i+=1; continue
        if quote==1:
            acc.append(ch)
            if ch=='"' and not escaped: quote=0
            escaped=(ch=='\\' and not escaped)
            if ch!='\\': escaped=False
            i+=1; continue
        if quote==0:
            if ch=='[': depth+=1
            elif ch==']': depth-=1
            acc.append(ch)
            nxt=text[i+1] if i+1<n else ''
            if ch=='.' and depth<=0 and (nxt=='' or nxt.isspace()):
                b=''.join(acc).strip(); acc=[]
                if b and not b.startswith('@prefix'):
                    yield b
            i+=1; continue
        acc.append(ch); i+=1

def split_top(s, sep):
    out=[]; acc=[]; depth=0; quote=0; escaped=False; i=0; n=len(s)
    while i<n:
        if quote==0 and s.startswith('"""', i): quote=3; acc.append('"""'); i+=3; continue
        if quote==3 and s.startswith('"""', i): quote=0; acc.append('"""'); i+=3; continue
        ch=s[i]
        if quote==0 and ch=='"': quote=1; escaped=False; acc.append(ch); i+=1; continue
        if quote==1:
            acc.append(ch)
            if ch=='"' and not escaped: quote=0
            escaped=(ch=='\\' and not escaped)
            if ch!='\\': escaped=False
            i+=1; continue
        if quote==0:
            if ch=='[': depth+=1
            elif ch==']': depth-=1
            if ch==sep and depth==0:
                out.append(''.join(acc).strip()); acc=[]; i+=1; continue
        acc.append(ch); i+=1
    tail=''.join(acc).strip()
    if tail: out.append(tail)
    return out

def subject_and_rest(block):
    m=re.match(r"^([A-Za-z_][\w-]*:[\w.%-]+|<[^>]+>)\s+(.*)\s*\.\s*$", block, re.S)
    if not m: return None, None
    return m.group(1), m.group(2)

def local(x):
    if not x: return ""
    if x.startswith('<'):
        return x.strip('<>').rstrip('/').split('/')[-1].split('#')[-1]
    return x.split(':',1)[1] if ':' in x else x

def values(obj):
    vals=[]
    # short string literals only; prose predicates are dropped before this function.
    for m in re.finditer(r'"""(.*?)"""(?:\^\^[A-Za-z_:][\w:.-]+|@[a-zA-Z-]+)?', obj, re.S):
        x=' '.join(m.group(1).split())
        if x and len(x)<=120: vals.append(x)
    for m in re.finditer(r'"([^"\\]*(?:\\.[^"\\]*)*)"(?:\^\^[A-Za-z_:][\w:.-]+|@[a-zA-Z-]+)?', obj):
        x=m.group(1)
        if x and len(x)<=160: vals.append(x)
    scrub=re.sub(r'""".*?"""', ' ', obj, flags=re.S)
    scrub=re.sub(r'"[^"\\]*(?:\\.[^"\\]*)*"(?:\^\^[A-Za-z_:][\w:.-]+|@[a-zA-Z-]+)?', ' ', scrub)
    for x in re.findall(r'<[^>]+>|[A-Za-z_][\w-]*:[\w.%-]+|[-+]?\d+(?:\.\d+)?(?:E[-+]?\d+)?|\btrue\b|\bfalse\b', scrub):
        vals.append(x)
    # stable de-dupe
    seen=set(); out=[]
    for v in vals:
        if v not in seen:
            seen.add(v); out.append(v)
    return out

def parse_row(block, rel, kind, prefix):
    subj, rest = subject_and_rest(block)
    if not subj or not subj.startswith(prefix) or subj == prefix:
        return None
    if any(n in subj.lower() for n in DROP_VALUE_NEEDLES):
        return None
    fields=[]; classes=[]; label=None; symbol=None
    for seg in split_top(rest, ';'):
        if not seg: continue
        if seg.startswith('a '):
            vals=values(seg[2:])
            classes=[v for v in vals if ':' in v or v.startswith('<')]
            if classes:
                fields.append(('rdf:type', classes))
            continue
        m=re.match(r'([A-Za-z_][\w-]*:[\w.%-]+)\s+(.*)$', seg, re.S)
        if not m: continue
        pred,obj=m.group(1),m.group(2).strip()
        pl=pred.lower()
        if pred not in KEEP_PRED or pred in PROSE_PRED or 'ucum' in pl:
            continue
        if kind == 'schema_term' and pred == 'rdfs:subClassOf' and '[' in obj:
            continue
        vals=[v for v in values(obj) if not any(n in v.lower() for n in DROP_VALUE_NEEDLES)]
        if not vals: continue
        if pred in ('rdfs:label','skos:prefLabel') and label is None:
            label=vals[0]
        if pred in ('qudt:symbol','qudt:prefixMultiplier') and symbol is None:
            symbol=vals[0]
        fields.append((pred, vals[:32]))
    if not fields and not classes:
        return None
    return {
        'kind': kind,
        'source_file': rel,
        'subject': subj,
        'id': local(subj),
        'classes': classes,
        'label': label,
        'symbol': symbol,
        'fields': fields,
    }

rows=[]; source_files=[]
for name, rel, kind, prefix in SOURCES:
    source_files.append({'name': name, 'path': rel, 'sha256': sha(rel)})
    for b in blocks(read(rel)):
        r=parse_row(b, rel, kind, prefix)
        if r: rows.append(r)
raw_counts=Counter(r['kind'] for r in rows)
limits={'unit': unit_limit, 'quantity_kind': qk_limit, 'constant': constant_limit}
kept=[]; seen_by_kind=Counter()
for r in rows:
    lim=limits.get(r['kind'], 0)
    if lim and seen_by_kind[r['kind']] >= lim:
        continue
    kept.append(r); seen_by_kind[r['kind']] += 1
rows=kept
counts=Counter(r['kind'] for r in rows)
version_path=os.path.join(src,'VERSION')
url_path=os.path.join(src,'SOURCE_URL')
zip_sha_path=os.path.join(src,'ZIP_SHA256')
version=open(version_path).read().strip() if os.path.exists(version_path) else 'unknown'
url=open(url_path).read().strip() if os.path.exists(url_path) else 'unknown'
zip_sha=open(zip_sha_path).read().strip() if os.path.exists(zip_sha_path) else ''
lines=[]
lines.append('{ schema = "units.qudt.v1";')
lines.append('  source = "QUDT public repository native core";')
lines.append('  license = "CC-BY-4.0";')
lines.append('  attribution = "QUDT.org, Creative Commons Attribution 4.0 International License (CC BY 4.0).";')
lines.append(f'  source_version = {pstr(version)};')
lines.append(f'  source_url = {pstr(url)};')
lines.append(f'  zip_sha256 = {pstr(zip_sha)};')
lines.append('  extraction_policy = "selected QUDT core TTL files; selected source predicates compacted into native row keys; default bounded core; prose, UCUM fields, and bulky OWL restrictions excluded; graph/math wiring not performed";')
lines.append('  counts = ' + pattrs(dict(counts)) + ';')
lines.append('  total_available_counts = ' + pattrs(dict(raw_counts)) + ';')
lines.append('  default_limits = ' + pattrs({"unit": unit_limit, "quantity_kind": qk_limit, "constant": constant_limit}) + ';')
lines.append('  source_files = [')
for sf in source_files: lines.append('    ' + pattrs(sf))
lines.append('  ];')
lines.append('  records = [')
PRED_KEY = {
    "rdfs:label": "labels",
    "skos:prefLabel": "labels",
    "skos:broader": "broader",
    "rdfs:subClassOf": "subclass_of",
    "rdfs:subPropertyOf": "subproperty_of",
    "rdfs:domain": "domain",
    "rdfs:range": "range",
    "qudt:abbreviation": "abbreviations",
    "qudt:symbol": "symbols",
    "qudt:conversionMultiplier": "conversion_multiplier",
    "qudt:conversionMultiplierSN": "conversion_multiplier_sn",
    "qudt:conversionOffset": "conversion_offset",
    "qudt:conversionOffsetSN": "conversion_offset_sn",
    "qudt:hasDimensionVector": "dimension_vectors",
    "qudt:hasQuantityKind": "quantity_kinds",
    "qudt:applicableUnit": "applicable_units",
    "qudt:applicableSystem": "applicable_systems",
    "qudt:isUnitOfSystem": "systems",
    "qudt:prefix": "prefixes",
    "qudt:scalingOf": "scaling_of",
    "qudt:hasUnit": "units",
    "qudt:hasFactorUnit": "factor_units",
    "qudt:factorUnitScalar": "factor_unit_scalars",
    "qudt:qkdvNumerator": "qkdv_numerators",
    "qudt:qkdvDenominator": "qkdv_denominators",
    "qudt:hasReferenceQuantityKind": "reference_quantity_kinds",
    "qudt:dimensionExponentForAmountOfSubstance": "exp_amount_of_substance",
    "qudt:dimensionExponentForElectricCurrent": "exp_electric_current",
    "qudt:dimensionExponentForLength": "exp_length",
    "qudt:dimensionExponentForLuminousIntensity": "exp_luminous_intensity",
    "qudt:dimensionExponentForMass": "exp_mass",
    "qudt:dimensionExponentForThermodynamicTemperature": "exp_thermodynamic_temperature",
    "qudt:dimensionExponentForTime": "exp_time",
    "qudt:dimensionlessExponent": "exp_dimensionless",
    "qudt:prefixMultiplier": "prefix_multiplier",
    "qudt:prefixMultiplierSN": "prefix_multiplier_sn",
    "qudt:quantityValue": "quantity_values",
    "qudt:numericValue": "numeric_values",
    "qudt:unit": "units",
    "qudt:value": "values",
    "qudt:standardUncertainty": "standard_uncertainty",
    "qudt:standardUncertaintySN": "standard_uncertainty_sn",
    "qudt:constantValue": "constant_values",
    "qudt:systemAllowedUnit": "system_allowed_units",
    "qudt:coherentUnitOfSystem": "coherent_units",
    "qudt:baseUnitOfSystem": "base_units",
    "qudt:derivedUnitOfSystem": "derived_units",
    "qudt:definedUnitOfSystem": "defined_units",
    "qudt:exactMatch": "exact_matches",
    "qudt:siExactMatch": "si_exact_matches",
}
for r in rows:
    outrow={k:r[k] for k in ('classes','id','kind','label','source_file','subject','symbol')}
    for pred, vals in r['fields']:
        if pred == 'rdf:type':
            continue
        key=PRED_KEY.get(pred)
        if not key:
            continue
        if key in outrow and isinstance(outrow[key], list):
            outrow[key].extend(vals[:24])
        elif key in outrow and outrow[key] not in (None, ''):
            outrow[key]=[outrow[key]] + vals[:24]
        else:
            outrow[key]=vals[:24]
    # stable de-dupe list values
    for k,v in list(outrow.items()):
        if isinstance(v, list):
            seen=set(); xs=[]
            for x in v:
                if x not in seen:
                    seen.add(x); xs.append(x)
            outrow[k]=xs
    lines.append('    ' + pattrs(outrow))
lines.append('  ];')
lines.append('}')
os.makedirs(os.path.dirname(out), exist_ok=True)
with open(out,'w',encoding='utf-8') as f: f.write('\n'.join(lines)+'\n')
print(f"generated {out}: records={len(rows)} " + ' '.join(f"{k}={v}" for k,v in sorted(counts.items())))
PY
