#!/usr/bin/env bash
set -euo pipefail

# LLVM LangRef RST → pnix attrset source 생성.
# 원문 prose bulk 저장 금지: headings/labels/instruction syntax/attribute token 등 구조만 추출.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IN="$ROOT/ingest/code/llvm"
OUT="$ROOT/stdlib/lib/corpus/llvm-langref.generated.px"
SYNTAX_LIMIT="${PNIX_LLVM_SYNTAX_LIMIT:-240}"
ATTRIBUTE_LIMIT="${PNIX_LLVM_ATTRIBUTE_LIMIT:-360}"

python3 - "$IN" "$OUT" "$SYNTAX_LIMIT" "$ATTRIBUTE_LIMIT" <<'PY'
import hashlib, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
syntax_limit=int(sys.argv[3]); attribute_limit=int(sys.argv[4])
manifest_path=src/'source-manifest.json'
manifest=json.loads(manifest_path.read_text(encoding='utf-8')) if manifest_path.exists() else {}
lines=(src/'LangRef.rst').read_text(encoding='utf-8', errors='replace').splitlines()

def pnix(v):
    if v is None: return 'null'
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False).replace('${','\\${')
    if isinstance(v,list): return '[ ' + ' '.join(pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {pnix(v[k])};' for k in sorted(v.keys())) + ' }'
    raise TypeError(type(v))

source_files=[]
for rel in ['LICENSE.TXT','LangRef.rst']:
    b=(src/rel).read_bytes()
    source_files.append({'path':rel,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'lines':len(b.decode('utf-8','replace').splitlines())})

labels_by_line={}
for i,l in enumerate(lines,1):
    m=re.match(r'\.\. _([^:]+):\s*$', l)
    if m: labels_by_line[i]=m.group(1)

headings=[]; label_stack=[]
for i in range(len(lines)-1):
    title=lines[i].strip(); under=lines[i+1].strip()
    if not title or len(under)<3: continue
    if len(set(under))==1 and under[0] in '=~-^"`#*+':
        # 가까운 바로 위 label만 구조 id로 붙임
        label=''
        for j in range(i, max(-1,i-4), -1):
            if (j+1) in labels_by_line:
                label=labels_by_line[j+1]; break
        level='=~-^"`#*+'.index(under[0])
        kind='section'
        if label.startswith('i_') or 'Instruction' in title: kind='instruction_heading'
        elif label.startswith('t_') or title.endswith(' Type') or title.endswith(' Types') or 'Type System' in title: kind='type_heading'
        elif 'Attribute' in title or 'attribute' in title or label.endswith('attrs'): kind='attribute_heading'
        elif 'Metadata' in title or 'metadata' in title: kind='metadata_heading'
        if kind != 'section':
            headings.append({'line':i+1,'level':level,'kind':kind,'label':label,'title':re.sub(r'[`*]+','',title)[:120]})

instructions=[]
for h in headings:
    if h['kind']=='instruction_heading':
        m=re.search(r'``([^`]+)``', h['title']) or re.search(r"'([^']+)'", h['title'])
        name=''
        if h['label'].startswith('i_'): name=h['label'][2:].replace('_','.')
        if not name:
            name=h['title'].replace(' Instruction','').replace("'",'').strip()
        instructions.append({'line':h['line'],'label':h['label'],'name':name})

syntax_rows=[]
for inst in instructions:
    start=inst['line']-1
    end=len(lines)
    for h in headings:
        if h['line']>inst['line'] and h['kind']=='instruction_heading':
            end=h['line']-1; break
    for idx in range(start, min(end,start+140)):
        if lines[idx].strip()=='Syntax:':
            j=idx+1
            while j<end and ('code-block:: llvm' not in lines[j]) and (lines[j].strip() != '::') and j<idx+12: j+=1
            if j<end and (('code-block:: llvm' in lines[j]) or (lines[j].strip() == '::')):
                k=j+1
                while k<end and len(syntax_rows)<syntax_limit:
                    raw=lines[k]
                    if raw.strip()=='' and k>j+1 and (k+1>=end or not lines[k+1].startswith('   ')): break
                    if raw.startswith('   ') and raw.strip():
                        text=raw.strip()
                        if not text.startswith(';'):
                            syntax_rows.append({'instruction':inst['name'],'line':k+1,'syntax':text[:180]})
                    elif k>j+1 and raw and not raw.startswith('   '):
                        break
                    k+=1
            break
        if len(syntax_rows)>=syntax_limit: break
    if len(syntax_rows)>=syntax_limit: break

attr_tokens=[]; seen=set(); in_attr=False
for i,l in enumerate(lines,1):
    if i in labels_by_line and (labels_by_line[i].endswith('attrs') or labels_by_line[i] in {'paramattrs','fnattrs','callsiteattrs','glattrs'}):
        in_attr=True
    if in_attr:
        for tok in re.findall(r'``([A-Za-z_][A-Za-z0-9_.-]*(?:\([^`]*\))?)``', l):
            base=tok.split('(')[0]
            if len(base)>1 and base not in seen and len(attr_tokens)<attribute_limit:
                seen.add(base); attr_tokens.append({'line':i,'name':base,'raw_token':tok[:80]})
    if in_attr and re.match(r'^[-=~^"`#*+]{3,}\s*$', l.strip()) and i>2000:
        # 다음 큰 섹션에서 자연 종료는 엄밀히 추적하지 않고 토큰 limit로 제어한다.
        pass

# 타입/메타데이터 heading만 별도 뷰로 제공
type_rows=[{'line':h['line'],'label':h['label'],'title':h['title']} for h in headings if h['kind']=='type_heading']
metadata_rows=[{'line':h['line'],'label':h['label'],'title':h['title']} for h in headings if h['kind']=='metadata_heading']

payload={
 'schema':'code.llvm.langref.v1',
 'source':{'project':'LLVM Language Reference Manual','repo':manifest.get('repo','https://github.com/llvm/llvm-project'),'commit_sha':manifest.get('commit_sha',''),'retrieved_at':manifest.get('retrieved_at','2026-06-19'),'license_id':'Apache-2.0 WITH LLVM-exception','scope':'LangRef structural rows only; bounded overlay; no prose/examples bulk; no graph/math wiring'},
 'attribution':'LLVM Project LangRef, Apache-2.0 WITH LLVM-exception, https://github.com/llvm/llvm-project',
 'source_files':source_files,
 'headings':headings,
 'instructions':instructions,
 'instruction_syntax':syntax_rows,
 'attributes':attr_tokens,
 'types':type_rows,
 'metadata':metadata_rows,
 'counts':{'source_files':len(source_files),'headings':len(headings),'instructions':len(instructions),'instruction_syntax':len(syntax_rows),'attributes':len(attr_tokens),'types':len(type_rows),'metadata':len(metadata_rows)},
 'limits':{'instruction_syntax':syntax_limit,'attributes':attribute_limit}
}
out.write_text('# GENERATED by scripts/gen-code-llvm-langref.sh. Do not edit.\n'+pnix(payload)+'\n',encoding='utf-8')
print('generated '+str(out)+': '+', '.join(f'{k}={v}' for k,v in payload['counts'].items()))
PY
