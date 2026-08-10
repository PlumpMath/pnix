#!/usr/bin/env bash
# PubChem PUG REST compound property updater. Bounded CID property rows only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/chem/pubchem-compound-properties"
CID_START="${PUBCHEM_CID_START:-1}"
CID_END="${PUBCHEM_CID_END:-250}"
mkdir -p "$DEST"
python3 - "$DEST" "$CID_START" "$CID_END" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
DEST=pathlib.Path(sys.argv[1]); start=int(sys.argv[2]); end=int(sys.argv[3])
props=[
 'MolecularFormula','MolecularWeight','CanonicalSMILES','IsomericSMILES','InChI','InChIKey','IUPACName',
 'XLogP','ExactMass','MonoisotopicMass','TPSA','Complexity','Charge','HBondDonorCount','HBondAcceptorCount',
 'RotatableBondCount','HeavyAtomCount','AtomStereoCount','DefinedAtomStereoCount','UndefinedAtomStereoCount',
 'BondStereoCount','DefinedBondStereoCount','UndefinedBondStereoCount','CovalentUnitCount'
]
cids=','.join(str(i) for i in range(start,end+1))
url=f'https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/cid/{cids}/property/{",".join(props)}/JSON'
req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0 (PubChem property rows only)'})
with urllib.request.urlopen(req,timeout=120) as r:
    raw=r.read(); final=r.geturl(); ctype=r.headers.get('content-type','')
(DEST/'compound-properties.json').write_bytes(raw)
obj=json.loads(raw)
rows=(obj.get('PropertyTable') or {}).get('Properties') or []
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'PubChem PUG REST compound property table',
 'version':f'snapshot-2026-06-19-cid-{start}-{end}',
 'url':url,
 'final_url':final,
 'sha256':hashlib.sha256(raw).hexdigest(),
 'size_bytes':len(raw),
 'content_type':ctype,
 'cid_start':start,
 'cid_end':end,
 'requested_cid_count':end-start+1,
 'returned_row_count':len(rows),
 'properties':props,
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'NIH/NCBI public domain core data with PubChem depositor/BioAssay caveat',
 'scope':'bounded PubChem Compound property rows only; synonyms, Substance/SID depositor records, BioAssay/AID, PUG-View prose, hazard/handling/synthesis guidance excluded'
}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated PubChem compound properties: cid={start}-{end} rows={len(rows)} sha={receipt["sha256"]} bytes={len(raw)}')
PY
