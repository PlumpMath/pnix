# pnix-rs 파운데이션 경로

확장 검증/연구 예제 전에 이 경로를 먼저 본다.

| 단계 | 관심사 | 예제 | 소유 |
|------|--------|------|------|
| 1 | 기본 PNIX 평가 | `00-foundation/basic.sh` | `pnix-rs` 실행 |
| 2 | Rust/PNIX 사영 | `00-foundation/interop.sh` | `pnix-rs` + rs-meta 메커니즘 |
| 3 | meta-circular 실행 | `00-foundation/meta_circular.sh` | rs-meta 기판 + PNIX 타워 |

현재 공개 제품은 바이너리이므로, 이 예제들은 아직 없는 in-process Rust
컴포넌트 ABI 를 가장하지 않고 **CLI** 를 쓴다. 정본 계약은
`component_invocation_runtime_defined = false` 를 명시한다.

CLI 텍스트는 관측용일 뿐 타입 권위가 아니다. HABI 링크는 전체
`pnix.boundary-type.v1` 구조 노드와 digest 를 나르며, `"I64"` / `"ProbeInput"`
같은 Rust 문자열을 타입 대용으로 쓰지 않는다.

## 확장 카탈로그

| 역할 | 기존 예제 |
|------|-----------|
| 기본 평가·러너 | `01`, `14` |
| Rust/PNIX 사영·임베드 | `03`, `04`, `15` |
| meta-circular 메커니즘 | `06`, `10`, `11`, `12` |
| 상태/격리 | `07`, `08` |
| 독립 proof/연구 | `02`, `05`, `09`, `13` |

rs-meta 는 기본 호스트 능력이다. attestation, mirror 영수증, 서비스 verdict 는
독립 검증 표면이다.

호스트 간 균형: 모노레포 [`examples/EXAMPLES_BALANCE.md`](../../../examples/EXAMPLES_BALANCE.md).
