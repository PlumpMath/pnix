#!/usr/bin/env bash
# 모든 예제의 pnix-rs 방식을 순서대로 실행. devShell 안이거나 PNIX_RS/RS_META_BOOTSTRAP 지정 시 동작.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
PNIX_RS="${PNIX_RS:-pnix-rs}"
export PNIX_RS
for d in "$here"/[0-9]*/; do
  name="$(basename "$d")"
  echo "======================================================================"
  echo "== $name"
  echo "======================================================================"
  bash "$d/pnix_rs_way.sh" || echo "  (섹션 $name: 위 판정 참고)"
  echo
done
