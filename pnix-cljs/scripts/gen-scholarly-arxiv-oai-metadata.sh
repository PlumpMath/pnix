#!/usr/bin/env bash
# arXiv OAI-PMH metadata snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ARXIV_OAI_SRC:-$ROOT/ingest/scholarly/arxiv-oai-metadata}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/arxiv-oai-metadata.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing arXiv OAI metadata snapshot: $SRC" >&2
  echo "run scripts/update-scholarly-arxiv-oai-metadata.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
NS={'o':'http://www.openarchives.org/OAI/2.0/','a':'http://arxiv.org/OAI/arXiv/'}
def text(node, path, ns=NS):
    x=node.find(path, ns)
    return x.text.strip() if x is not None and x.text else None
def parse_xml(rel):
    p=src/rel
    return ET.parse(p).getroot()
identity={}; metadata_formats=[]; sets=[]; records=[]
try:
    root=parse_xml('raw/Identify.xml'); ident=root.find('o:Identify',NS)
    if ident is not None:
        identity={'repository_name':text(ident,'o:repositoryName'),'base_url':text(ident,'o:baseURL'),'protocol_version':text(ident,'o:protocolVersion'),'earliest_datestamp':text(ident,'o:earliestDatestamp'),'deleted_record':text(ident,'o:deletedRecord'),'granularity':text(ident,'o:granularity'),'admin_email_count':len(ident.findall('o:adminEmail',NS))}
except Exception as e:
    identity={'parse_error':str(e)}
try:
    root=parse_xml('raw/ListMetadataFormats.xml')
    for mf in root.findall('.//o:metadataFormat',NS):
        metadata_formats.append({'metadata_prefix':text(mf,'o:metadataPrefix'),'schema':text(mf,'o:schema'),'metadata_namespace':text(mf,'o:metadataNamespace')})
except Exception as e:
    metadata_formats.append({'parse_error':str(e)})
try:
    root=parse_xml('raw/ListSets.xml')
    for s in root.findall('.//o:set',NS):
        sets.append({'set_spec':text(s,'o:setSpec'),'set_name':text(s,'o:setName')})
except Exception as e:
    sets.append({'parse_error':str(e)})
try:
    root=parse_xml('raw/ListRecords.xml')
    for rec in root.findall('.//o:record',NS):
        header=rec.find('o:header',NS)
        meta=rec.find('o:metadata/a:arXiv',NS)
        if header is None: continue
        row={'oai_identifier':text(header,'o:identifier'),'datestamp':text(header,'o:datestamp'),'set_specs':[x.text for x in header.findall('o:setSpec',NS) if x.text]}
        if meta is not None:
            arxiv_id=text(meta,'a:id')
            cats=(text(meta,'a:categories') or '').split()
            authors=meta.findall('a:authors/a:author',NS)
            row.update({'arxiv_id':arxiv_id,'created':text(meta,'a:created'),'updated':text(meta,'a:updated'),'categories':cats,'doi':text(meta,'a:doi'),'license':text(meta,'a:license'),'has_title':text(meta,'a:title') is not None,'author_count':len(authors),'has_abstract':text(meta,'a:abstract') is not None,'has_comments':text(meta,'a:comments') is not None,'has_journal_ref':text(meta,'a:journal-ref') is not None})
        records.append(row)
except Exception as e:
    records.append({'parse_error':str(e)})
limit=receipt.get('record_limit') or 100
records=records[:limit]
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role'),'verb':f.get('verb')} for f in receipt.get('files') or []]
obj={'schema':'scholarly.arxiv.oai_metadata.v1','source':{'name':'arXiv OAI-PMH descriptive metadata snapshot','license':'CC0-1.0 for descriptive metadata','source_urls':['https://info.arxiv.org/help/oa/index.html','https://info.arxiv.org/help/oa/metadataPolicy.html','https://info.arxiv.org/help/api/tou.html','https://info.arxiv.org/help/bulk_data.html','https://export.arxiv.org/oai2'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope'),'from':receipt.get('from'),'until':receipt.get('until'),'metadata_prefix':receipt.get('metadata_prefix'),'record_limit':receipt.get('record_limit')},'generator':'scripts/gen-scholarly-arxiv-oai-metadata.sh','scope':'OAI capability/set/format metadata plus bounded identifier/category/license/date rows only; prose/person/fulltext excluded'},'summary':{'metadata_format_count':len(metadata_formats),'set_count':len(sets),'record_count':len(records),'record_limit':limit,'fulltext_payloads_downloaded':False,'pdf_tex_source_files_ingested':False,'abstract_title_author_values_ingested':False,'comments_journal_ref_values_ingested':False,'bulk_s3_payloads_ingested':False,'linked_payloads_ingested':False,'profiling_or_ranking_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'identity':identity,'metadata_formats':metadata_formats,'sets':sets,'records':records}
def pnix(v,indent=0):
    import json
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v,ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n'+''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v)+sp+']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n'+'\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v))+'\n'+sp+'}'
    return json.dumps(str(v),ensure_ascii=False)
content='# stdlib/lib/corpus/arxiv-oai-metadata.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-scholarly-arxiv-oai-metadata.sh && scripts/gen-scholarly-arxiv-oai-metadata.sh\n'
content+='# 범위: arXiv OAI metadata identifiers/categories/license/date only. fulltext/prose/person/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: formats={len(metadata_formats)} sets={len(sets)} records={len(records)} bytes={len(content.encode())}')
PY
