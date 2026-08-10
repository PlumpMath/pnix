#!/usr/bin/env bash
# BIPM SI Digital Framework TTL -> pnix attrset source.
# Host responsibility only: TTL transcription/extraction. No pnix math/graph wiring.
# Extracted data = structural unit/prefix/constant/quantity/ontology rows.
# Excluded data = comments, definition prose, examples, branding assets.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/units/bipm-si"
OUT="$ROOT/stdlib/lib/corpus/bipm-si.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$OUT" <<'PY'
import hashlib, os, re, sys
src, out = sys.argv[1], sys.argv[2]
files = {
    "units": "knowledge_graphs/SI_Reference_Point/units.ttl",
    "prefixes": "knowledge_graphs/SI_Reference_Point/prefixes.ttl",
    "constants": "knowledge_graphs/SI_Reference_Point/constants.ttl",
    "si": "knowledge_graphs/SI_Reference_Point/si.ttl",
    "quantities": "knowledge_graphs/quantities/quantities.ttl",
}

def read(rel):
    with open(os.path.join(src, rel), encoding="utf-8") as f:
        return f.read()

def sha(rel):
    with open(os.path.join(src, rel), "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()

def esc(s):
    if s is None:
        return ""
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("${", "\\${")

def pstr(s):
    return '"' + esc(str(s)) + '"'

def plist(xs):
    return "[ " + " ".join(pstr(x) for x in xs if x is not None and str(x) != "") + " ]"

def pattrs(d):
    parts = []
    for k in sorted(d):
        v = d[k]
        if v is None or v == "" or v == []:
            continue
        if isinstance(v, bool):
            parts.append(f"{k} = {'true' if v else 'false'};")
        elif isinstance(v, list):
            parts.append(f"{k} = {plist(v)};")
        else:
            parts.append(f"{k} = {pstr(v)};")
    return "{ " + " ".join(parts) + " }"

def clean_ttl(text):
    out = []
    i, n = 0, len(text)
    quote = 0
    escaped = False
    while i < n:
        if quote == 0 and text.startswith('"""', i):
            quote = 3
            out.append('"""')
            i += 3
            continue
        if quote == 3 and text.startswith('"""', i):
            quote = 0
            out.append('"""')
            i += 3
            continue
        ch = text[i]
        if quote == 0 and ch == '"':
            quote = 1
            escaped = False
            out.append(ch)
            i += 1
            continue
        if quote == 1:
            out.append(ch)
            if ch == '"' and not escaped:
                quote = 0
            escaped = (ch == "\\" and not escaped)
            if ch != "\\":
                escaped = False
            i += 1
            continue
        if quote == 0 and ch == '#':
            while i < n and text[i] != '\n':
                i += 1
            continue
        out.append(ch)
        i += 1
    return ''.join(out)

def blocks(text):
    text = clean_ttl(text)
    acc = []
    depth = 0
    quote = 0
    escaped = False
    i, n = 0, len(text)
    while i < n:
        if quote == 0 and text.startswith('"""', i):
            quote = 3
            acc.append('"""')
            i += 3
            continue
        if quote == 3 and text.startswith('"""', i):
            quote = 0
            acc.append('"""')
            i += 3
            continue
        ch = text[i]
        if quote == 0 and ch == '"':
            quote = 1
            escaped = False
            acc.append(ch)
            i += 1
            continue
        if quote == 1:
            acc.append(ch)
            if ch == '"' and not escaped:
                quote = 0
            escaped = (ch == "\\" and not escaped)
            if ch != "\\":
                escaped = False
            i += 1
            continue
        if quote == 0:
            if ch == '[':
                depth += 1
            elif ch == ']':
                depth -= 1
            acc.append(ch)
            nxt = text[i + 1] if i + 1 < n else ''
            if ch == '.' and depth <= 0 and (nxt == '' or nxt.isspace()):
                b = ''.join(acc).strip()
                acc = []
                if b and not b.startswith('@prefix'):
                    yield b
            i += 1
            continue
        acc.append(ch)
        i += 1

def subject(block):
    m = re.match(r"^([A-Za-z_][\w-]*:[\w.%-]+|<[^>]+>)\s+", block)
    return m.group(1) if m else None

def local(curie):
    if not curie:
        return ""
    if curie.startswith("<"):
        return curie.strip("<>").rstrip("/").split("/")[-1].split("#")[-1]
    return curie.split(":", 1)[1]

def classes(block):
    # Turtle shorthand `a`: normally appears right after the subject (`units:metre a si:SIBaseUnit ;`)
    # and may also appear inside blank nodes. For row classification we need the subject-level first `a`.
    m = re.match(r"^[A-Za-z_][\w-]*:[\w.%-]+\s+a\s+([^;\n]+)", block)
    if not m:
        m = re.match(r"^<[^>]+>\s+a\s+([^;\n]+)", block)
    if not m:
        m = re.search(r"(?:^|[;\n]\s*)a\s+([^;\n]+)", block)
    if not m:
        return []
    return re.findall(r"[A-Za-z_][\w-]*:[\w.%-]+", m.group(1))

def lits(block, pred, lang=None):
    if lang:
        return re.findall(re.escape(pred) + r'\s+"([^"\\]*(?:\\.[^"\\]*)*)"@' + re.escape(lang), block)
    return re.findall(re.escape(pred) + r'\s+"([^"\\]*(?:\\.[^"\\]*)*)"(?:\^\^[A-Za-z_:][\w:.-]+|@[a-zA-Z-]+)?', block)

def first_lit(block, pred, lang=None):
    xs = lits(block, pred, lang)
    return xs[0] if xs else None

def refs(block, pred, prefix=None):
    out = []
    rx = re.escape(pred) + r"\s+([^;\.\n]+(?:,\s*[^;\.\n]+)*)"
    for chunk in re.findall(rx, block):
        for x in re.findall(r"[A-Za-z_][\w-]*:[\w.%-]+|<[^>]+>", chunk):
            if prefix is None or x.startswith(prefix + ":"):
                out.append(x)
    return sorted(set(out))

def bool_lit(block, pred):
    m = re.search(re.escape(pred) + r"\s+(true|false)\b", block)
    return None if not m else (m.group(1) == "true")

def row_common(kind, subj, block):
    return {
        "kind": kind,
        "id": local(subj),
        "subject": subj,
        "classes": classes(block),
        "label_en": first_lit(block, "skos:prefLabel", "en"),
        "label_fr": first_lit(block, "skos:prefLabel", "fr"),
        "symbol": first_lit(block, "si:hasSymbol"),
    }

unit_rows = []
for b in blocks(read(files["units"])):
    s = subject(b)
    if not s or not s.startswith("units:") or s == "units:":
        continue
    cs = classes(b)
    if not any(c.startswith("si:") and ("Unit" in c or c.endswith("Unit")) for c in cs):
        continue
    r = row_common("unit", s, b)
    r.update({
        "quantity_kinds": [local(x) for x in refs(b, "si:isUnitOfQtyKind", "quantities")],
        "prefix_restriction": bool_lit(b, "si:prefixRestriction"),
        "base_units": [local(x) for x in refs(b, "si:hasBaseSIUnit", "units")],
        "product_units": [local(x) for x in refs(b, "si:hasProductUnit", "units")],
        "non_si_unit": local(refs(b, "si:hasNonSIUnit", "units")[0]) if refs(b, "si:hasNonSIUnit", "units") else None,
        "factor": first_lit(b, "si:hasFactor"),
        "exponent": first_lit(b, "si:hasExponent"),
    })
    unit_rows.append(r)

prefix_rows = []
for b in blocks(read(files["prefixes"])):
    s = subject(b)
    if not s or not s.startswith("prefixes:") or s == "prefixes:":
        continue
    if "si:SIPrefix" not in classes(b):
        continue
    r = row_common("prefix", s, b)
    r.update({
        "exponent": first_lit(b, "si:hasExponent"),
        "scaling_factor": first_lit(b, "si:hasScalingFactor"),
        "datatype": local(refs(b, "si:hasDatatype")[0]) if refs(b, "si:hasDatatype") else None,
    })
    prefix_rows.append(r)

constant_rows = []
for b in blocks(read(files["constants"])):
    s = subject(b)
    if not s or not s.startswith("constants:") or s == "constants:":
        continue
    if "si:Constant" not in classes(b):
        continue
    r = row_common("constant", s, b)
    r.update({
        "value": first_lit(b, "si:hasValue"),
        "value_as_string": first_lit(b, "si:hasValueAsString"),
        "unit": local(refs(b, "si:hasUnit", "units")[0]) if refs(b, "si:hasUnit", "units") else None,
        "updated_date": first_lit(b, "si:hasUpdatedDate"),
    })
    constant_rows.append(r)

quantity_rows = []
for b in blocks(read(files["quantities"])):
    s = subject(b)
    if not s or not s.startswith("quantities:") or s == "quantities:":
        continue
    cs = classes(b)
    if not any(c.endswith("QuantityKind") or c.endswith("Quantity") for c in cs):
        continue
    r = row_common("quantity", s, b)
    alt_en = lits(b, "skos:altLabel", "en")[:16]
    units = [local(x) for x in refs(b, "si:hasUnit", "units")]
    r.update({
        "alt_labels_en": alt_en,
        "units": units,
        "has_compound_unit": "si:hasUnit [" in b,
    })
    quantity_rows.append(r)

ontology_rows = []
for b in blocks(read(files["si"])):
    s = subject(b)
    if not s or not s.startswith("si:") or s == "si:":
        continue
    cs = classes(b)
    if not any(c in ("owl:Class", "owl:ObjectProperty", "owl:DatatypeProperty", "rdf:Property") for c in cs):
        continue
    kind = "ontology_term"
    if "owl:Class" in cs:
        kind = "ontology_class"
    elif "owl:ObjectProperty" in cs:
        kind = "ontology_object_property"
    elif "owl:DatatypeProperty" in cs:
        kind = "ontology_datatype_property"
    r = row_common(kind, s, b)
    r.update({
        "domain": local(refs(b, "rdfs:domain")[0]) if refs(b, "rdfs:domain") else None,
        "range": local(refs(b, "rdfs:range")[0]) if refs(b, "rdfs:range") else None,
        "subclass_of": [local(x) for x in refs(b, "rdfs:subClassOf")],
        "subproperty_of": [local(x) for x in refs(b, "rdfs:subPropertyOf")],
    })
    ontology_rows.append(r)

commit_path = os.path.join(src, "COMMIT")
commit = open(commit_path, encoding="utf-8").read().strip() if os.path.exists(commit_path) else "unknown"
source_files = []
for name, rel in files.items():
    source_files.append({"name": name, "path": rel, "sha256": sha(rel)})
counts = {
    "units": len(unit_rows),
    "prefixes": len(prefix_rows),
    "constants": len(constant_rows),
    "quantities": len(quantity_rows),
    "ontology_terms": len(ontology_rows),
}
rows = []
rows.append('{ schema = "units.bipm.si.v1";')
rows.append('  source = "BIPM SI Digital Framework / SI Reference Point";')
rows.append('  license = "CC-BY-3.0-IGO";')
rows.append('  attribution = "BIPM SI Digital Framework, CC BY 3.0 IGO, https://si-digital-framework.org/";')
rows.append(f'  source_commit = {pstr(commit)};')
rows.append('  extraction_policy = "structural TTL fields only; comments, definitions, examples, and branding assets excluded; graph/math wiring not performed";')
rows.append('  counts = ' + pattrs(counts) + ';')
rows.append('  source_files = [')
for sf in source_files:
    rows.append('    ' + pattrs(sf))
rows.append('  ];')
for key, vals in [
    ("units", unit_rows),
    ("prefixes", prefix_rows),
    ("constants", constant_rows),
    ("quantities", quantity_rows),
    ("ontology_terms", ontology_rows),
]:
    rows.append(f'  {key} = [')
    for r in vals:
        rows.append('    ' + pattrs(r))
    rows.append('  ];')
rows.append('}')
os.makedirs(os.path.dirname(out), exist_ok=True)
with open(out, "w", encoding="utf-8") as f:
    f.write("\n".join(rows) + "\n")
print(f"generated {out}: " + ", ".join(f"{k}={v}" for k, v in counts.items()))
PY
