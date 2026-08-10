#!/usr/bin/env bash
# CPython Parser/Python.asdl -> pnix attrset source.
# Host script = structural transcription only. No graph/mirror wiring.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/code/python/Python.asdl"
MANIFEST="$ROOT/ingest/code/python/manifest.json"
OUT="$ROOT/stdlib/lib/corpus/python-asdl.generated.px"
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --manifest) MANIFEST="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$MANIFEST" "$OUT" <<'PY'
import json, re, sys
src, manifest_path, out = sys.argv[1:]
manifest = json.load(open(manifest_path, encoding='utf-8'))
text = open(src, encoding='utf-8').read()
body = re.search(r'module\s+Python\s*\{(.*)\}\s*$', text, re.S)
if not body:
    raise SystemExit('module Python body not found')
s = body.group(1)
s = re.sub(r'--.*', '', s)
# Join continuation lines by tracking top-level definitions: name = ... until next "name =" at col-ish.
lines = [ln.rstrip() for ln in s.splitlines() if ln.strip()]
defs = []
cur = None
for ln in lines:
    if re.match(r'^\s*[A-Za-z_][A-Za-z0-9_]*\s*=', ln):
        if cur:
            defs.append(cur)
        cur = ln.strip()
    elif cur is not None:
        cur += ' ' + ln.strip()
if cur:
    defs.append(cur)

def split_top(src, sep):
    out, cur, depth = [], '', 0
    for ch in src:
        if ch == '(':
            depth += 1
        elif ch == ')':
            depth -= 1
        if ch == sep and depth == 0:
            out.append(cur.strip()); cur=''
        else:
            cur += ch
    if cur.strip(): out.append(cur.strip())
    return out

def parse_fields(arg):
    arg = arg.strip()
    if not arg:
        return []
    fields=[]
    for part in split_top(arg, ','):
        bits=part.strip().split()
        if not bits: continue
        typ=bits[0]
        name=bits[1] if len(bits)>1 else ''
        seq=typ.endswith('*')
        opt=typ.endswith('?')
        typ=typ.rstrip('*?')
        fields.append({'type':typ,'name':name,'seq':seq,'optional':opt})
    return fields

rows=[]
for d in defs:
    name, rhs = d.split('=',1)
    name=name.strip(); rhs=rhs.strip()
    attrs=[]
    if ' attributes ' in rhs:
        rhs, attr = rhs.split(' attributes ',1)
        attr=attr.strip()
        if attr.startswith('(') and attr.endswith(')'):
            attr=attr[1:-1]
        attrs=parse_fields(attr)
    alts=split_top(rhs, '|')
    if len(alts)==1 and '(' in alts[0] and not re.match(r'^[A-Za-z_][A-Za-z0-9_]*\s*\(', alts[0].strip()):
        kind='product'
    else:
        kind='sum'
    ctors=[]
    product_fields=[]
    if kind=='product':
        x=alts[0].strip()
        if x.startswith('(') and x.endswith(')'):
            x=x[1:-1]
        product_fields=parse_fields(x)
    else:
        for alt in alts:
            m=re.match(r'^([A-Za-z_][A-Za-z0-9_]*)(?:\((.*)\))?$', alt.strip())
            if not m: continue
            ctors.append({'name':m.group(1),'fields':parse_fields(m.group(2) or '')})
    rows.append({'name':name,'kind':kind,'fields':product_fields,'constructors':ctors,'attributes':attrs})

def esc(x): return x.replace('\\','\\\\').replace('"','\\"')
def emit_field(f):
    return '{ type = "%s"; name = "%s"; seq = %s; optional = %s; }' % (esc(f['type']), esc(f['name']), 'true' if f['seq'] else 'false', 'true' if f['optional'] else 'false')
def emit_fields(fs):
    return '[ ' + ' '.join(emit_field(f) for f in fs) + ' ]'
def emit_ctor(c):
    return '{ name = "%s"; fields = %s; }' % (esc(c['name']), emit_fields(c['fields']))
lines=[]
lines.append('# stdlib/lib/corpus/python-asdl.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-code-python-asdl.sh')
lines.append('{')
lines.append('  schema = "code.python.asdl.v1";')
lines.append('  source = {')
for k in ['project','tag','source_path','license_path','source_sha256','license_sha256']:
    lines.append('    %s = "%s";' % (k, esc(str(manifest.get(k,'')))))
lines.append('    license = "Python-2.0-compatible / PSF License";')
lines.append('  };')
lines.append('  definitions = [')
for r in rows:
    lines.append('    { name = "%s"; kind = "%s"; fields = %s; constructors = [ %s ]; attributes = %s; }' % (
        esc(r['name']), esc(r['kind']), emit_fields(r['fields']), ' '.join(emit_ctor(c) for c in r['constructors']), emit_fields(r['attributes'])))
lines.append('  ];')
lines.append('  definition_count = %d;' % len(rows))
lines.append('  generated_note = "CPython ASDL structural transcription only; no graph/mirror/math wiring";')
lines.append('}')
open(out,'w',encoding='utf-8').write('\n'.join(lines)+'\n')
print(f'generated {out}: definitions={len(rows)} tag={manifest.get("tag")}')
PY
