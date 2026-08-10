#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RECEIPT="$ROOT/ingest/code/scim-schema-tokens/source-receipt.json"
OUT="$ROOT/stdlib/lib/corpus/scim-schema-tokens.generated.px"
if [[ ! -f "$RECEIPT" ]]; then echo "missing $RECEIPT; run update first" >&2; exit 1; fi
python3 - "$RECEIPT" "$OUT" <<'PY'
import json, sys
from pathlib import Path
receipt=json.loads(Path(sys.argv[1]).read_text()); out=Path(sys.argv[2])
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
resource_types=[
 {"name":"User","endpoint":"/Users","source_rfc":"RFC7643"},
 {"name":"Group","endpoint":"/Groups","source_rfc":"RFC7643"},
 {"name":"EnterpriseUser","endpoint":"extension","source_rfc":"RFC7643"},
 {"name":"ServiceProviderConfig","endpoint":"/ServiceProviderConfig","source_rfc":"RFC7643"},
 {"name":"ResourceType","endpoint":"/ResourceTypes","source_rfc":"RFC7643"},
 {"name":"Schema","endpoint":"/Schemas","source_rfc":"RFC7643"}
]
attrs_user='id externalId meta userName name displayName nickName profileUrl title userType preferredLanguage locale timezone active password emails phoneNumbers ims photos addresses groups entitlements roles x509Certificates'.split()
attrs_group='id externalId meta displayName members'.split()
attrs_enterprise='employeeNumber costCenter organization division department manager'.split()
attributes=[]
for a in attrs_user: attributes.append({"resource":"User","name":a,"source_rfc":"RFC7643"})
for a in attrs_group: attributes.append({"resource":"Group","name":a,"source_rfc":"RFC7643"})
for a in attrs_enterprise: attributes.append({"resource":"EnterpriseUser","name":a,"source_rfc":"RFC7643"})
endpoints='/Users /Groups /Me /ServiceProviderConfig /Schemas /ResourceTypes /Bulk /Search'.split()
methods='GET POST PUT PATCH DELETE'.split()
query_params='filter sortBy sortOrder startIndex count attributes excludedAttributes'.split()
patch_ops='add remove replace'.split()
data={"schema":"code.scim.schema_tokens.v1","source":"SCIM RFC 7643/7644 schema/protocol token metadata","license":"IETF Trust Legal Provisions / token-only metadata","rfcs":[{"id":"RFC7643","url":"https://www.rfc-editor.org/rfc/rfc7643"},{"id":"RFC7644","url":"https://www.rfc-editor.org/rfc/rfc7644"}],"source_receipt":receipt,"resource_types":resource_types,"attributes":attributes,"endpoints":[{"path":e,"source_rfc":"RFC7644"} for e in endpoints],"methods":[{"name":m,"source_rfc":"RFC7644"} for m in methods],"query_parameters":[{"name":q,"source_rfc":"RFC7644"} for q in query_params],"patch_operations":[{"name":p,"source_rfc":"RFC7644"} for p in patch_ops],"exclusions":["RFC body text","examples","prose explanations","live IAM exports","user/group records","credentials","authorization decisions","provisioning execution","mirror/graph wiring"]}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: resources={len(resource_types)} attributes={len(attributes)} endpoints={len(endpoints)} methods={len(methods)} bytes={out.stat().st_size}")
PY
