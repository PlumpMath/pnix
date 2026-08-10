#!/usr/bin/env bash
# TypeScript compiler SyntaxKind enum -> pnix attrset source.
# Source: microsoft/TypeScript src/compiler/types.ts (Apache-2.0).
# Host script = IO/structural transcription only. No graph/mirror wiring.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC="$ROOT/ingest/code/typescript/types.ts"
OUT="$ROOT/stdlib/lib/corpus/typescript-syntaxkind.generated.px"
TAG="${TYPESCRIPT_TAG:-v6.0.3}"
while [ $# -gt 0 ]; do
  case "$1" in
    --src) SRC="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    --tag) TAG="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
python3 - "$SRC" "$OUT" "$TAG" <<'PY'
import re, sys, hashlib, datetime
src, out, tag = sys.argv[1], sys.argv[2], sys.argv[3]
raw = open(src, 'rb').read()
sha = hashlib.sha256(raw).hexdigest()
text = raw.decode('utf-8')
match = re.search(r'export\s+(?:const\s+)?enum\s+SyntaxKind\b', text)
if not match:
    raise SystemExit('SyntaxKind enum not found')
start = match.start()
brace = text.find('{', start)
if brace < 0:
    raise SystemExit('SyntaxKind enum brace not found')
level = 0
end = None
for i in range(brace, len(text)):
    ch = text[i]
    if ch == '{':
        level += 1
    elif ch == '}':
        level -= 1
        if level == 0:
            end = i
            break
if end is None:
    raise SystemExit('SyntaxKind enum end not found')
body = text[brace+1:end]
body = re.sub(r'/\*.*?\*/', '', body, flags=re.S)
body = re.sub(r'//.*', '', body)
parts = [p.strip() for p in body.split(',') if p.strip()]
rows = []
values = {}
next_value = 0
for idx, part in enumerate(parts):
    if '=' in part:
        name, init = part.split('=', 1)
        name = name.strip()
        init = init.strip()
    else:
        name, init = part.strip(), None
    if not re.match(r'^[A-Za-z_][A-Za-z0-9_]*$', name):
        continue
    resolved = None
    if init is None:
        resolved = next_value
    else:
        m = re.match(r'^[-]?(?:0[xX][0-9a-fA-F]+|\d+)$', init)
        if m:
            resolved = int(init, 0)
        elif init in values:
            resolved = values[init]
        else:
            # Preserve unresolved initializer text; continue sequence only if possible.
            resolved = None
    if resolved is not None:
        values[name] = resolved
        next_value = resolved + 1
    rows.append({
        'name': name,
        'ordinal_index': idx,
        'initializer': init,
        'value': resolved,
    })

def esc(s):
    return s.replace('\\', '\\\\').replace('"', '\\"')

lines = []
lines.append('# stdlib/lib/corpus/typescript-syntaxkind.generated.px — GENERATED, do not commit.')
lines.append('# 생성: scripts/gen-code-typescript-syntaxkind.sh')
lines.append('{')
lines.append('  schema = "code.typescript.syntaxkind.v1";')
lines.append('  source = {')
lines.append('    name = "TypeScript SyntaxKind";')
lines.append('    project = "microsoft/TypeScript";')
lines.append(f'    tag = "{esc(tag)}";')
lines.append('    license = "Apache-2.0";')
lines.append('    source_path = "src/compiler/types.ts";')
lines.append(f'    source_sha256 = "{sha}";')
lines.append('    source_url = "https://github.com/microsoft/TypeScript";')
lines.append('  };')
lines.append('  members = [')
for r in rows:
    parts = [f'name = "{esc(r["name"])}";', f'ordinal_index = {r["ordinal_index"]};']
    if r['initializer'] is not None:
        parts.append(f'initializer = "{esc(r["initializer"])}";')
    if r['value'] is not None:
        parts.append(f'value = {r["value"]};')
    lines.append('    { ' + ' '.join(parts) + ' }')
lines.append('  ];')
lines.append('  member_count = %d;' % len(rows))
lines.append('  generated_note = "structural enum transcription only; no source corpus, prose, graph wiring, or semantic normalization";')
lines.append('}')
open(out, 'w', encoding='utf-8').write('\n'.join(lines) + '\n')
print(f'generated {out}: members={len(rows)} sha256={sha}')
PY
