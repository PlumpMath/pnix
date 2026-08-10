#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${TEKTON_PIPELINE_API_SRC:-$ROOT/ingest/devops/tekton-pipeline-api}"
OUT="${TEKTON_PIPELINE_API_OUT:-$ROOT/stdlib/lib/corpus/tekton-pipeline-api.generated.px}"
LIMIT="${TEKTON_SCHEMA_PATH_LIMIT:-1200}"
python3 - "$SRC" "$OUT" "$LIMIT" <<'PY'
import json, os, re, sys
src, out, limit_s = sys.argv[1:]
limit = int(limit_s)
raw = os.path.join(src, 'raw')
receipt_path = os.path.join(src, 'source-receipt.json')
receipt = json.load(open(receipt_path, encoding='utf-8'))
files = ['300-pipeline.yaml', '300-task.yaml', '300-pipelinerun.yaml', '300-taskrun.yaml']
structural_keys = {
    'apiVersion','kind','metadata','labels','annotations','spec','versions','schema','openAPIV3Schema',
    'type','description','properties','items','required','enum','default','format','nullable','additionalProperties',
    'x-kubernetes-list-type','x-kubernetes-list-map-keys','x-kubernetes-map-type','x-kubernetes-preserve-unknown-fields',
    'x-kubernetes-int-or-string','oneOf','anyOf','allOf','not','pattern','minimum','maximum','minItems','maxItems',
    'minLength','maxLength','patternProperties','uniqueItems','title','examples','externalDocs'
}

def q(s):
    return json.dumps(s, ensure_ascii=False)

def atom(v):
    if isinstance(v, bool): return 'true' if v else 'false'
    if isinstance(v, int): return str(v)
    if isinstance(v, str): return q(v)
    if isinstance(v, list): return '[ ' + ' '.join(atom(x) for x in v) + ' ]'
    if isinstance(v, dict):
        parts = []
        for k in sorted(v):
            parts.append(f'{q(str(k))} = {atom(v[k])};')
        return '{ ' + ' '.join(parts) + ' }'
    if v is None: return 'null'
    raise TypeError(type(v))

def first_re(text, pat, default=''):
    m = re.search(pat, text, re.M)
    return m.group(1).strip().strip('"') if m else default

def names_block(text):
    names = {}
    cats = []
    in_names = False
    in_categories = False
    for line in text.splitlines():
        if re.match(r'^  names:\s*$', line):
            in_names = True; in_categories = False; continue
        if in_names and re.match(r'^  [A-Za-z0-9_-]+:', line):
            break
        if not in_names: continue
        m = re.match(r'^    (plural|singular|kind|listKind):\s*(.+?)\s*$', line)
        if m:
            names[m.group(1)] = m.group(2).strip('"')
            in_categories = False
            continue
        if re.match(r'^    categories:\s*$', line):
            in_categories = True
            continue
        if in_categories:
            cm = re.match(r'^      -\s*(.+?)\s*$', line)
            if cm:
                cats.append(cm.group(1).strip('"'))
    names['categories'] = cats
    return names

def version_rows(text):
    rows = []
    in_versions = False
    cur = None
    for line in text.splitlines():
        if re.match(r'^  versions:\s*$', line):
            in_versions = True; continue
        if in_versions and re.match(r'^  [A-Za-z0-9_-]+:', line):
            break
        if not in_versions: continue
        m = re.match(r'^    - name:\s*(.+?)\s*$', line)
        if m:
            if cur: rows.append(cur)
            cur = {'name': m.group(1).strip('"'), 'served': False, 'storage': False}
            continue
        if cur:
            sm = re.match(r'^      (served|storage):\s*(true|false)\s*$', line)
            if sm: cur[sm.group(1)] = sm.group(2) == 'true'
    if cur: rows.append(cur)
    return rows

def schema_paths(text, file_name, crd_kind, remaining):
    rows = []
    enabled = False
    stack = []
    seen = set()
    for line in text.splitlines():
        if 'openAPIV3Schema:' in line:
            enabled = True
            stack = []
            continue
        if not enabled or len(rows) >= remaining:
            continue
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        m = re.match(r'^(\s*)([A-Za-z_][A-Za-z0-9_-]*):(?:\s*(.*))?$', line)
        if not m:
            continue
        indent, key, rest = len(m.group(1)), m.group(2), (m.group(3) or '').strip()
        while stack and indent <= stack[-1][0]:
            stack.pop()
        if key in structural_keys:
            continue
        path = '.'.join([x[1] for x in stack] + [key])
        if path in seen:
            continue
        seen.add(path)
        rows.append({'file': file_name, 'crd_kind': crd_kind, 'path': path, 'indent': indent})
        stack.append((indent, key))
    return rows

crds = []
paths = []
for name in files:
    text = open(os.path.join(raw, name), encoding='utf-8').read()
    ns = names_block(text)
    versions = version_rows(text)
    crd = {
        'file': name,
        'metadata_name': first_re(text, r'^  name:\s*(\S+)'),
        'api_group': first_re(text, r'^  group:\s*(\S+)'),
        'scope': first_re(text, r'^  scope:\s*(\S+)'),
        'kind': ns.get('kind',''),
        'plural': ns.get('plural',''),
        'singular': ns.get('singular',''),
        'listKind': ns.get('listKind',''),
        'categories': ns.get('categories',[]),
        'versions': versions,
        'served_versions': [v['name'] for v in versions if v.get('served')],
        'storage_versions': [v['name'] for v in versions if v.get('storage')],
    }
    crds.append(crd)
    paths.extend(schema_paths(text, name, crd['kind'], max(0, limit - len(paths))))
seed = {
    'schema': 'devops.tekton.pipeline_api.v1',
    'source': {
        'name': 'Tekton Pipeline API CRD structural metadata',
        'license': 'Apache-2.0',
        'source_ref': receipt.get('source_ref',''),
        'source_id': 'tekton-pipeline-api',
    },
    'summary': {
        'crd_count': len(crds),
        'version_count': sum(len(c['versions']) for c in crds),
        'schema_path_count': len(paths),
        'schema_path_limit': limit,
        'actual_pipeline_instances_ingested': False,
        'execution_or_deploy_behavior_ingested': False,
        'mirror_graph_wiring': False,
    },
    'source_receipt': receipt,
    'crds': crds,
    'schema_paths': paths,
    'excluded': ['actual CR instances', 'secrets', 'logs', 'artifacts', 'execution/deployment behavior', 'prose/examples', 'mirror/graph/math wiring'],
}
os.makedirs(os.path.dirname(out), exist_ok=True)
with open(out, 'w', encoding='utf-8') as f:
    f.write('# GENERATED by scripts/gen-devops-tekton-pipeline-api.sh; do not commit.\n')
    f.write('# Tekton Pipeline API CRD structural metadata only; no CR instances, secrets, logs, artifacts, execution, or graph wiring.\n')
    f.write(atom(seed))
    f.write('\n')
print(f'generated {out}: crds={len(crds)} versions={seed["summary"]["version_count"]} schema_paths={len(paths)} bytes={os.path.getsize(out)}')
PY
