# 28-project-health-gates — 프로젝트 자기정합 게이트 3종

## 쉽게 말하면 (비유)
지금까지의 게이트들은 "pnix-rs가 무엇을 할 수 있는가"를 보였다. 이
셋은 방향이 다르다 — "pnix-rs 프로젝트 스스로가 **자기 설명과 실제로
어긋나지 않는가**"를 본다: 통합 설명이 개별 게이트와 일치하는가(explain),
생성 문서가 최신인가(capabilities), 등록부가 거짓말을 안 하는가
(registry).

## 무엇을
**explain-check**: 통합 explain 리포트(`value`/`ir_sha256`/`pure`/
`mirror_status`)가 각각을 직접 계산하는 개별 게이트(`px_run`/`ir_of`/
`gate_check`/`mirror_run`)와 **정확히 일치**(집계와 구성요소 사이에
drift 없음) + explain 자체의 결정성.

**capabilities-check**: `docs/CAPABILITIES.md`(생성 문서)가 라이브
인덱스(실제 소스에서 뽑은 최신 상태)와 일치 — docs drift 게이트.

**registry-check**: capability 레지스트리가 거짓말을 못 하게: (1) `check`
가 재생하는 모든 게이트가 registry에 placeholder 아닌 `gate_proves`
설명을 갖고 있는지, (2) 로드맵에 적힌 모든 항목의 제안(proposal) 파일이
실제로 디스크에 있는지 — 그래서 registry가 손으로 관리되는 위키가 아니라
**게이트로 검증되는** 문서가 된다.

## plain Rust의 한계 (`limit_rust.rs`)
"코드 주석/README가 실제 구현과 어긋나지 않는다"는 대개 사람의 리뷰에
맡겨진다 — 생성 문서 drift, 통합 리포트와 개별 체크의 불일치, 로드맵의
빈 약속을 자동으로 게이트하는 표준 관행이 없다.

## pnix-rs의 방식 (`pnix_rs_way.sh`)
- `pnix-rs explain-check` — 통합 explain == 개별 게이트 + 결정성
- `pnix-rs capabilities-check` — 생성 문서 == 라이브 인덱스(drift 없음)
- `pnix-rs registry-check` — 모든 게이트에 실질 설명 + 모든 로드맵
  항목에 실재 제안 파일

## 어디에 쓰나
문서/설명/로드맵이 코드에서 갈라져 나가는 것(documentation drift)을
사람의 주의력이 아니라 게이트로 막는다 — 이번 세션에서 다른 4개 host의
게이트를 실제로 돌려서 찾아낸 "실행 안 해봐서 몰랐던 drift"들과 같은
문제를 pnix-rs는 이 3개 게이트로 상시 방지한다.

## 실행
```sh
rustc -O examples/28-project-health-gates/limit_rust.rs -o /tmp/limit_28-project-health-gates && /tmp/limit_28-project-health-gates
bash examples/28-project-health-gates/pnix_rs_way.sh
```
