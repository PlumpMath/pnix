#!/usr/bin/env bash
set -euo pipefail

# Lisp-family source snapshots → pnix attrset source 생성.
# raw source/prose/docstring 저장 금지: symbol/form/token fact rows만 추출.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IN="$ROOT/ingest/code/lisp-family"
OUT="$ROOT/stdlib/lib/corpus/lisp-family-refs.generated.px"
CL_LIMIT="${PNIX_LISP_CL_SYMBOL_LIMIT:-1000}"
RACKET_LIMIT="${PNIX_LISP_RACKET_FORM_LIMIT:-220}"
CHEZ_LIMIT="${PNIX_LISP_CHEZ_PRIM_LIMIT:-320}"

python3 - "$IN" "$OUT" "$CL_LIMIT" "$RACKET_LIMIT" "$CHEZ_LIMIT" <<'PY'
import hashlib,json,pathlib,re,sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
cl_limit=int(sys.argv[3]); racket_limit=int(sys.argv[4]); chez_limit=int(sys.argv[5])
manifest=json.loads((src/'source-manifest.json').read_text(encoding='utf-8'))

def pnix(v):
    if v is None: return 'null'
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return json.dumps(v,ensure_ascii=False).replace('${','\\${')
    if isinstance(v,list): return '[ ' + ' '.join(pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {pnix(v[k])};' for k in sorted(v.keys())) + ' }'
    raise TypeError(type(v))

def read(rel): return (src/rel).read_text(encoding='utf-8',errors='replace').splitlines()
def text(rel): return (src/rel).read_text(encoding='utf-8',errors='replace')

def source_files():
    rows=[]
    for inc in manifest.get('included',[]):
        sid=inc['source_id']
        for f in inc['files']:
            p=src/sid/f['path']; b=p.read_bytes()
            rows.append({'source':sid,'path':f['path'],'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'lines':len(b.decode('utf-8','replace').splitlines())})
    return rows

# Common Lisp symbols from SBCL ANSI export list.
cl_text=text('sbcl/src/cold/common-lisp-exports.lisp-expr')
cl_symbols=[]
for sym in re.findall(r'"([^"]+)"', cl_text):
    if len(cl_symbols) < cl_limit:
        cl_symbols.append({'name':sym,'package':'COMMON-LISP','source':'sbcl/common-lisp-exports'})
# Small reader function/token names from SBCL reader source; no bodies.
reader_names=[]; seen=set()
for i,line in enumerate(read('sbcl/src/code/reader.lisp'),1):
    m=re.match(r'^\(def(un|macro|constant|var|parameter)?\s+([*A-Za-z0-9+\-/<>=!?$%&_.:]+)', line.strip(), re.I)
    if m:
        name=m.group(2).upper()
        if name not in seen and len(reader_names)<160:
            seen.add(name); reader_names.append({'line':i,'name':name})

# Racket forms from Scribble structure macros, not prose body.
racket_forms=[]; seen=set()
syntax_lines=read('racket/pkgs/racket-doc/scribblings/reference/syntax.scrbl')
for i,line in enumerate(syntax_lines,1):
    for m in re.finditer(r'@(?:defform\*?/subs|defform/subs|defform\*|defform/none|defform|defidform|defproc)\s*\[([^\]\n]+)', line):
        chunk=m.group(1)
        sm=re.search(r'\(?\s*([#%λA-Za-z0-9_+*?!<>=./:-]+)', chunk)
        if sm:
            name=sm.group(1)
            if name and name not in seen and len(racket_forms)<racket_limit:
                seen.add(name); racket_forms.append({'line':i,'name':name,'kind':'syntax_form'})
# Reader quote table and litchar forms.
reader_tokens=[]
reader_text=text('racket/pkgs/racket-doc/scribblings/reference/reader.scrbl')
for m in re.finditer(r'@litchar(?:\["([^"]+)"\]|\{([^}]+)\}).*?@racket\[([^\]]+)\]', reader_text, re.S):
    token=m.group(1) or m.group(2)
    form=m.group(3)
    if len(token)<=8 and len(reader_tokens)<80:
        reader_tokens.append({'token':token,'form':form})
for m in re.finditer(r'@racket\[([A-Za-z][A-Za-z0-9?\-]+)\]', reader_text):
    name=m.group(1)
    if name.startswith('read-') and name not in seen and len(racket_forms)<racket_limit:
        seen.add(name); racket_forms.append({'line':reader_text[:m.start()].count('\n')+1,'name':name,'kind':'reader_parameter_or_proc'})

# Chez primitive entries from primdata, with current library context.
chez_prims=[]; current_lib=[]; current_flags=[]
for i,line in enumerate(read('chez/s/primdata.ss'),1):
    s=line.strip()
    lm=re.match(r'\(define-symbol-flags\*\s+\(\[libraries\s+([^\]]+)\]\s+\[flags\s+([^\]]*)\]', s)
    if lm:
        current_lib=[x for x in re.findall(r'[A-Za-z0-9+\-*/<>=!?$%&_.:]+', lm.group(1)) if x not in {'rnrs'}] or ['rnrs']
        current_flags=re.findall(r'[A-Za-z0-9+\-*/<>=!?$%&_.:]+', lm.group(2))
        continue
    if current_lib and s.startswith(')'):
        current_lib=[]; current_flags=[]; continue
    if current_lib:
        em=re.match(r'\(\(?([A-Za-z0-9+\-*/<>=!?$%&_.:]+)\)?(?:\s|\[|\))', s)
        if em and em.group(1) not in {'define-symbol-flags*'} and len(chez_prims)<chez_limit:
            chez_prims.append({'line':i,'name':em.group(1),'libraries':current_lib,'flags':current_flags[:12]})
# Chez syntax/base names from syntax.ss/base-lang.ss definitions only.
chez_syntax=[]; seen2=set()
for rel in ['chez/s/syntax.ss','chez/s/base-lang.ss']:
    for i,line in enumerate(read(rel),1):
        m=re.match(r'^\s*\((define-syntax|define|module|define-record-type)\s+([A-Za-z0-9+\-*/<>=!?$%&_.:]+)', line)
        if m:
            key=(rel,m.group(2))
            if key not in seen2 and len(chez_syntax)<220:
                seen2.add(key); chez_syntax.append({'file':rel,'line':i,'kind':m.group(1),'name':m.group(2)})

inc={x['source_id']:x for x in manifest['included']}
payload={
 'schema':'code.lisp_family.forms.v1',
 'source':{'project':'Scheme/Racket/Common Lisp structural refs','retrieved_at':manifest.get('retrieved_at','2026-06-19'),'license_policy':manifest.get('license_policy',''),'scope':'fact rows only; no source/prose/docstring bodies; no graph/math wiring'},
 'attribution':'SBCL, Racket, and Chez Scheme source repositories; per-source license recorded; factual symbol/form rows only.',
 'source_files':source_files(),
 'common_lisp':{'record_schema':'code.common_lisp.forms.v1','source':inc['sbcl'],'symbols':cl_symbols,'reader_names':reader_names},
 'racket':{'record_schema':'code.racket.forms.v1','source':inc['racket'],'forms':racket_forms,'reader_tokens':reader_tokens},
 'chez_scheme':{'record_schema':'code.scheme.chez.forms.v1','source':inc['chez'],'primitives':chez_prims,'syntax_bindings':chez_syntax},
 'counts':{'source_files':len(source_files()),'common_lisp_symbols':len(cl_symbols),'common_lisp_reader_names':len(reader_names),'racket_forms':len(racket_forms),'racket_reader_tokens':len(reader_tokens),'chez_primitives':len(chez_prims),'chez_syntax_bindings':len(chez_syntax)},
 'limits':{'common_lisp_symbols':cl_limit,'racket_forms':racket_limit,'chez_primitives':chez_limit}
}
out.write_text('# GENERATED by scripts/gen-code-lisp-family-refs.sh. Do not edit.\n'+pnix(payload)+'\n',encoding='utf-8')
print('generated '+str(out)+': '+', '.join(f'{k}={v}' for k,v in payload['counts'].items()))
PY
