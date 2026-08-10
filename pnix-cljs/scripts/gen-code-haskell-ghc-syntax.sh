#!/usr/bin/env bash
set -euo pipefail

# GHC syntax/parser source snapshots → pnix attrset source 생성.
# raw source/prose body 저장 금지: 선언명/constructor명/parser/token/extension flag 이름만 추출.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IN="$ROOT/ingest/code/haskell/ghc"
OUT="$ROOT/stdlib/lib/corpus/haskell-ghc-syntax.generated.px"
CONSTRUCTOR_LIMIT="${PNIX_GHC_CONSTRUCTOR_LIMIT:-420}"
NONTERMINAL_LIMIT="${PNIX_GHC_NONTERMINAL_LIMIT:-360}"
TOKEN_LIMIT="${PNIX_GHC_TOKEN_LIMIT:-220}"

python3 - "$IN" "$OUT" "$CONSTRUCTOR_LIMIT" "$NONTERMINAL_LIMIT" "$TOKEN_LIMIT" <<'PY'
import hashlib,json,pathlib,re,sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
constructor_limit=int(sys.argv[3]); nonterminal_limit=int(sys.argv[4]); token_limit=int(sys.argv[5])
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

def source_files():
    rows=[]
    for f in manifest.get('files',[]):
        p=src/f['path']; b=p.read_bytes()
        rows.append({'path':f['path'],'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'lines':len(b.decode('utf-8','replace').splitlines())})
    return rows

syntax_files=[
 'compiler/Language/Haskell/Syntax/Expr.hs','compiler/Language/Haskell/Syntax/Pat.hs','compiler/Language/Haskell/Syntax/Type.hs','compiler/Language/Haskell/Syntax/Decls.hs','compiler/Language/Haskell/Syntax/Binds.hs','compiler/Language/Haskell/Syntax/Basic.hs'
]
declarations=[]; constructors=[]
for rel in syntax_files:
    current=''
    for i,line in enumerate(read(rel),1):
        s=line.strip()
        m=re.match(r'^(data|newtype|type)\s+([A-Z][A-Za-z0-9_\']*)\b', s)
        if m:
            current=m.group(2) if m.group(1) in {'data','newtype'} else ''
            declarations.append({'file':rel,'line':i,'kind':m.group(1),'name':m.group(2)})
        cm=re.match(r'^(?:=|\|)\s*([A-Z][A-Za-z0-9_\']*)\b', s)
        if cm and current and len(constructors)<constructor_limit:
            constructors.append({'file':rel,'line':i,'type':current,'name':cm.group(1)})

parser=read('compiler/GHC/Parser.y')
parser_entries=[]; parser_nonterminals=[]; parser_tokens=[]; in_token=False
for i,line in enumerate(parser,1):
    s=line.strip()
    m=re.match(r'%name\s+([A-Za-z0-9_\']+)\s+([A-Za-z0-9_\']+)', s)
    if m: parser_entries.append({'line':i,'entry':m.group(1),'nonterminal':m.group(2)})
    m=re.match(r'^([A-Za-z_][A-Za-z0-9_\']*)\s*::\s*\{\s*(.*?)\s*\}\s*$', line)
    if m and len(parser_nonterminals)<nonterminal_limit:
        parser_nonterminals.append({'line':i,'name':m.group(1),'result_type':m.group(2)[:160]})
    if s == '%token': in_token=True; continue
    if in_token:
        if s.startswith('%') and s != '%token': in_token=False
        else:
            tm=re.match(r"^('.*?'|[A-Z][A-Z0-9_]*|[a-z][A-Za-z0-9_]*)\s+\{.*?\b(IT[A-Za-z0-9_']+)", s)
            if tm and len(parser_tokens)<token_limit:
                parser_tokens.append({'line':i,'label':tm.group(1).strip("'"),'token_constructor':tm.group(2)})

lexer_macros=[]
for i,line in enumerate(read('compiler/GHC/Parser/Lexer.x'),1):
    m=re.match(r'^([@$])([A-Za-z_][A-Za-z0-9_\']*)\s*=', line.strip())
    if m:
        lexer_macros.append({'line':i,'kind':'char_class' if m.group(1)=='$' else 'regex_macro','name':m.group(2)})

extensions=[]; in_ext=False
for i,line in enumerate(read('libraries/ghc-internal/src/GHC/Internal/LanguageExtensions.hs'),1):
    s=line.strip()
    if s == 'data Extension': in_ext=True; continue
    if in_ext:
        m=re.match(r'^(?:=|\|)\s*([A-Z][A-Za-z0-9_]*)\b', s)
        if m: extensions.append({'line':i,'name':m.group(1)})
        if s.startswith('deriving') or s.startswith('instance '):
            break

payload={
 'schema':'code.haskell.ghc.syntax.v1',
 'source':{'project':'GHC exposed Haskell syntax/parser structures','repo':manifest.get('repo','https://github.com/ghc/ghc'),'commit_sha':manifest.get('commit_sha',''),'retrieved_at':manifest.get('retrieved_at','2026-06-19'),'license_id':'GHC BSD-3-Clause-style','scope':'fact rows only; no source/prose bodies; no graph/math wiring'},
 'attribution':'Glasgow Haskell Compiler source repository; GHC BSD-style license; factual syntax/parser rows only.',
 'source_files':source_files(),
 'ast_declarations':declarations,
 'ast_constructors':constructors,
 'parser_entries':parser_entries,
 'parser_nonterminals':parser_nonterminals,
 'parser_tokens':parser_tokens,
 'lexer_macros':lexer_macros,
 'language_extensions':extensions,
 'counts':{'source_files':len(source_files()),'ast_declarations':len(declarations),'ast_constructors':len(constructors),'parser_entries':len(parser_entries),'parser_nonterminals':len(parser_nonterminals),'parser_tokens':len(parser_tokens),'lexer_macros':len(lexer_macros),'language_extensions':len(extensions)},
 'limits':{'ast_constructors':constructor_limit,'parser_nonterminals':nonterminal_limit,'parser_tokens':token_limit}
}
out.write_text('# GENERATED by scripts/gen-code-haskell-ghc-syntax.sh. Do not edit.\n'+pnix(payload)+'\n',encoding='utf-8')
print('generated '+str(out)+': '+', '.join(f'{k}={v}' for k,v in payload['counts'].items()))
PY
