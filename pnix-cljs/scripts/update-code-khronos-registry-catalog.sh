#!/usr/bin/env bash
# Khronos Vulkan/SPIR-V/OpenCL official machine-readable registry/header snapshot.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${KHRONOS_REGISTRY_DEST:-$ROOT/ingest/code/khronos-registry-catalog}"
mkdir -p "$DEST/raw/vulkan" "$DEST/raw/spirv" "$DEST/raw/opencl"
python3 - "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, urllib.request, sys
DEST=pathlib.Path(sys.argv[1])
UA='pnix-khronos-registry-ingest/1.0 (structural registry metadata only; no spec prose)'
FILES=[
  ('vulkan','vk.xml','raw/vulkan/vk.xml','https://raw.githubusercontent.com/KhronosGroup/Vulkan-Headers/main/registry/vk.xml','Apache-2.0 OR MIT'),
  ('spirv','spir-v.xml','raw/spirv/spir-v.xml','https://raw.githubusercontent.com/KhronosGroup/SPIRV-Headers/main/include/spirv/spir-v.xml','MIT'),
  ('spirv','spirv.core.grammar.json','raw/spirv/spirv.core.grammar.json','https://raw.githubusercontent.com/KhronosGroup/SPIRV-Headers/main/include/spirv/unified1/spirv.core.grammar.json','MIT'),
  ('opencl','opencl.h','raw/opencl/opencl.h','https://raw.githubusercontent.com/KhronosGroup/OpenCL-Headers/main/CL/opencl.h','Apache-2.0'),
  ('opencl','cl.h','raw/opencl/cl.h','https://raw.githubusercontent.com/KhronosGroup/OpenCL-Headers/main/CL/cl.h','Apache-2.0'),
  ('opencl','cl_platform.h','raw/opencl/cl_platform.h','https://raw.githubusercontent.com/KhronosGroup/OpenCL-Headers/main/CL/cl_platform.h','Apache-2.0'),
  ('opencl','cl_ext.h','raw/opencl/cl_ext.h','https://raw.githubusercontent.com/KhronosGroup/OpenCL-Headers/main/CL/cl_ext.h','Apache-2.0'),
  ('opencl','LICENSE','raw/opencl/LICENSE','https://raw.githubusercontent.com/KhronosGroup/OpenCL-Headers/main/LICENSE','Apache-2.0'),
]
files=[]
for family,name,rel,url,lic in FILES:
    req=urllib.request.Request(url,headers={'User-Agent':UA})
    raw=urllib.request.urlopen(req,timeout=90).read()
    p=DEST/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(raw)
    files.append({'family':family,'name':name,'relative_path':rel,'url':url,'license':lic,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Khronos Vulkan/SPIR-V/OpenCL registry catalog','retrieved_at':dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/KhronosGroup/Vulkan-Headers','https://github.com/KhronosGroup/SPIRV-Headers','https://github.com/KhronosGroup/OpenCL-Headers'],'license':'Vulkan vk.xml: Apache-2.0 OR MIT; SPIR-V registry: MIT; OpenCL headers: Apache-2.0','scope':'official machine-readable registry/header files only; no specification prose, examples, generated SDK code, conformance payloads, runtime execution, or mirror/graph/math wiring','files':files}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded Khronos registry catalog: files={len(files)} bytes={sum(f["size_bytes"] for f in files)} -> {DEST}')
PY
