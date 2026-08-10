#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RECEIPT="$ROOT/ingest/code/ietf-bcp14-keywords/source-receipt.json"
OUT="$ROOT/stdlib/lib/corpus/ietf-bcp14-keywords.generated.px"
if [[ ! -f "$RECEIPT" ]]; then echo "missing $RECEIPT; run update first" >&2; exit 1; fi
python3 - "$RECEIPT" "$OUT" <<'PY'
import json, sys
from pathlib import Path
receipt=json.loads(Path(sys.argv[1]).read_text())
out=Path(sys.argv[2])
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
keywords=[
  {"keyword":"MUST","polarity":"positive","strength":"absolute","class":"requirement","source_rfc":"RFC2119"},
  {"keyword":"MUST NOT","polarity":"negative","strength":"absolute","class":"prohibition","source_rfc":"RFC2119"},
  {"keyword":"REQUIRED","polarity":"positive","strength":"absolute","class":"requirement","source_rfc":"RFC2119"},
  {"keyword":"SHALL","polarity":"positive","strength":"absolute","class":"requirement","source_rfc":"RFC2119"},
  {"keyword":"SHALL NOT","polarity":"negative","strength":"absolute","class":"prohibition","source_rfc":"RFC2119"},
  {"keyword":"SHOULD","polarity":"positive","strength":"recommended","class":"recommendation","source_rfc":"RFC2119"},
  {"keyword":"SHOULD NOT","polarity":"negative","strength":"recommended","class":"discouragement","source_rfc":"RFC2119"},
  {"keyword":"RECOMMENDED","polarity":"positive","strength":"recommended","class":"recommendation","source_rfc":"RFC2119"},
  {"keyword":"NOT RECOMMENDED","polarity":"negative","strength":"recommended","class":"discouragement","source_rfc":"RFC8174"},
  {"keyword":"MAY","polarity":"positive","strength":"optional","class":"permission","source_rfc":"RFC2119"},
  {"keyword":"OPTIONAL","polarity":"positive","strength":"optional","class":"permission","source_rfc":"RFC2119"}
]
data={
  "schema":"code.ietf_bcp14.keywords.v1",
  "source":"IETF BCP 14 requirement keyword token metadata",
  "license":"IETF Trust Legal Provisions / token-only metadata",
  "bcp":"BCP14",
  "rfcs":[{"id":"RFC2119","url":"https://www.rfc-editor.org/rfc/rfc2119"},{"id":"RFC8174","url":"https://www.rfc-editor.org/rfc/rfc8174"}],
  "source_receipt":receipt,
  "keywords":keywords,
  "exclusions":["RFC body text","examples","prose explanations","derived excerpts","requirements engine execution","mirror/graph wiring"]
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: keywords={len(keywords)} sources={len(receipt.get('sources', []))} bytes={out.stat().st_size}")
PY
