#!/usr/bin/env bash
# Keep a Changelog -> pnix attrset source.
# Host script is IO/transcription only. It excludes prose body and project-specific change entries.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${KEEP_A_CHANGELOG_SRC:-$ROOT/ingest/code/keep-a-changelog/src}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/keep-a-changelog.generated.px}"
RECEIPT="$ROOT/ingest/code/keep-a-changelog/source-receipt.json"
if [[ ! -f "$SRC/CHANGELOG.md" ]]; then
  echo "missing CHANGELOG.md under $SRC" >&2
  echo "run scripts/update-code-keep-a-changelog.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import json, sys, re, hashlib, datetime
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
try: receipt=json.load(open(receipt_path))
except Exception: receipt={}
changelog=(src/'CHANGELOG.md').read_text(encoding='utf-8')
readme=(src/'README.md').read_text(encoding='utf-8') if (src/'README.md').exists() else ''
links=json.load(open(src/'data/links.json')) if (src/'data/links.json').exists() else {}
license_text=(src/'LICENSE').read_text(encoding='utf-8') if (src/'LICENSE').exists() else ''
section_order=['Added','Changed','Deprecated','Removed','Fixed','Security']
seen_sections=[]
for m in re.finditer(r'^###\s+(.+?)\s*$', changelog, re.M):
    name=m.group(1).strip()
    if name not in seen_sections:
        seen_sections.append(name)
versions=[]
for m in re.finditer(r'^##\s+\[([^\]]+)\](?:\s+-\s+([0-9]{4}-[0-9]{2}-[0-9]{2}))?\s*$', changelog, re.M):
    versions.append({'label':m.group(1), 'date':m.group(2), 'unreleased':m.group(1).lower()=='unreleased'})
compare_links=[]
for m in re.finditer(r'^\[([^\]]+)\]:\s+(\S+)\s*$', changelog, re.M):
    label,url=m.group(1),m.group(2)
    kind='unreleased_compare' if label.lower()=='unreleased' else ('release_tag' if '/releases/tag/' in url else 'version_compare')
    compare_links.append({'label':label, 'kind':kind, 'url_pattern_sample':url})
obj={
  'schema':'code.keep_a_changelog.v2',
  'source':{
    'name':'Keep a Changelog',
    'repository':'olivierlacan/keep-a-changelog',
    'license':'MIT',
    'source_urls':['https://keepachangelog.com/','https://github.com/olivierlacan/keep-a-changelog'],
    'receipt':receipt,
    'generated_at':receipt.get('retrieved_at'),
    'generator':'scripts/gen-code-keep-a-changelog.sh',
    'scope':'changelog section taxonomy and structural patterns only; prose body/project-specific change entries/release automation excluded'
  },
  'spec':{
    'source_path':'CHANGELOG.md',
    'source_sha256':hashlib.sha256(changelog.encode()).hexdigest(),
    'readme_sha256':hashlib.sha256(readme.encode()).hexdigest() if readme else None,
    'license_sha256':hashlib.sha256(license_text.encode()).hexdigest(),
    'license':'MIT',
  },
  'taxonomy':{
    'canonical_sections':[{'name':x, 'observed_in_source':x in seen_sections} for x in section_order],
    'observed_sections':seen_sections,
    'unreleased_section':{'label':'Unreleased','heading_pattern':'## [Unreleased]'},
  },
  'version_heading':{
    'pattern':'## [VERSION] - YYYY-MM-DD',
    'version_reference':'Semantic Versioning',
    'date_format':'YYYY-MM-DD',
    'observed_release_count':sum(1 for v in versions if not v['unreleased']),
    'observed_headings':versions,
  },
  'link_patterns':{
    'compare_link_reference_pattern':'[VERSION]: repository/compare/vPREVIOUS...vVERSION',
    'unreleased_reference_pattern':'[unreleased]: repository/compare/vLATEST...HEAD',
    'observed_count':len(compare_links),
    'observed_links':compare_links,
  },
  'external_refs':{
    'semver':links.get('semver'),
    'github_releases':links.get('github_releases'),
    'gnustyle':links.get('gnustyle'),
    'gnunews':links.get('gnunews'),
  }
}

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
content='# stdlib/lib/corpus/keep-a-changelog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-keep-a-changelog.sh && scripts/gen-code-keep-a-changelog.sh\n'
content+='# 범위: Keep a Changelog taxonomy/structure metadata only. project entries/prose/release automation 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f"generated {out}: sections={len(seen_sections)} releases={sum(1 for v in versions if not v['unreleased'])} bytes={len(content.encode())}")
PY
