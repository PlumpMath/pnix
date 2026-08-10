#!/usr/bin/env bash
# MathML/OpenMath source files -> pnix attrset source.
# Extracts schema/symbol metadata only. Drops OpenMath Description/CMP/FMP/Example bodies.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/math/mathml-openmath"
OUT="$ROOT/stdlib/lib/corpus/mathml-openmath.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$OUT" <<'PY'
import hashlib, os, re, sys, xml.etree.ElementTree as ET
from pathlib import Path
src, out = sys.argv[1], sys.argv[2]
NS={'cd':'http://www.openmath.org/OpenMathCD'}
def esc(s): return str(s).replace('\\','\\\\').replace('"','\\"').replace('${','\\${')
def pstr(s): return '"'+esc(s)+'"'
def plist(xs): return '[ ' + ' '.join(pstr(x) for x in xs if x not in (None,'')) + ' ]'
def pattrs(d):
    parts=[]
    for k in sorted(d):
        v=d[k]
        if v is None or v=='' or v==[]: continue
        if isinstance(v,bool): parts.append(f"{k} = {'true' if v else 'false'};")
        elif isinstance(v,list): parts.append(f"{k} = {plist(v)};")
        else: parts.append(f"{k} = {pstr(v)};")
    return '{ ' + ' '.join(parts) + ' }'
def sha(path):
    with open(os.path.join(src,path),'rb') as f: return hashlib.sha256(f.read()).hexdigest()
def read(path): return Path(src,path).read_text(errors='ignore')
def text(el, tag):
    x=el.find('cd:'+tag, NS)
    return ''.join(x.itertext()).strip() if x is not None else ''
records=[]; source_files=[]
# MathML RNC: lightweight element/attribute/pattern references.
for p in sorted(Path(src,'mathml/rnc').glob('*.rnc')):
    rel=str(p.relative_to(src)); data=p.read_text(errors='ignore')
    source_files.append({'path':rel,'sha256':sha(rel),'source':'w3c/mathml-schema'})
    records.append({'kind':'mathml_schema_file','id':p.name,'file':rel,'line_count':str(data.count('\n')+1)})
    elems=sorted(set(re.findall(r'element\s+(?:m:)?([A-Za-z][A-Za-z0-9_.-]*)', data)))
    attrs=sorted(set(re.findall(r'attribute\s+([A-Za-z_:][A-Za-z0-9_:.-]*)', data)))
    defs=sorted(set(re.findall(r'^([A-Za-z][A-Za-z0-9_.-]*)\s*=', data, re.M)))
    for e in elems[:300]: records.append({'kind':'mathml_element_ref','id':e,'file':p.name})
    for a in attrs[:300]: records.append({'kind':'mathml_attribute_ref','id':a,'file':p.name})
    for d in defs[:300]: records.append({'kind':'mathml_pattern_ref','id':d,'file':p.name})
# OpenMath official CDs.
for p in sorted(Path(src,'openmath/cd/Official').glob('*.ocd')):
    rel=str(p.relative_to(src)); source_files.append({'path':rel,'sha256':sha(rel),'source':'OpenMath/CDs official'})
    root=ET.parse(p).getroot()
    cdname=text(root,'CDName') or p.stem
    meta={'kind':'openmath_cd','id':cdname,'file':rel,'base':text(root,'CDBase'),'url':text(root,'CDURL'),'status':text(root,'CDStatus'),'date':text(root,'CDDate'),'review_date':text(root,'CDReviewDate'),'version':text(root,'CDVersion'),'revision':text(root,'CDRevision')}
    records.append(meta)
    for d in root.findall('cd:CDDefinition', NS):
        name=text(d,'Name'); role=text(d,'Role')
        if name:
            records.append({'kind':'openmath_symbol','id':cdname+'.'+name,'cd':cdname,'name':name,'role':role})
mathml_commit=Path(src,'mathml/COMMIT').read_text().strip() if Path(src,'mathml/COMMIT').exists() else ''
openmath_commit=Path(src,'openmath/COMMIT').read_text().strip() if Path(src,'openmath/COMMIT').exists() else ''
retrieved=Path(src,'RETRIEVED_AT').read_text().strip() if Path(src,'RETRIEVED_AT').exists() else ''
lines=[]
lines.append('{ schema = "math.formula_metadata.v1";')
lines.append('  source = "W3C MathML schema + OpenMath official content dictionaries";')
lines.append('  license = "W3C Software Notice and License for MathML schema; OpenMath license for official CDs";')
lines.append('  attribution = "W3C MathML schema; OpenMath Society official Content Dictionaries.";')
lines.append('  extraction_policy = "schema/symbol metadata only; OpenMath Description/CMP/FMP/Example bodies and spec prose excluded; pnix math graph wiring not performed";')
lines.append(f'  mathml_commit = {pstr(mathml_commit)};')
lines.append(f'  openmath_commit = {pstr(openmath_commit)};')
lines.append(f'  retrieved_at = {pstr(retrieved)};')
lines.append('  source_files = [')
for sf in source_files: lines.append('    '+pattrs(sf))
lines.append('  ];')
lines.append('  records = [')
for r in records: lines.append('    '+pattrs(r))
lines.append('  ];')
lines.append('}')
os.makedirs(os.path.dirname(out), exist_ok=True)
Path(out).write_text('\n'.join(lines)+'\n')
print(f"generated {out}: records={len(records)} source_files={len(source_files)}")
PY
