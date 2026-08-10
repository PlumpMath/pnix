#!/usr/bin/env bash
# MDN browser-compat-data snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${MDN_BCD_SRC:-$ROOT/ingest/web/mdn-browser-compat-data}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/mdn-browser-compat-data.generated.px}"
LIMIT="${MDN_BCD_FEATURE_LIMIT:-500}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing MDN BCD snapshot: $SRC" >&2
  echo "run scripts/update-web-mdn-browser-compat-data.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$LIMIT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); limit=int(sys.argv[3])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
PROSE_KEYS={'description','notes','impl_url','mdn_url'}
def support_entries(support):
    rows=[]
    if not isinstance(support,dict): return rows
    for browser,val in sorted(support.items()):
        vals=val if isinstance(val,list) else [val]
        for item in vals[:3]:
            if not isinstance(item,dict): continue
            rows.append({'browser':browser,'version_added':item.get('version_added'),'version_removed':item.get('version_removed'),'partial_implementation':item.get('partial_implementation'),'flags_count':len(item.get('flags') or []),'prefix':item.get('prefix'),'alternative_name':item.get('alternative_name')})
    return rows[:24]
def walk(obj,path,category,rows):
    if isinstance(obj,dict):
        compat=obj.get('__compat')
        if isinstance(compat,dict):
            status=compat.get('status') if isinstance(compat.get('status'),dict) else {}
            rows.append({'category':category,'path':'.'.join(path),'spec_url':compat.get('spec_url') if isinstance(compat.get('spec_url'),str) else None,'status':{'experimental':status.get('experimental'),'deprecated':status.get('deprecated'),'standard_track':status.get('standard_track')},'support':support_entries(compat.get('support'))})
        for k,v in obj.items():
            if k == '__compat' or k in PROSE_KEYS: continue
            walk(v,path+[k],category,rows)
raw=src/'raw'
features=[]; json_file_count=0; total_feature_count=0
for f in sorted(raw.rglob('*.json')):
    rel=f.relative_to(raw)
    if rel.parts and rel.parts[0] == 'browsers': continue
    category=rel.parts[0] if rel.parts else 'unknown'
    try: data=json.loads(f.read_text(encoding='utf-8'))
    except Exception: continue
    before=len(features)
    local=[]; walk(data,[rel.stem],category,local)
    total_feature_count += len(local); json_file_count += 1
    for r in local:
        if len(features) < limit: features.append(r)
browsers=[]
for f in sorted((raw/'browsers').glob('*.json')) if (raw/'browsers').exists() else []:
    try: data=json.loads(f.read_text(encoding='utf-8'))
    except Exception: continue
    for bid,b in sorted(data.items()):
        if isinstance(b,dict):
            releases=b.get('releases') if isinstance(b.get('releases'),dict) else {}
            browsers.append({'id':bid,'name':b.get('name'),'type':b.get('type'),'preview_name':b.get('preview_name'),'release_count':len(releases),'current_release':b.get('current_release')})
package={}
pkg=raw/'package.json'
if pkg.exists():
    try:
        p=json.loads(pkg.read_text(encoding='utf-8'))
        package={'name':p.get('name'),'version':p.get('version'),'license':p.get('license')}
    except Exception: pass
receipt_summary={'retrieved_at':receipt.get('retrieved_at'),'ref':receipt.get('ref'),'archive_url':receipt.get('archive_url'),'archive_sha256':receipt.get('archive_sha256'),'file_count':len(receipt.get('files') or []),'license':receipt.get('license')}
obj={'schema':'web.mdn_browser_compat_data.v1','source':{'name':'MDN browser-compat-data','license':'CC0-1.0','source_urls':['https://github.com/mdn/browser-compat-data'],'receipt_summary':receipt_summary,'package':package,'generator':'scripts/gen-web-mdn-browser-compat-data.sh','scope':'bounded feature compatibility structure only; docs prose/examples/notes/telemetry/runtime probing/advice/graph wiring excluded'},'summary':{'json_file_count':json_file_count,'feature_count_total':total_feature_count,'feature_count_stored':len(features),'browser_count':len(browsers),'feature_limit':limit,'documentation_prose_ingested':False,'examples_ingested':False,'notes_ingested':False,'browser_telemetry_or_logs_ingested':False,'runtime_probe_enabled':False,'compatibility_advice_enabled':False,'mirror_graph_wiring':False},'browsers':browsers,'features':features}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x, indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/mdn-browser-compat-data.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-web-mdn-browser-compat-data.sh && scripts/gen-web-mdn-browser-compat-data.sh\n'
content+='# 범위: MDN browser compatibility structure only. docs prose/examples/notes/telemetry/runtime probing/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: features={len(features)}/{total_feature_count} browsers={len(browsers)} bytes={len(content.encode())}')
PY
