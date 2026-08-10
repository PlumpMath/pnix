#!/usr/bin/env bash
# CPC scheme/title/symbol snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CPC_SCHEME_SRC:-$ROOT/ingest/patent/uspto-cpc-scheme}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/uspto-cpc-scheme.generated.px}"
TITLE_LIMIT="${CPC_TITLE_LIMIT:-1000}"
SYMBOL_LIMIT="${CPC_SYMBOL_LIMIT:-1000}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing CPC scheme snapshot: $SRC" >&2
  echo "run scripts/update-patent-uspto-cpc-scheme.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$TITLE_LIMIT" "$SYMBOL_LIMIT" <<'PY'
import collections, csv, io, json, pathlib, sys, zipfile
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); title_limit=int(sys.argv[3]); symbol_limit=int(sys.argv[4])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; symbols=[]; title_rows=[]; sections=[]
level_counts=collections.Counter(); status_counts=collections.Counter(); flag_counts=collections.Counter(); title_count_by_section=collections.Counter()
for f in receipt.get('files') or []:
    source_files.append({'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role')})
    p=src/f.get('relative_path')
    role=f.get('role')
    if role=='cpc_symbol_list_zip':
        with zipfile.ZipFile(p) as z:
            name=next(n for n in z.namelist() if n.lower().endswith('.csv'))
            text=io.TextIOWrapper(z.open(name), encoding='utf-8-sig', newline='')
            for row in csv.DictReader(text):
                sym=(row.get('SYMBOL') or '').strip()
                if not sym: continue
                level=(row.get('level') or '').strip(); status=(row.get('status') or '').strip()
                level_counts[level]+=1; status_counts[status]+=1
                for key in ['breakdown-code','not-allocatable','additional-only']:
                    if (row.get(key) or '').strip().upper()=='TRUE': flag_counts[key]+=1
                if len(symbols)<symbol_limit:
                    symbols.append({'symbol':sym,'level':level,'status':status,'sort_key':(row.get('sort-key') or '').strip(),'breakdown_code':(row.get('breakdown-code') or '').strip().upper()=='TRUE','not_allocatable':(row.get('not-allocatable') or '').strip().upper()=='TRUE','additional_only':(row.get('additional-only') or '').strip().upper()=='TRUE'})
    elif role=='cpc_title_list_zip':
        with zipfile.ZipFile(p) as z:
            for name in sorted(n for n in z.namelist() if n.lower().endswith('.txt')):
                section=name.split('cpc-section-',1)[-1][:1] if 'cpc-section-' in name else None
                for raw in z.read(name).decode('utf-8','replace').splitlines():
                    if not raw.strip(): continue
                    parts=raw.split('\t')
                    symbol=(parts[0] if parts else '').strip()
                    title=' '.join(x.strip() for x in parts[1:] if x.strip())
                    if not symbol or not title: continue
                    sec=symbol[:1]
                    title_count_by_section[sec]+=1
                    row={'symbol':symbol,'section':sec,'title':title}
                    if len(symbol)==1 and not any(s.get('section')==symbol for s in sections): sections.append({'section':symbol,'title':title})
                    if len(title_rows)<title_limit: title_rows.append(row)
level_rows=[{'level':k,'count':v} for k,v in sorted(level_counts.items(), key=lambda kv: (int(kv[0]) if str(kv[0]).isdigit() else 999, str(kv[0])))]
status_rows=[{'status':k,'count':v} for k,v in sorted(status_counts.items())]
flag_rows=[{'flag':k,'count':v} for k,v in sorted(flag_counts.items())]
title_count_rows=[{'section':k,'count':v} for k,v in sorted(title_count_by_section.items())]
obj={'schema':'patent.uspto_cpc.scheme.v1','source':{'name':'Cooperative Patent Classification official scheme/title/symbol metadata','license':'CPC official open data / public classification scheme','source_urls':['https://www.cooperativepatentclassification.org/cpcSchemeAndDefinitions/bulk','https://www.cooperativepatentclassification.org/cpcSchemeAndDefinitions/CPCopenLinkedData','https://www.uspto.gov/web/offices/pac/mpep/s905.html'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'version':receipt.get('version'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-patent-uspto-cpc-scheme.sh','scope':'official CPC scheme schema/symbol/title/validity metadata only; patent docs, definitions prose/PDF, MCF assignment payloads, and graph wiring excluded'},'summary':{'version':receipt.get('version'),'source_file_count':len(source_files),'section_count':len(sections),'symbol_sample_count':len(symbols),'title_sample_count':len(title_rows),'level_count_rows':len(level_rows),'status_count_rows':len(status_rows),'title_limit':title_limit,'symbol_limit':symbol_limit,'cpc_definitions_downloaded':False,'cpc_definition_prose_ingested':False,'patent_documents_ingested':False,'mcf_assignment_payloads_ingested':False,'linked_payloads_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'sections':sections,'level_counts':level_rows,'status_counts':status_rows,'flag_counts':flag_rows,'title_counts_by_section':title_count_rows,'symbol_rows':symbols,'title_rows':title_rows}
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
content='# stdlib/lib/corpus/uspto-cpc-scheme.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-patent-uspto-cpc-scheme.sh && scripts/gen-patent-uspto-cpc-scheme.sh\n'
content+='# 범위: CPC scheme/title/symbol metadata only. patent docs/definitions prose/MCF payload/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: sections={len(sections)} symbol_rows={len(symbols)} title_rows={len(title_rows)} bytes={len(content.encode())}')
PY
