#!/usr/bin/env bash
# NIST SP 330/SP 811 HTML/PDF metadata -> pnix attrset source.
# Host responsibility only: extract bounded structural refs. No prose body, no PDF text extraction.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/units/nist-si"
OUT="$ROOT/stdlib/lib/corpus/nist-si.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$OUT" <<'PY'
import hashlib, html, os, re, sys
from pathlib import Path
src, out = sys.argv[1], sys.argv[2]

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
def sha(rel):
    with open(os.path.join(src, rel), 'rb') as f: return hashlib.sha256(f.read()).hexdigest()
def read(rel): return Path(src, rel).read_text(errors='ignore')
def clean(s): return re.sub(r'\s+', ' ', html.unescape(re.sub('<.*?>',' ',s))).strip()
def html_text(rel): return read(rel)
def url_for(rel):
    p=Path(src, rel+'.url')
    return p.read_text().strip() if p.exists() else ''
def unique(xs):
    out=[]; seen=set()
    for x in xs:
        if x and x not in seen:
            seen.add(x); out.append(x)
    return out
records=[]
# publication metadata rows: PDFs are hashed only; PDF prose is not ingested.
records.extend([
  {'kind':'publication', 'id':'NIST.SP.330-2019', 'publication':'SP 330', 'edition':'2019', 'title':'The International System of Units (SI)', 'source_url':url_for('NIST.SP.330-2019.pdf'), 'sha256':sha('NIST.SP.330-2019.pdf'), 'status':'current_us_si_interpretation'},
  {'kind':'publication', 'id':'NIST.SP.811-2008', 'publication':'SP 811', 'edition':'2008', 'title':'Guide for the Use of the International System of Units (SI)', 'source_url':url_for('nistspecialpublication811e2008.pdf'), 'sha256':sha('nistspecialpublication811e2008.pdf'), 'status':'pre_2019_si_revision_guidance'},
])
# SP330 table refs from official page.
sp330=html_text('sp330.html')
for m in re.finditer(r'Table\s+(\d+)\.\s*([^<\n]{1,180})', sp330, re.I):
    no, title = m.group(1), clean(m.group(2))
    if title:
        records.append({'kind':'table_ref','publication':'SP 330','edition':'2019','id':'SP330.table.'+no,'number':no,'title':title})
# Appendix refs from SP330/SP811 pages.
for pub, edition, rel in [('SP 330','2019','sp330.html'), ('SP 811','2008','sp811.html')]:
    text=html_text(rel)
    for m in re.finditer(r'Appendix\s+([A-Z0-9](?:\.\d+)?)\s*[-.:]?\s*([^<\n]{1,160})', text, re.I):
        no,title=m.group(1), clean(m.group(2))
        if title and not title.lower().startswith('and') and not title.strip().startswith('&'):
            records.append({'kind':'appendix_ref','publication':pub,'edition':edition,'id':pub.replace(' ','')+'.appendix.'+no.replace('.','_'),'number':no,'title':title})
# Version refs from version history pages.
for pub, rel in [('SP 330','sp330-version-history.html'), ('SP 811','sp811-version-history.html')]:
    text=html_text(rel)
    hits=[]
    for m in re.finditer(r'(Published\s+\d{4}\s+Version|Web\s+Version\s+\d\.\d|Version\s+\d\.\d)', text, re.I):
        hits.append(clean(m.group(1)))
    for i,h in enumerate(unique(hits),1):
        records.append({'kind':'version_ref','publication':pub,'id':pub.replace(' ','')+'.version.'+str(i),'label':h})
# Style/schema anchors. These are short references, not rule prose.
for no,title in [
    ('5.1','use of unit symbols and names'), ('5.2','unit symbols'), ('5.3','unit names'),
    ('5.4','expressing values of quantities'), ('5.4.4','formatting numbers and decimal marker'),
    ('5.4.5','measurement uncertainty'), ('5.4.6','multiplication and division formatting'),
    ('SP811.B8','conversion factors listed alphabetically'), ('SP811.B9','conversion factors by quantity or field')
]:
    records.append({'kind':'style_ref','id':'nist.si.style.'+no.replace('.','_'),'section':no,'title':title,'source':'SP 330/SP 811'})
# de-dupe by kind/id/title
seen=set(); out_records=[]
for r in records:
    key=(r.get('kind'), r.get('id'), r.get('label',''))
    # For duplicate appendix ids from page nav/title variants, keep the first structural row.
    if key not in seen:
        seen.add(key); out_records.append(r)
records=out_records
retrieved=Path(src,'RETRIEVED_AT').read_text().strip() if Path(src,'RETRIEVED_AT').exists() else ''
source_files=[]
for rel in ['sp330.html','sp330-version-history.html','sp811.html','sp811-version-history.html','NIST.SP.330-2019.pdf','nistspecialpublication811e2008.pdf']:
    source_files.append({'path':rel, 'url':url_for(rel), 'sha256':sha(rel)})
lines=[]
lines.append('{ schema = "units.nist.si_style.v1";')
lines.append('  source = "NIST SP 330/SP 811 SI publications";')
lines.append('  license = "US-PD/NIST public information; BIPM-derived SP330 prose not ingested";')
lines.append('  attribution = "National Institute of Standards and Technology (NIST), SP 330 and SP 811 SI publications.";')
lines.append('  extraction_policy = "publication/version/table/appendix/style-reference metadata only; PDF body prose and BIPM SI Brochure text not ingested; graph/math wiring not performed";')
lines.append(f'  retrieved_at = {pstr(retrieved)};')
lines.append('  source_files = [')
for sf in source_files: lines.append('    ' + pattrs(sf))
lines.append('  ];')
lines.append('  records = [')
for r in records: lines.append('    '+pattrs(r))
lines.append('  ];')
lines.append('}')
os.makedirs(os.path.dirname(out), exist_ok=True)
Path(out).write_text('\n'.join(lines)+'\n')
print(f"generated {out}: records={len(records)}")
PY
