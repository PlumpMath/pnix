# 14 — 러너 & REPL (역할 분류)

두 언어, 언어마다 두 역할. flake 러너가 각자 할 일을 보여 준다.

| 러너 | 역할 |
|------|------|
| `rs-meta -- run` | RUST 인터프리터 (meta-circular 신뢰 바닥) |
| `rs-meta -- native-run` | RUST 컴파일러 (rustc 네이티브 티어; TV 로 interp 과 동일) |
| `repl-pnix-rs-rust` | 대화형 RUST REPL (rs-meta 인터프리터 구동) |
| `pnix-rs-pnix -- -f default.px` | PNIX (px) 컴파일러/평가기 (`.px` 파일) |
| `repl-pnix-rs-pnix` | 대화형 PNIX (px) REPL |

rs-meta 는 cargo/rustc 드롭인 대체물이 아니라, rustc 툴체인을 *쓰는*
subset **meta-circular peer 엔진**이다. 순수 바닥(대화형 io 없음)을 유지하고,
**pnix-rs** 가 양쪽 REPL 을 몰며 bootstrap CLI 로 rs-meta 를 peer 로 호출한다.

실행: `bash pnix_rs_way.sh` (devShell, 또는 `PNIX_RS` / `RS_META_BOOTSTRAP` 설정).
