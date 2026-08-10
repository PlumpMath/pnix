#!/usr/bin/env bash
# pnix-rs 방식 — 호스트 Rust에 임베드된 pnix에 "의미"를 준다(08 · reader-embed 상당).
# Rust 고유 메타프로그래밍(macro_rules!)은 rs-meta에서 held이고, 설령 있어도 pnix에
# 의미를 못 준다. 그래서 pnix-rs는 rs-meta의 "Rust 코드생성"(rust-mirror)을 매크로/
# reader로 삼아: 임베드된 pnix를 (1)평가하고 (2)Rust로 사영해 substrate가 3-way로
# 의미보존을 판정한다. pnix는 비동형 그대로 두고, 두 언어를 언어 수준에서 잇는다.
set -u
PNIX_RS="${PNIX_RS:-pnix-rs}"
if ! command -v "$PNIX_RS" >/dev/null 2>&1 && [ ! -x "$PNIX_RS" ]; then
  echo "pnix-rs 실행파일을 찾을 수 없습니다. nix develop 안에서 돌리거나 PNIX_RS를 지정하세요." >&2
  exit 2
fi
run() { echo "\$ pnix-rs $*"; "$PNIX_RS" "$@"; }

# limit_rust.rs가 소스에 임베드한 것과 '같은' pnix 스니펫.
PX='let base = 6; in base * 7'

echo "# 1) read-time 승격 + 평가 — 임베드된 pnix에 의미를 준다 (plain Rust는 죽은 문자열)"
run px-eval -c "$PX"

echo
echo "# 2) 호스트 언어(Rust)로 사영 — rs-meta 코드생성이 '매크로 확장' 역할"
echo "#    (px 값 -> Rust print-program -> substrate에서 interp==rustc==native)"
run rust-mirror -c "$PX" | grep -E "px_value|program_sha256|status|loss_status"

echo
echo "# 3) 폴리글롯 witness — 이 언어-간 임베드가 의미를 보존하는가에 대한 .px 증거"
run rust-mirror -c "$PX" | grep -E "direction|source_lang|target_lang|out_hash" | head -4

echo
echo "# 4) 오타 pnix는 read-time에 정직하게 거부된다 (plain Rust는 &str이라 통과시킴)"
echo "\$ pnix-rs px-eval -c 'let base = 6; in base * '"
"$PNIX_RS" px-eval -c 'let base = 6; in base * ' 2>&1 | head -1 || true
