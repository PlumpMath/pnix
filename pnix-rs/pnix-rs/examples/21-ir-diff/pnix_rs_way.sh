#!/usr/bin/env bash
# pnix-rs 방식 — Rust <-> pnix. flake로 설치했거나 PATH에 pnix-rs가 있으면 그대로,
# 아니면 PNIX_RS=/path/to/pnix-rs 로 지정. (devShell에서는 자동으로 PATH에 있음)
set -u
PNIX_RS="${PNIX_RS:-pnix-rs}"
if ! command -v "$PNIX_RS" >/dev/null 2>&1 && [ ! -x "$PNIX_RS" ]; then
  echo "pnix-rs 실행파일을 찾을 수 없습니다. flake devShell(nix develop) 안에서 돌리거나 PNIX_RS를 지정하세요." >&2
  exit 2
fi
run() { echo "\$ pnix-rs $*"; "$PNIX_RS" "$@"; }

echo "# 정본 IR 구조 diff: 재배열=동일 / 의미변경=첫 차이 위치"
run ir-diff-check
