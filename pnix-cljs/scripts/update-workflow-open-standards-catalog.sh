#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${WORKFLOW_OPEN_STANDARDS_DEST:-$ROOT/ingest/workflow/open-standards-catalog}"
WDL_REF="${WDL_REF:-wdl-1.3}"
ROCRATE_REF="${ROCRATE_REF:-main}"
mkdir -p "$DEST/raw/wdl" "$DEST/raw/ro-crate"
python3 - "$DEST" "$WDL_REF" "$ROCRATE_REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, urllib.request, sys
DEST=pathlib.Path(sys.argv[1]); WDL_REF=sys.argv[2]; RO_REF=sys.argv[3]
UA='pnix-workflow-open-standards-ingest/1.0 (structural metadata only; no payloads/execution)'
FILES=[]
def fetch(source, rel, url, lic):
    req=urllib.request.Request(url,headers={'User-Agent':UA})
    raw=urllib.request.urlopen(req,timeout=90).read()
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    FILES.append({'source':source,'relative_path':rel,'url':url,'license':lic,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
fetch('wdl','raw/wdl/LICENSE',f'https://raw.githubusercontent.com/openwdl/wdl/{WDL_REF}/LICENSE','BSD-3-Clause')
fetch('wdl','raw/wdl/README.md',f'https://raw.githubusercontent.com/openwdl/wdl/{WDL_REF}/README.md','BSD-3-Clause')
fetch('wdl','raw/wdl/SPEC.md',f'https://raw.githubusercontent.com/openwdl/wdl/{WDL_REF}/SPEC.md','BSD-3-Clause')
for rel in ['LICENSE','docs/_specification/1.2/context.jsonld','docs/_specification/1.2/ro-crate-metadata.json','docs/_specification/1.2/ro-crate-metadata.jsonld']:
    fetch('ro-crate','raw/ro-crate/'+rel.replace('/','__'),f'https://raw.githubusercontent.com/ResearchObject/ro-crate/{RO_REF}/{rel}','Apache-2.0')
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'WDL and RO-Crate workflow/FAIR schema catalog','retrieved_at':dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),'wdl_ref':WDL_REF,'ro_crate_ref':RO_REF,'source_urls':['https://github.com/openwdl/wdl','https://github.com/ResearchObject/ro-crate'],'license':'WDL BSD-3-Clause; RO-Crate Apache-2.0','scope':'structural metadata only; no prose bodies, examples, real workflow/crate payloads, command lines, data files, execution, or graph/mirror wiring','files':FILES}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded workflow open standards catalog: files={len(FILES)} -> {DEST}')
PY
