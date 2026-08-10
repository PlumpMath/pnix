#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/bio/ncbi-gene-reference-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, os, pathlib, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
defaults=[
 ('human','https://ftp.ncbi.nlm.nih.gov/gene/DATA/GENE_INFO/Mammalia/Homo_sapiens.gene_info.gz'),
 ('mouse','https://ftp.ncbi.nlm.nih.gov/gene/DATA/GENE_INFO/Mammalia/Mus_musculus.gene_info.gz'),
 ('rat','https://ftp.ncbi.nlm.nih.gov/gene/DATA/GENE_INFO/Mammalia/Rattus_norvegicus.gene_info.gz'),
 ('drosophila','https://ftp.ncbi.nlm.nih.gov/gene/DATA/GENE_INFO/Invertebrates/Drosophila_melanogaster.gene_info.gz'),
 ('celegans','https://ftp.ncbi.nlm.nih.gov/gene/DATA/GENE_INFO/Invertebrates/Caenorhabditis_elegans.gene_info.gz'),
 ('zebrafish','https://ftp.ncbi.nlm.nih.gov/gene/DATA/GENE_INFO/Vertebrates/Danio_rerio.gene_info.gz')
]
files=[]; skipped=[]
for key,url in defaults:
    try:
        req=urllib.request.Request(url,headers={'User-Agent':'pnix-ncbi-gene-reference/1.0'})
        with urllib.request.urlopen(req,timeout=90) as r:
            data=r.read(); ctype=r.headers.get('content-type') or ''
    except Exception as e:
        skipped.append({'key':key,'url':url,'error':type(e).__name__+': '+str(e)[:160]}); continue
    rel=f'raw/{key}.gene_info.gz'
    (out/rel).write_bytes(data)
    files.append({'kind':'gene_info_gz','key':key,'url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'NCBI Gene organism-specific gene_info files','retrieved_at':datetime.date.today().isoformat(),'policy':'bounded gene identifier metadata only; no sequences/phenotypes/interactions/full all-species bulk','files':files,'skipped':skipped}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files),'skipped':len(skipped)},ensure_ascii=False))
PY
