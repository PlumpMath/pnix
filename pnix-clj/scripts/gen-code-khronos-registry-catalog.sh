#!/usr/bin/env bash
# Khronos registry/header snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${KHRONOS_REGISTRY_SRC:-$ROOT/ingest/code/khronos-registry-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/khronos-registry-catalog.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing Khronos registry snapshot: $SRC" >&2
  echo "run scripts/update-code-khronos-registry-catalog.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
def text_of(e, tag):
    c=e.find(tag)
    return ''.join(c.itertext()).strip() if c is not None else None
def attrs(e, keys):
    return {k:e.attrib.get(k) for k in keys if e.attrib.get(k) not in (None,'')}
# Vulkan XML structural rows
vkroot=ET.parse(src/'raw/vulkan/vk.xml').getroot()
LIMITS={'vk_types':20,'vk_enums':50,'vk_commands':20,'vk_features':5,'vk_extensions':20,'spirv_instructions':50,'spirv_operand_enums':10,'opencl_defines':50,'opencl_typedefs':30,'opencl_functions':30}
vk_types=[]
for t in vkroot.findall('./types/type'):
    name=t.attrib.get('name') or text_of(t,'name')
    if name:
        r={'name':name}; r.update(attrs(t,['category','requires','alias','api','parent','objtypeenum','returnedonly','structextends']))
        members=[]
        for m in t.findall('member')[:32]:
            mn=text_of(m,'name')
            if mn: members.append({'name':mn,'type':text_of(m,'type'),'optional':m.attrib.get('optional'),'len':m.attrib.get('len')})
        if members: r['members']=members
        vk_types.append(r)
vk_enums=[]
for group in vkroot.findall('./enums'):
    gname=group.attrib.get('name') or group.attrib.get('type') or 'anonymous'
    for en in group.findall('enum'):
        name=en.attrib.get('name')
        if name:
            r={'group':gname,'name':name}; r.update(attrs(en,['value','bitpos','alias','extends','extnumber','offset','dir','protect','api','type']))
            vk_enums.append(r)
vk_commands=[]
for c in vkroot.findall('./commands/command'):
    if c.attrib.get('alias'): continue
    name=text_of(c,'./proto/name') or text_of(c,'name')
    if name:
        params=[]
        for p in c.findall('param')[:24]:
            pn=text_of(p,'name')
            if pn: params.append({'name':pn,'type':text_of(p,'type'),'optional':p.attrib.get('optional'),'len':p.attrib.get('len')})
        vk_commands.append({'name':name,'return_type':text_of(c,'./proto/type'),'param_count':len(params),'params':params})
vk_features=[]
for f in vkroot.findall('./feature'):
    refs=[]
    for req in f.findall('require'):
        for k in ('type','enum','command'):
            for x in req.findall(k)[:80]:
                nm=x.attrib.get('name')
                if nm: refs.append({'kind':k,'name':nm})
    r=attrs(f,['api','name','number','comment']); r['ref_count']=len(refs); r['refs']=refs[:240]
    vk_features.append(r)
vk_exts=[]
for e in vkroot.findall('./extensions/extension'):
    refs=[]
    for req in e.findall('require'):
        for k in ('type','enum','command'):
            for x in req.findall(k)[:80]:
                nm=x.attrib.get('name')
                if nm: refs.append({'kind':k,'name':nm})
    r=attrs(e,['name','number','type','supported','promotedto','deprecatedby','protect','platform','author','contact']); r['ref_count']=len(refs); r['refs']=refs[:120]
    vk_exts.append(r)
# SPIR-V grammar
spg=json.loads((src/'raw/spirv/spirv.core.grammar.json').read_text(encoding='utf-8'))
spirv_inst=[]
for ins in spg.get('instructions',[])[:LIMITS['spirv_instructions']]:
    spirv_inst.append({'opname':ins.get('opname'),'opcode':ins.get('opcode'),'class':ins.get('class'),'capabilities':ins.get('capabilities') or [],'extensions':ins.get('extensions') or [],'operands':[{'kind':o.get('kind'),'name':o.get('name'),'quantifier':o.get('quantifier')} for o in ins.get('operands',[])[:12]]})
spirv_kinds=[]
for ok in spg.get('operand_kinds',[]):
    enums=[]
    for en in ok.get('enumerants',[])[:LIMITS['spirv_operand_enums']]:
        enums.append({'enumerant':en.get('enumerant'),'value':en.get('value'),'capabilities':en.get('capabilities') or [],'extensions':en.get('extensions') or []})
    spirv_kinds.append({'kind':ok.get('kind'),'category':ok.get('category'),'bases':ok.get('bases') or [],'enumerant_count':len(ok.get('enumerants',[]) or []),'enumerants':enums})
spxml=ET.parse(src/'raw/spirv/spir-v.xml').getroot()
spirv_xml=[]
for child in list(spxml)[:500]:
    r={'tag':child.tag}; r.update({k:v for k,v in child.attrib.items() if v})
    vals=[]
    for g in list(child)[:120]:
        x={'tag':g.tag}; x.update({k:v for k,v in g.attrib.items() if v})
        vals.append(x)
    if vals: r['children']=vals
    spirv_xml.append(r)
# OpenCL headers token rows
header_files=[]; defines=[]; typedefs=[]; funcs=[]
for rel in ['raw/opencl/opencl.h','raw/opencl/cl.h','raw/opencl/cl_platform.h','raw/opencl/cl_ext.h']:
    txt=(src/rel).read_text(encoding='utf-8',errors='ignore')
    header_files.append({'relative_path':rel,'line_count':txt.count('\n')+1})
    for m in re.finditer(r'^#define\s+(CL_[A-Za-z0-9_]+)\s+([^/\n]+)', txt, re.M):
        name,val=m.group(1),m.group(2).strip()
        if len(val) <= 120: defines.append({'header':rel,'name':name,'value':val})
    for m in re.finditer(r'typedef\s+(?:struct\s+)?[^;{\n]+?\s+(cl_[A-Za-z0-9_]+)\s*;', txt):
        typedefs.append({'header':rel,'name':m.group(1)})
    for m in re.finditer(r'CL_API_ENTRY\s+(.+?)\s+CL_API_CALL\s+(cl[A-Za-z0-9_]+)\s*\((.*?)\)', txt, re.S):
        ret=re.sub(r'\s+',' ',m.group(1)).strip()
        params=re.sub(r'\s+',' ',m.group(3)).strip()
        funcs.append({'header':rel,'name':m.group(2),'return_type':ret,'param_count':0 if params in ('void','') else params.count(',')+1})
defines=defines[:LIMITS['opencl_defines']]; typedefs=typedefs[:LIMITS['opencl_typedefs']]; funcs=funcs[:LIMITS['opencl_functions']]
obj={'schema':'code.khronos.registry_catalog.v1','source':{'name':'Khronos Vulkan/SPIR-V/OpenCL machine-readable registry catalog','license':'Vulkan vk.xml Apache-2.0 OR MIT; SPIR-V MIT; OpenCL-Headers Apache-2.0','source_urls':['https://github.com/KhronosGroup/Vulkan-Headers','https://github.com/KhronosGroup/SPIRV-Headers','https://github.com/KhronosGroup/OpenCL-Headers'],'receipt':receipt,'generator':'scripts/gen-code-khronos-registry-catalog.sh','scope':'registry/header structural tokens only; spec prose/examples/generated SDK code/implementation bodies/conformance payloads/execution/mirror graph wiring excluded'},'summary':{'limits':LIMITS,'vulkan_type_count_total':len(vk_types),'vulkan_enum_count_total':len(vk_enums),'vulkan_command_count_total':len(vk_commands),'vulkan_feature_count_total':len(vk_features),'vulkan_extension_count_total':len(vk_exts),'spirv_instruction_count_stored':len(spirv_inst),'spirv_operand_kind_count':len(spirv_kinds),'opencl_define_count_stored':len(defines),'opencl_typedef_count_stored':len(typedefs),'opencl_function_count_stored':len(funcs),'specification_prose_ingested':False,'examples_ingested':False,'implementation_source_bodies_ingested':False,'shader_or_program_corpus_ingested':False,'conformance_payloads_ingested':False,'runtime_execution_enabled':False,'mirror_graph_wiring':False},'vulkan':{'types':vk_types[:LIMITS['vk_types']],'enums':vk_enums[:LIMITS['vk_enums']],'commands':vk_commands[:LIMITS['vk_commands']],'features':vk_features[:LIMITS['vk_features']],'extensions':vk_exts[:LIMITS['vk_extensions']]},'spirv':{'instructions':spirv_inst,'operand_kinds':spirv_kinds,'xml_registry_rows':spirv_xml[:10]},'opencl':{'header_files':header_files,'defines':defines,'typedefs':typedefs,'functions':funcs}}
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
content='# stdlib/lib/corpus/khronos-registry-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-khronos-registry-catalog.sh && scripts/gen-code-khronos-registry-catalog.sh\n'
content+='# 범위: Khronos registry/header structural tokens only. prose/examples/source bodies/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: vk_types={len(vk_types)} vk_enums={len(vk_enums)} vk_commands={len(vk_commands)} spirv_inst={len(spirv_inst)} opencl_defines={len(defines)} bytes={len(content.encode())}')
PY
