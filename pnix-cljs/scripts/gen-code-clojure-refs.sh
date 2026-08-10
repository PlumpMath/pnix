#!/usr/bin/env bash
set -euo pipefail

# Clojure/ClojureScript source snapshots → pnix attrset source 생성.
# raw code/prose/docstring 저장 금지: symbol/category/reader dispatch 같은 사실 구조만 추출.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IN="$ROOT/ingest/code/clojure"
OUT="$ROOT/stdlib/lib/corpus/clojure-refs.generated.px"
CORE_LIMIT="${PNIX_CLOJURE_CORE_LIMIT:-320}"

python3 - "$IN" "$OUT" "$CORE_LIMIT" <<'PY'
import hashlib, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); core_limit=int(sys.argv[3])
manifest=json.loads((src/'source-manifest.json').read_text(encoding='utf-8'))

def pnix(v):
    if v is None: return 'null'
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False).replace('${','\\${')
    if isinstance(v,list): return '[ ' + ' '.join(pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {pnix(v[k])};' for k in sorted(v.keys())) + ' }'
    raise TypeError(type(v))

def read(rel): return (src/rel).read_text(encoding='utf-8', errors='replace').splitlines()
def text(rel): return (src/rel).read_text(encoding='utf-8', errors='replace')

def source_files():
    rows=[]
    for inc in manifest.get('included',[]):
        sid=inc['source_id']
        for f in inc['files']:
            p=src/sid/f['path']; b=p.read_bytes()
            rows.append({'source':sid,'path':f['path'],'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'lines':len(b.decode('utf-8','replace').splitlines())})
    return rows

# Clojure compiler special symbols
comp=text('clojure/src/jvm/clojure/lang/Compiler.java')
consts={}
for m in re.finditer(r'static final Symbol\s+([A-Z0-9_]+)\s*=\s*(?:\(Symbol\)\s*)?Symbol\.intern\(([^;]+?)\);', comp):
    ident=m.group(1); args=m.group(2)
    strs=re.findall(r'"([^"]+)"', args)
    if len(strs)==1: consts[ident]=strs[0]
    elif len(strs)>=2: consts[ident]=strs[0]+'/'+strs[1]
body=comp[comp.find('static final public IPersistentMap specials'):comp.find('static public boolean isSpecial', comp.find('static final public IPersistentMap specials'))]
compiler_specials=[]
for m in re.finditer(r'\n\s*([A-Z0-9_]+),\s*new\s+([A-Za-z0-9_$.]+)\(', body):
    ident=m.group(1)
    compiler_specials.append({'symbol':consts.get(ident, ident), 'const':ident, 'parser':m.group(2)})
for m in re.finditer(r'\n\s*(FN),\s*null', body):
    compiler_specials.append({'symbol':consts.get(m.group(1), m.group(1)), 'const':m.group(1), 'parser':'null'})

# Reader macro tables
lr=read('clojure/src/jvm/clojure/lang/LispReader.java')
reader_macros=[]
for i,line in enumerate(lr,1):
    m=re.search(r"(dispatchMacros|macros)\['(.?)'\]\s*=\s*new\s+([A-Za-z0-9_]+Reader)\(", line)
    if m:
        table='dispatch' if m.group(1)=='dispatchMacros' else 'reader'
        ch=m.group(2)
        token=('#'+ch) if table=='dispatch' else ch
        reader_macros.append({'line':i,'table':table,'token':token,'reader':m.group(3)})
reader_symbolic=[]
for m in re.finditer(r'Symbol\.intern\("([^"]+)"\)', text('clojure/src/jvm/clojure/lang/LispReader.java')):
    val=m.group(1)
    if val in {'Inf','-Inf','NaN'}: reader_symbolic.append({'symbol':val})

# repl special-doc-map keys only, no docs/forms copied.
repl=read('clojure/src/clj/clojure/repl.clj')
repl_specials=[]; in_map=False
for i,line in enumerate(repl,1):
    if '(def ^:private special-doc-map' in line: in_map=True; continue
    if in_map:
        if line.startswith('    ') and re.match(r'\s{4}([A-Za-z0-9_.*+!?#<>=/-]+)\s+\{', line):
            name=re.match(r'\s{4}([^\s]+)\s+\{', line).group(1)
            repl_specials.append({'line':i,'symbol':name})
        if line.strip() == '}})':
            break

# def rows from core files; only name/kind/line.
def extract_defs(rel, source_id, ns):
    rows=[]
    for i,line in enumerate(read(rel),1):
        s=line.strip()
        m=re.match(r'^\((defn|defmacro|definline|defprotocol|defrecord|deftype)\s+(.*)$', s)
        if not m: continue
        kind=m.group(1); rest=m.group(2).split()
        name=''
        for tok in rest:
            if tok.startswith('^') or tok.startswith('{') or tok in [':private', ':static', ':dynamic']:
                continue
            name=tok.strip('[]()')
            break
        if name:
            rows.append({'source':source_id,'ns':ns,'line':i,'kind':kind,'name':name})
    return rows[:core_limit]
core_defs=extract_defs('clojure/src/clj/clojure/core.clj','clojure','clojure.core')
cljs_core_defs=extract_defs('clojurescript/src/main/cljs/cljs/core.cljs','clojurescript','cljs.core')

# CLJS analyzer specials and parse dispatch
cljs_an=text('clojurescript/src/main/clojure/cljs/analyzer.cljc')
cljs_specials=[]
sm=re.search(r"\(def specials '#\{([^}]+)\}\)", cljs_an, re.S)
if sm:
    for sym in sm.group(1).split(): cljs_specials.append({'symbol':sym})
cljs_parse=[]
for i,line in enumerate(cljs_an.splitlines(),1):
    m=re.match(r"\(defmethod\s+parse\s+'([^\s\)]+)", line.strip())
    if m: cljs_parse.append({'line':i,'symbol':m.group(1)})

payload={
 'schema':'code.clojure.forms.v1',
 'source':{'project':'Clojure/ClojureScript language refs','retrieved_at':manifest.get('retrieved_at','2026-06-19'),'license_policy':manifest.get('license_policy',''), 'scope':'fact rows only; no source/prose/docstrings; no graph/math wiring'},
 'attribution':'Clojure and ClojureScript source repositories, EPL-1.0; only factual symbol/category rows are stored.',
 'source_files':source_files(),
 'clojure':{'source':next(x for x in manifest['included'] if x['source_id']=='clojure'), 'compiler_specials':compiler_specials, 'repl_specials':repl_specials, 'reader_macros':reader_macros, 'reader_symbolic_values':reader_symbolic, 'core_defs':core_defs},
 'clojurescript':{'source':next(x for x in manifest['included'] if x['source_id']=='clojurescript'), 'analyzer_specials':cljs_specials, 'parse_methods':cljs_parse, 'core_defs':cljs_core_defs},
 'counts':{'source_files':len(source_files()), 'compiler_specials':len(compiler_specials), 'repl_specials':len(repl_specials), 'reader_macros':len(reader_macros), 'reader_symbolic_values':len(reader_symbolic), 'clojure_core_defs':len(core_defs), 'cljs_analyzer_specials':len(cljs_specials), 'cljs_parse_methods':len(cljs_parse), 'cljs_core_defs':len(cljs_core_defs)},
 'limits':{'core_defs_per_runtime':core_limit}
}
out.write_text('# GENERATED by scripts/gen-code-clojure-refs.sh. Do not edit.\n'+pnix(payload)+'\n', encoding='utf-8')
print('generated '+str(out)+': '+', '.join(f'{k}={v}' for k,v in payload['counts'].items()))
PY
