# 0009 — canonical Rust IR + hash (feeds the peer-engine verdict)

상태: **v0 DONE (2026-07-03)** — rs-meta rust-ir-check 4/4; pnix-rs engine
verdict의 ir_hash 채움(engine-verdict-check 5/5).

## 동기 (사용자 분석 section 5, 2순위)
"지금 AST dump는 있으나 pnix-hy의 IR처럼 source → canonical Rust IR → stable
hash → eval이 필요." source_hash는 포맷에 민감. adapter의 verdict는 포맷 불변
content address(ir_hash)가 있어야 TV 실패 위치 추적/캐시 키로 쓸 수 있다.

## 구현
- **rs-meta (pnix-free)**: `rust-ir` 커맨드 + `check::rust_ir_of` — mirror-proven
  ast-canonical(sig_program) + FNV ir_hash + evaluable(emit 재파스). rust-ir-
  check(4/4): 결정성 · 포맷 불변(공백/주석→같은 hash) · faithful(다른 프로그램
  →다른 hash) · evaluable.
- **pnix-rs adapter**: engine.rs가 `rust-ir`를 호출해 verdict.ir_hash 채움.
  engine-verdict-check(4→5): ir_hash가 포맷 불변이고 source_hash와 구별됨을 증명.

## 효과
- verdict.ir_hash = 포맷 불변 canonical Rust IR content address.
- TV 실패 위치 추적: 두 프로그램의 ir 차이는 rs-meta ast-diff로 국소화 가능.
- 공통 verdict가 source_hash(raw) + ir_hash(canonical) 둘 다 노출.

## 남은 것 (v1+)
- eval-parity를 verdict에 노출(현재는 rust-ir의 evaluable flag까지; interp==rustc
  parity는 verdict.tv_equal이 담당).
- witness_id 연결(0008 잔여).
