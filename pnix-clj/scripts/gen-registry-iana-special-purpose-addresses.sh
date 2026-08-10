#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/registry/iana-special-purpose-addresses"
OUT="$ROOT/stdlib/lib/corpus/iana-special-purpose-addresses.generated.px"
python3 - <<'PY' "$SRC" "$OUT"
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
NS='{http://www.iana.org/assignments}'

def esc(s):
    if s is None: s=''
    return json.dumps(' '.join(str(s).split()), ensure_ascii=False)

def attr_escape(s):
    return json.dumps(str(s), ensure_ascii=False)

def text(e):
    if e is None: return ''
    return ''.join(e.itertext())

def xref_value(e):
    parts=[]
    for k in ('type','data','section','registry','id'):
        if k in e.attrib: parts.append(f'{k}={e.attrib[k]}')
    t=' '.join(text(e).split())
    if t: parts.append(t)
    return ' '.join(parts)

def parse_doc(path, doc_id):
    root=ET.fromstring(path.read_bytes())
    title=' '.join((root.findtext(NS+'title') or '').split())
    registries=[]; rec_total=0
    for reg in root.findall(NS+'registry'):
        rid=reg.attrib.get('id','')
        rtitle=' '.join((reg.findtext(NS+'title') or '').split())
        records=[]
        for rec in reg.findall(NS+'record'):
            fields=[]; xrefs=[]
            for ch in list(rec):
                tag=ch.tag.split('}',1)[-1]
                if tag=='xref':
                    xv=xref_value(ch)
                    if xv: xrefs.append(xv)
                else:
                    val=' '.join(text(ch).split())
                    if val: fields.append((tag,val))
                    for xr in ch.findall(NS+'xref'):
                        xv=xref_value(xr)
                        if xv: xrefs.append(f'{tag}: {xv}')
            records.append((fields,xrefs)); rec_total+=1
        registries.append((rid,rtitle,records))
    return {'id':doc_id,'path':path.name,'title':title,'registries':registries,'record_count':rec_total}

docs=[parse_doc(src/'iana-ipv4-special-registry.xml','iana-ipv4-special-registry'), parse_doc(src/'iana-ipv6-special-registry.xml','iana-ipv6-special-registry')]
registry_count=sum(len(d['registries']) for d in docs); record_count=sum(d['record_count'] for d in docs)
lines=[]
lines.append('{')
lines.append('  schema = "registry.iana.special_purpose_addresses.v1";')
lines.append('  source = {')
lines.append('    project = "IANA Special-Purpose Address Registries";')
lines.append('    license = "IANA any-purpose registry terms";')
lines.append('    retrieved_from = [ "https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xml" "https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xml" ];')
lines.append('  };')
lines.append(f'  document_count = {len(docs)};')
lines.append(f'  registry_count = {registry_count};')
lines.append(f'  record_count = {record_count};')
lines.append('  documents = [')
for d in docs:
    lines.append('    {')
    lines.append(f'      id = {attr_escape(d["id"])};')
    lines.append(f'      path = {attr_escape(d["path"])};')
    lines.append(f'      title = {esc(d["title"])};')
    lines.append(f'      record_count = {d["record_count"]};')
    lines.append('      registries = [')
    for rid,rtitle,records in d['registries']:
        lines.append('        {')
        lines.append(f'          id = {attr_escape(rid)};')
        lines.append(f'          title = {esc(rtitle)};')
        lines.append(f'          record_count = {len(records)};')
        lines.append('          records = [')
        for fields,xrefs in records:
            lines.append('            {')
            lines.append('              fields = [')
            for k,v in fields:
                lines.append(f'                {{ name = {attr_escape(k)}; value = {esc(v)}; }}')
            lines.append('              ];')
            lines.append('              xrefs = [ ' + ' '.join(esc(x) for x in xrefs) + ' ];')
            lines.append('            }')
        lines.append('          ];')
        lines.append('        }')
    lines.append('      ];')
    lines.append('    }')
lines.append('  ];')
lines.append('}')
out.write_text('\n'.join(lines)+'\n', encoding='utf-8')
print(f'generated {out}: documents={len(docs)} registries={registry_count} records={record_count}')
PY
