#!/usr/bin/env bash
# DCMI Metadata Terms RDF vocabulary snapshot.
# Downloads official RDF/XML + Turtle vocabulary files. No documentation prose/examples/graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${DCMI_DEST:-$ROOT/ingest/metadata/dcmi-terms}"
RDF_URL="${DCMI_RDF_URL:-https://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_terms.rdf}"
TTL_URL="${DCMI_TTL_URL:-https://www.dublincore.org/specifications/dublin-core/dcmi-terms/dublin_core_terms.ttl}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$RDF_URL" "$TTL_URL" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); urls=[('dublin_core_terms.rdf',sys.argv[2],'dcmi_terms_rdf_xml'),('dublin_core_terms.ttl',sys.argv[3],'dcmi_terms_turtle_hash_only')]
rows=[]
for name,url,role in urls:
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest-dcmi/0.1'})
    raw=urllib.request.urlopen(req, timeout=60).read()
    rel=pathlib.Path('raw')/name
    (out/rel).write_bytes(raw)
    rows.append({'source_path':str(rel),'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'DCMI Metadata Terms RDF vocabulary','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://www.dublincore.org/specifications/dublin-core/dcmi-terms/','https://www.dublincore.org/about/copyright/']+[u for _,u,_ in urls],'license':'CC-BY-4.0 / DCMI attribution license family; vocabulary structure only','scope':'official RDF vocabulary term structure only; rdfs:comment prose, specification narrative, examples, linked payloads, and graph wiring excluded','files':rows,'comment_prose_downloaded_but_not_ingested':True,'specification_prose_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded DCMI terms snapshot: files={len(rows)} -> {out}')
PY
