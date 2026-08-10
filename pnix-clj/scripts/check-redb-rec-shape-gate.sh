#!/usr/bin/env bash
# redb-rec-shape-gate: 모든 redb 파일이 pnix rec { ... } 형태인지 확인
# - commit 단계: staged redb (.redb)만 검사
# - push   단계: HEAD에 포함된 redb 전체 검사
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT_DIR"

MODE="staged"
if [ $# -gt 0 ]; then
  case "$1" in
    --staged)
      MODE="staged"
      shift
      ;;
    --head|--push|--committed|--all-committed)
      MODE="head"
      shift
      ;;
    --all)
      MODE="all"
      shift
      ;;
    --paths)
      MODE="paths"
      shift
      ;;
    --help|-h)
      echo "usage: $0 [--staged|--head|--paths <path...>]"
      exit 0
      ;;
    *)
      echo "FAIL: unknown arg '$1' (use --staged or --head)" >&2
      exit 2
      ;;
  esac
fi

collect_files() {
  local mode="$1"
  shift || true
  local files=""
  case "$mode" in
    staged)
      files="$(git diff --cached --name-only --diff-filter=ACMR -- '*.redb' 2>/dev/null || true)"
      ;;
    head)
      files="$(git ls-tree -r --name-only HEAD -- '*.redb' 2>/dev/null | tr '\n' '\n')"
      ;;
    all)
      # tracked + untracked 포함(권장 범위는 좁지만, 커밋/푸시 가드에는 과대 포획이 더 안전)
      files="$(git ls-files -z '*.redb' | tr '\0' '\n'
git ls-files -o -z --exclude-standard '*.redb' | tr '\0' '\n')"
      ;;
    paths)
      files="$*"
      ;;
    *)
      files=""
      ;;
  esac
  printf '%s\n' "$files"
}

is_binary() {
  local file="$1"
  # grep -Iq returns 1 if binary; -q suppresses output.
  if LC_ALL=C grep -Iq . "$file" 2>/dev/null; then
    return 1
  fi
  return 0
}

first_non_empty_non_comment_line() {
  local file="$1"
  awk '
    {
      gsub(/\r$/, "", $0);
      if ($0 ~ /^[[:space:]]*$/) next;
      if ($0 ~ /^[[:space:]]*#/ ) next;
      if ($0 ~ /^[[:space:]]*\/\//) next;
      print;
      exit;
    }
  ' "$file"
}

fail_count=0
checked_count=0

while IFS= read -r redb; do
  [ -z "$redb" ] && continue
  if [ ! -f "$redb" ]; then
    continue
  fi

  checked_count=$((checked_count + 1))
  if is_binary "$redb"; then
    echo "FAIL: redb gate -- binary substrate prohibited: ${redb}"
    echo "      reason: redb must be pnix 텍스트(rec { ... })만 허용"
    fail_count=$((fail_count + 1))
    continue
  fi

  first_line="$(first_non_empty_non_comment_line "$redb")"
  if [ -z "$first_line" ]; then
    echo "FAIL: redb gate -- empty redb source: ${redb}"
    fail_count=$((fail_count + 1))
    continue
  fi

  # Strip UTF-8 BOM if present.
  first_line="$(printf '%s' "$first_line" | sed '1s/^\xEF\xBB\xBF//')"
  if ! printf '%s' "$first_line" | grep -Eq '^[[:space:]]*rec[[:space:]]*\{'; then
    echo "FAIL: redb gate -- top-level pnix rec { } missing: ${redb}"
    echo "      first non-empty line: ${first_line}"
    echo "      expected: rec { ... }"
    fail_count=$((fail_count + 1))
    continue
  fi
done < <(collect_files "$MODE" "$@")

if [ "$checked_count" -eq 0 ]; then
  echo "OK: redb gate -- checked 0 redb files (mode=${MODE})."
  exit 0
fi

if [ "$fail_count" -ne 0 ]; then
  echo "FAIL: redb gate -- ${fail_count}/${checked_count} redb file(s) violate rec { } only rule."
  echo "      mode=${MODE}; 커밋/푸시가 차단됩니다."
  exit 1
fi

echo "OK: redb gate -- ${checked_count} redb file(s) all start with rec { ... }."
exit 0
