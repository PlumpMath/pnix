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

echo "# 정본 IR + 내용주소 sha256"
run ir -c 'let a = 5; b = a + 1; in a * b' | grep ir_sha256
echo
echo "# 바인딩 순서만 다른 두 식이 같은 IR 해시를 공유 (identity sharing)"
h1=$("$PNIX_RS" ir -c 'let a = 1; b = 2; in a + b' | awk '/ir_sha256/{print $2}')
h2=$("$PNIX_RS" ir -c 'let b = 2; a = 1; in a + b' | awk '/ir_sha256/{print $2}')
echo "h1=$h1"; echo "h2=$h2"; [ "$h1" = "$h2" ] && echo "=> 같은 주소 (identity sharing OK)" || echo "=> 다름"
echo
echo "# 같은 보장을 코퍼스 전체로: sha256 self-test + 모든 프로그램이 직접 평가 가능 + identity sharing"
run ir-check
