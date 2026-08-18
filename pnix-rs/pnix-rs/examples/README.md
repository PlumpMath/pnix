# examples — 왜 meta-circular인가 (plain Rust의 한계 vs pnix-rs/rs-meta)

> **호스트 간 균형:** 모노레포
> [`examples/EXAMPLES_BALANCE.md`](../../../examples/EXAMPLES_BALANCE.md)
> (rs ≈ 중간 카탈로그; clj/hy 조밀; cljs/clr 코어 00–06).
>
> **파운데이션 진입:** [FOUNDATION_PATH.md](FOUNDATION_PATH.md) 와
> [`00-foundation`](00-foundation/README.md). PNIX 런타임과 rs-meta
> meta-circular 메커니즘은 기본 능력이다. Mirror/proof 영수증과 서비스
> admission 은 독립 검사이며 평가의 전제가 아니다.

The existing numbered examples remain the extended verification/research
catalog. They do not make proof status part of the PNIX language outcome.

이 폴더는 **사람이 직접 코드를 보고 "이 기능을 어디에 쓸지" 판단**하도록 만든 예제 모음입니다.
각 섹션은 한 가지 meta-circular 능력을 다루고 **두 파일을 나란히** 둡니다:

- `limit_rust.rs` — **plain Rust의 한계**: 그 언어를 "그냥" 쓰면 왜 안 되는지 / 무엇이 불가능한지.
  (실제로 `rustc`로 컴파일·실행되어 한계를 정직하게 보여줍니다.)
- `pnix_rs_way.sh` — **pnix-rs / rs-meta로 같은 문제를 어떻게 해결**하는지 (실제 `pnix-rs` CLI 호출).

모든 파일에 **한글 주석**이 있고, 각 섹션 `README.md`가 "무엇을 / 왜 / 어디에 쓰나 / 쉽게 말하면"을 정리합니다.

## 쉽게 말하면

```text
plain Rust
= 강타입·메모리안전하지만, 순수/증거/의미보존/재현성/사영은 직접 다 챙겨야 함

pnix-rs / rs-meta
= 실행하면서 "왜 안전한가, 무엇을 했나, 같은 의미인가, 증거가 뭔가, Rust↔px 사영이 합치하는가"를 같이 남김
```

이 lane의 축은 **Rust ↔ px 사영**입니다: pnix-rs는 rs-meta(Rust-in-Rust meta-circular 컴파일러/평가기)를
기판으로 삼아, px 값을 Rust로 사영하고 Rust를 px로 물화하며, 그 왕복을 **rs-meta 자신이 판정**합니다.

## 핵심 대비 (한 줄 요약)

| 섹션 | plain Rust의 한계 | pnix-rs / rs-meta |
|---|---|---|
| `01-pure-sandbox` | 실행 전 순수성/효과 판정 게이트가 없음 | 순수/효과-클래스 admission + 미지 builtin **fail-closed** |
| `02-canonical-hash-and-drift` | 의미의 안정적 내용주소 해시가 없음(비재현) | 정본 IR + **sha256 내용주소** + identity sharing |
| `03-mirror-roundtrip` | parse→재출력의 의미보존을 스스로 증명 안 함 | **mirror**: 모든 단면 + emit 고정점 + lossless |
| `04-rust-pnix-projection` | Rust↔표현 사영을 substrate로 증명 못 함 | **rust-mirror**: px값→Rust 3-way + Rust→px→Rust 재구성(AST 동일) |
| `05-witness-and-gate` | 실행에 증거/권한 기록이 없음 | 13-필드 **witness** + **capability gate** |
| `06-specialization-futamura` | 부분평가/잔여 프로그램 생성이 없음 | **specialize** + **1·2차 Futamura 사영** |
| `07-incremental` | 이름 기반 캐시(alpha-rename에 무효) | 의존성-치환 content hash(**알파 불변**) + realisation cutoff |
| `08-compartment-isolation` | 런타임 capability 격리 환경이 없음 | **compartment**(SES식 자기 env/모듈) |
| `09-action-checkpoint` | 종합 승인 verdict가 없음 | **action** = gate+mirror+ir+witness 단일 판정 |
| `10-substrate-contract` | interp==compiler 강제가 없음 | **substrate-check**: rs-meta interp == rustc == native |
| `11-self-hosting-tower` | 코드=데이터(S=L) 자기해석기가 없음 | **tower**: px 자기해석기 == 네이티브(reify/reflect) |
| `12-bta-analysis` | binding-time analysis가 없음 | **bta**: static/dynamic 분류 + specializer 교차검증 |
| `13-peer-engine` | 두 엔진을 공통 제어면(.px)으로 잇는 표준이 없음 | **engine-verdict/attestation** = rs-meta를 peer 엔진으로, `.px` 봉투로 라우팅 |
| `14-runners-and-repls` | 두 언어의 컴파일러/인터프리터 실행기 분류가 없음 | 역할별 flake runner + px/Rust **REPL**(인터프리터 모드) |
| `15-embed-pnix-in-rust` † | 호스트 Rust에 임베드한 pnix는 죽은 문자열(read-time 승격 없음) | 임베드 pnix를 **평가+Rust 사영+witness** — 언어-수준 interop(형제 08 대응) |
| `16-jones-optimality` | 부분평가가 Jones-최적인지 측정할 표준이 없음 | **jones-check**: 인터프리터 비만-불변 + 인터프리터-free + 프로그램 추적성 |
| `17-welltyped-floor` | 타입검사 판정을 재검토 가능한 증거로 안 남김 | **welltyped-check**: meta-circular 플로어 typeck 재인증 + 이빨(음성 사례) |
| `18-cogen-second-futamura` | 2차 Futamura 사영(컴파일러 생성기)이 없음 | **cogen-check**: 프로그램별 컴파일된 residual 직접 생성 |
| `19-typed-attestation` | 타입 있는 증명서(predicate-subject)가 없음 | **attest-check**: witness → in-toto/SLSA식 typed attestation |
| `20-verifying-cache` | 빌드 캐시가 신뢰만 하고 재검증 안 함 | **verifying-cache-check**: hit 재검증 + 변조 탐지(이빨) |
| `21-ir-diff` | 텍스트 diff뿐, 의미 갈라지는 위치를 구조로 못 짚음 | **ir-diff-check**: 정본 IR 구조 diff + 첫 차이 위치 |
| `22-capability-attenuation` | effect 권한의 비가역적 축소 모델이 없음 | **attenuate-check**: 감쇠 + 비가역성 + 회수 + 연쇄 감쇠 |
| `23-specialization-soundness` | 특화의 가정 의존성/유효성을 안 추적함 | **phase-check + assumption-check**: 구조 경계 + stale 탐지·재특화 |
| `24-differential-certificate` | 배터리 동등성을 재검토 가능한 인증서로 안 묶음 | **certify-check**: 내용해시 인증서 + 결정성 + 이빨 |
| `25-cross-host-oracle` | 참조 데이터의 cross-project 스키마 고정을 안 게이트함 | **cross-host-check**: 오라클 export drift + witness 스키마 고정 |
| `26-stage-ladder` | 여러 실행 경로의 의미 일치를 코퍼스 규모로 안 검사함 | **stage-check**: direct/normalized/CAS/AST-roundtrip/replay 5경로 일치 |
| `27-reflect-tower-coherence` | reify/reflect 반복의 탑 정합성 개념이 없음 | **reflect-tower-check**: 레벨 1·2 정합 + 메타-레벨 투명성 |
| `28-project-health-gates` | 문서/설명이 구현과 어긋나는지 사람 리뷰에만 의존 | **explain/capabilities/registry-check**: 자기정합 3종 게이트 |

## 두 언어 · interop — 형제 프로젝트와의 대응

pnix-rs는 **Rust(rs-meta)와 pnix(px.rs)를 각각 온전히 구현하고 서로 interop**시킨다
(두 언어를 섞은 새 언어가 **아니다**). ~/pnix-hy(Hy↔pnix)·~/pnix-clj(Clojure↔pnix)와
같은 기둥을 Rust 판으로 세운 것이다. 언어-수준 interop 대응:

| 형제 canonical | 무엇 | pnix-rs |
|---|---|---|
| `04-host-interop-loss-effect` | host↔pnix 값 crossing의 loss/effect/capability | `04-rust-pnix-projection`(rust-mirror 값 사영) |
| `07-{host}-macro-over-pnix` | 호스트 매크로를 pnix 위에 적용 | rs-meta 코드생성(rust-mirror)이 그 역할 — Rust는 자기 자신이 Rust-in-Rust 컴파일러(`macro_rules!`는 rs-meta에서 held) |
| `08-{host}-reader-embed-pnix` | 호스트 reader가 pnix를 read-time 임베드 | **`15-embed-pnix-in-rust`**(호스트 Rust 소스에 pnix 임베드→승격·평가·사영·witness) |

## 실행

```sh
# 1) nix 설치 후 devShell 안에서 (권장 — pnix-rs가 PATH에, RS_META_BOOTSTRAP 자동 설정)
nix develop
cd pnix-rs && ./examples/run-all.sh

# 2) 소스에서 빌드한 바이너리로
export PNIX_RS=/path/to/pnix-rs
export RS_META_BOOTSTRAP=/path/to/rs-meta/bootstrap   # substrate/rust-mirror/cross-host에 필요
./examples/run-all.sh

# 개별 예제
rustc -O examples/01-pure-sandbox/limit_rust.rs -o /tmp/limit && /tmp/limit   # 한계
bash examples/01-pure-sandbox/pnix_rs_way.sh                                   # 방식
```
- **13-peer-engine** — rs-meta as a Rust-domain peer engine on a common `.px` control plane (profile/verdict/artifact envelopes; why meta-circular, not just rustc)
