#!/usr/bin/env bash
set -euo pipefail

# Tree-sitter official grammar repos 다운로드.
# GitHub org metadata에서 license가 MIT/Apache-2.0인 grammar repo만 포함한다.
# host=네트워크/파일 IO와 version pinning만 수행; graph/math 연결 없음.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/tree-sitter"
mkdir -p "$OUT"

python3 - "$OUT" <<'PY'
import hashlib, json, pathlib, subprocess, sys, urllib.error, urllib.request
out = pathlib.Path(sys.argv[1])
org_url = 'https://api.github.com/orgs/tree-sitter/repos?per_page=100&type=public'
allow_spdx = {'MIT', 'Apache-2.0', 'BSD-2-Clause', 'BSD-3-Clause'}
node_candidates = [
    'src/node-types.json',
    'typescript/src/node-types.json',
    'tsx/src/node-types.json',
    'php/src/node-types.json',
    'php_only/src/node-types.json',
]
license_candidates = ['LICENSE', 'LICENSE.md', 'LICENSE.txt', 'LICENSE-MIT', 'COPYING']

def fetch_json(url):
    with urllib.request.urlopen(url, timeout=30) as r:
        return json.load(r)

def fetch_bytes(url):
    with urllib.request.urlopen(url, timeout=30) as r:
        return r.read()

def try_fetch(url):
    try:
        return fetch_bytes(url)
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return None
        raise
    except urllib.error.URLError:
        return None

def ls_remote(repo, branch):
    cp = subprocess.run(['git', 'ls-remote', repo, f'refs/heads/{branch}'], text=True, capture_output=True, check=True)
    return cp.stdout.split()[0]

repos = fetch_json(org_url)
included = []
skipped = []
for r in sorted(repos, key=lambda x: x['name']):
    name = r['name']
    if not name.startswith('tree-sitter-') or name == 'tree-sitter-cli':
        continue
    lic = (r.get('license') or {}).get('spdx_id') or 'NOASSERTION'
    if lic not in allow_spdx:
        skipped.append({'repo': name, 'reason': 'license', 'license_id': lic})
        continue
    branch = r.get('default_branch') or 'master'
    repo_url = f'https://github.com/tree-sitter/{name}.git'
    try:
        commit = ls_remote(repo_url, branch)
    except Exception as e:
        skipped.append({'repo': name, 'reason': 'ls-remote', 'error': str(e)[:160]})
        continue
    raw_base = f'https://raw.githubusercontent.com/tree-sitter/{name}/{commit}'
    repo_dir = out / name
    repo_dir.mkdir(parents=True, exist_ok=True)
    node_paths = []
    for rel in node_candidates:
        b = try_fetch(f'{raw_base}/{rel}')
        if b is None:
            continue
        target = repo_dir / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(b)
        node_paths.append({'path': rel, 'sha256': hashlib.sha256(b).hexdigest(), 'bytes': len(b)})
    if not node_paths:
        skipped.append({'repo': name, 'reason': 'no-node-types', 'license_id': lic})
        continue
    license_file = None
    for rel in license_candidates:
        b = try_fetch(f'{raw_base}/{rel}')
        if b is not None:
            (repo_dir / rel).write_bytes(b)
            license_file = {'path': rel, 'sha256': hashlib.sha256(b).hexdigest(), 'bytes': len(b)}
            break
    lang = name[len('tree-sitter-'):].replace('-', '_')
    included.append({
        'repo': name,
        'language': lang,
        'record_schema': f'code.tree_sitter.grammar.{lang}.v1',
        'repo_url': f'https://github.com/tree-sitter/{name}',
        'default_branch': branch,
        'commit_sha': commit,
        'license_id': lic,
        'license_file': license_file,
        'node_type_files': node_paths,
    })
manifest = {
    'schema': 'pnix.ingest.source_manifest.v1',
    'source_id': 'tree-sitter-grammars',
    'source_name': 'Tree-sitter official grammars node-types',
    'retrieved_at': '2026-06-19',
    'license_policy': 'Only permissive GitHub-detected MIT/Apache/BSD grammar repos with node-types.json are included; unknown/non-permissive repos are skipped.',
    'included_count': len(included),
    'skipped_count': len(skipped),
    'included': included,
    'skipped': skipped,
}
(out / 'source-manifest.json').write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
print(f"included={len(included)} skipped={len(skipped)} -> {out}")
for g in included:
    print(f"  {g['repo']} {g['license_id']} {g['commit_sha'][:12]} files={len(g['node_type_files'])}")
if skipped:
    print('skipped:')
    for s in skipped:
        print(' ', s)
PY
