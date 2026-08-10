#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/registry/iana-bgp-parameters/bgp-parameters.xml"
OUT="$ROOT/stdlib/lib/corpus/iana-bgp-parameters.generated.px"
python3 - <<'PY' "$SRC" "$OUT"
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); NS='{http://www.iana.org/assignments}'
def esc(s): return json.dumps(' '.join(str(s or '').split()), ensure_ascii=False)
def attr(s): return json.dumps(str(s), ensure_ascii=False)
def txt(e): return '' if e is None else ''.join(e.itertext())
def xref(e):
    parts=[]
    for k in ('type','data','section','registry','id'):
        if k in e.attrib: parts.append(f'{k}={e.attrib[k]}')
    t=' '.join(txt(e).split())
    if t: parts.append(t)
    return ' '.join(parts)
root=ET.fromstring(src.read_bytes())
title=' '.join((root.findtext(NS+'title') or '').split())
registries=[]; total=0
for reg in root.findall(NS+'registry'):
    rid=reg.attrib.get('id',''); rtitle=' '.join((reg.findtext(NS+'title') or '').split()); records=[]
    for rec in reg.findall(NS+'record'):
        fields=[]; xrefs=[]
        for ch in list(rec):
            tag=ch.tag.split('}',1)[-1]
            if tag=='xref':
                v=xref(ch)
                if v: xrefs.append(v)
            else:
                v=' '.join(txt(ch).split())
                if v: fields.append((tag,v))
                for xr in ch.findall(NS+'xref'):
                    xv=xref(xr)
                    if xv: xrefs.append(f'{tag}: {xv}')
        records.append((fields,xrefs)); total+=1
    registries.append((rid,rtitle,records))
lines=['{']
lines.append('  schema = "registry.iana.bgp_parameters.v1";')
lines.append('  source = {')
lines.append('    project = "IANA Border Gateway Protocol (BGP) Parameters";')
lines.append('    license = "IANA any-purpose registry terms";')
lines.append('    retrieved_from = "https://www.iana.org/assignments/bgp-parameters/bgp-parameters.xml";')
lines.append('  };')
lines.append(f'  title = {esc(title)};')
lines.append(f'  registry_count = {len(registries)};')
lines.append(f'  record_count = {total};')
lines.append('  registries = [')
for rid,rtitle,records in registries:
    lines.append('    {')
    lines.append(f'      id = {attr(rid)};')
    lines.append(f'      title = {esc(rtitle)};')
    lines.append(f'      record_count = {len(records)};')
    lines.append('      records = [')
    for fields,xrefs in records:
        lines.append('        {')
        lines.append('          fields = [')
        for k,v in fields:
            lines.append(f'            {{ name = {attr(k)}; value = {esc(v)}; }}')
        lines.append('          ];')
        lines.append('          xrefs = [ ' + ' '.join(esc(x) for x in xrefs) + ' ];')
        lines.append('        }')
    lines.append('      ];')
    lines.append('    }')
lines.append('  ];')
lines.append('}')
out.write_text('\n'.join(lines)+'\n', encoding='utf-8')
print(f'generated {out}: registries={len(registries)} records={total}')
PY
