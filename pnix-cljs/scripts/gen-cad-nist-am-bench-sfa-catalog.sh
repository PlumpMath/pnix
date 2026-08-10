#!/usr/bin/env bash
# NIST AM-Bench + STEP File Analyzer catalog JSON/HTML receipts -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="${NIST_AM_SFA_SRC:-$ROOT/ingest/cad/nist-am-bench-sfa-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/nist-am-bench-sfa-catalog.generated.px}"
RECEIPT="$SRC_DIR/source-receipt.json"
if [[ ! -f "$RECEIPT" ]]; then
  echo "missing NIST AM/SFA source receipt: $RECEIPT" >&2
  echo "run scripts/update-cad-nist-am-bench-sfa-catalog.sh first" >&2
  exit 2
fi
python3 - "$SRC_DIR" "$OUT" "$RECEIPT" <<'PY'
import json, pathlib, sys
src_dir, out, receipt_path = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
repos=[]
for rp in sorted((src_dir/'github').glob('*.repo.json')):
    obj=json.loads(rp.read_text(encoding='utf-8'))
    tagp=rp.with_name(rp.name.replace('.repo.json','.tags.json'))
    try: tags=json.loads(tagp.read_text(encoding='utf-8'))
    except Exception: tags=[]
    lic=obj.get('license') or {}
    repos.append({
      'full_name':obj.get('full_name'),
      'html_url':obj.get('html_url'),
      'description_sha256':__import__('hashlib').sha256((obj.get('description') or '').encode()).hexdigest() if obj.get('description') else None,
      'default_branch':obj.get('default_branch'),
      'created_at':obj.get('created_at'),
      'updated_at':obj.get('updated_at'),
      'pushed_at':obj.get('pushed_at'),
      'archived':obj.get('archived'),
      'disabled':obj.get('disabled'),
      'fork':obj.get('fork'),
      'license_spdx_id':lic.get('spdx_id') if isinstance(lic,dict) else None,
      'license_name':lic.get('name') if isinstance(lic,dict) else None,
      'tags': [{'name':t.get('name'),'commit_sha':(t.get('commit') or {}).get('sha')} for t in tags[:20]],
      'source_contents_ingested':False,
      'readme_body_ingested':False,
    })
pages=[]
for p in receipt.get('pages',[]):
    pages.append({'slug':p.get('slug'),'url':p.get('url'),'sha256':p.get('sha256'),'size_bytes':p.get('size_bytes'),'content_type':p.get('content_type'),'html_body_ingested':False})
obj={
 'schema':'cad.nist_am_bench_sfa_catalog.v1',
 'source':{
   'name':'NIST AM-Bench and STEP File Analyzer public project/repository catalog metadata',
   'license':'NIST public metadata / public domain where applicable; repository code license not asserted by this ingest',
   'source_urls':['https://www.nist.gov/ambench','https://www.nist.gov/services-resources/software/step-file-analyzer-and-viewer','https://github.com/usnistgov/SFA','https://github.com/usnistgov/ambench','https://github.com/usnistgov/AMB2022-template','https://www.nist.gov/open/license'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-cad-nist-am-bench-sfa-catalog.sh',
   'scope':'project/repository catalog metadata only; source code/prose/benchmark data/CAD/STEP/toolpath/process payloads excluded'
 },
 'summary':{
   'page_count':len(pages),
   'repository_count':len(repos),
   'source_contents_ingested':False,
   'readme_or_page_bodies_ingested':False,
   'benchmark_measurement_data_ingested':False,
   'am_process_parameters_ingested':False,
   'cad_step_payloads_ingested':False,
   'toolpath_or_process_guidance_ingested':False,
   'mirror_graph_wiring':False,
 },
 'pages':pages,
 'repositories':repos,
}
def pnix(v, indent=0):
    import json
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
content='# stdlib/lib/corpus/nist-am-bench-sfa-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-cad-nist-am-bench-sfa-catalog.sh && scripts/gen-cad-nist-am-bench-sfa-catalog.sh\n'
content+='# 범위: NIST AM-Bench/SFA project/repository catalog metadata only. source/data/CAD/STEP/toolpath/process guidance 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: pages={len(pages)} repos={len(repos)} bytes={len(content.encode())}')
PY
