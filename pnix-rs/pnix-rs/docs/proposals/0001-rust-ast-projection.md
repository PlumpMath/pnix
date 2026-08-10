# 0001 — Rust AST 구조 축 projection (P6 v1)

상태: **v8 DONE — 제네릭 struct/impl** (2026-07-03) — v7(제네릭 함수)에 이어
제네릭 struct(`struct Wrap<T>`)/impl(`impl<T> Wrap<T>`)/method + 제네릭 타입
`G(Name,[args])` 왕복. struct/enum/impl/method에 generics 배선(sig+Rust 양방향).
held: 트레이트 solving/클로저 projection(수요 시 vN).

## v8 구현 기록 (2026-07-03)
- struct/enum/impl/method 아이템에 sig_typed_generics 배선(fn 방식 연장).
  impl은 `impl<G> Target`(generics가 impl 직후, target 앞). 렌더 generics_to_
  sig/rust 양방향. G() 타입은 v7에서 이미 처리.
- 게이트: rust-mirror-check 31/31 — generic-struct-v8(Wrap<T>/impl<T>/unwrap)
  첫 빌드에 3중 통과(AST 동일+rustc 정합). welltyped-check도 무영향(5/5).

## v7 구현 기록 (2026-07-03)
- 전제 수정(rs-meta): sig_generics로 fn/struct/enum/impl/method에 `<T,U>`
  직렬화 — E1c emit 제네릭-완전과 sig 정합(ast-canonical-check 4/4).
- pnix-rs: sig_typed_type에 `G(Name,[args])` 제네릭 타입, sig_typed_generics
  (`<T,U>` 파싱), rfn에 generics 필드, type_to_sig/rust·generics_to_sig/rust
  양방향. rust-mirror-check 30/30 — generic-fn-v7(id<T>/first<A,B>) 3중 통과.

## v6 구현 기록 (2026-07-03)
- 파서: let-mut(`let mut NAME:_=` — mut 키워드 판별), assign/while/foreach/
  un(deref)/ref 표현식, if-else의 `_`(no-else) 허용(rnoelse).
- 렌더: rlet mut 키워드, rassign/rwhile/rforeach/runary/rref 양방향, rnoelse
  (sig→`_` / Rust→else 생략), `rem`→`%`.
- 게이트: rust-mirror-check 29/29 — mirror_probe.rs-v6가 전체 왕복 3중 통과.
  디버깅 1건: if-without-else의 else가 블록 아닌 `_`(rnoelse 도입).

## v5 구현 기록 (2026-07-03)
- 파서: sig_typed_variant(`Name(types)[fields]`), sig_typed_pattern(bind/
  penum/penumstruct), enum 아이템 디스패치, match/eslit 표현식.
- 렌더: sig_pattern/rust_pattern, sig_variant/rust_variant + renum/rmatch/
  reslit 양방향. rust_variant는 tuple/struct/unit variant 구별,
  rust_pattern은 penumstruct의 rest(`, ..`)까지.
- 게이트: rust-mirror-check 28/28 — enum-match-v5 샘플(Shape Circle/Rect,
  match 두 arm, Shape::Circle(2)/eslit)이 **첫 빌드에 3중 통과**(누적된
  v3/v4 헬퍼 재사용). mut/루프 held 프로브(v6).

## v4 구현 기록 (2026-07-03)
- sig_typed_type(`i64` prim / `N(Point)` named), sig_typed_params/sig_params/
  rust_params 공유 헬퍼. sig_typed_item에 struct/impl 디스패치, sig_typed_
  method(recv ∈ assoc/&self/self/&mut self). expr에 slit/field/pcall/mcall.
- 렌더: type_to_sig(named→`N(...)`)/type_to_rust(named→bare), 새 kind 7종
  (rstruct/rimpl/rmethod/rslit/rfield/rpcall/rmcall) 양방향(sig 재생성 +
  Rust 재구성). 메서드 recv는 Rust에서 첫 파라미터로 복원(assoc는 없음).
- 게이트: rust-mirror-check 27/27 — struct-impl-v4 샘플(Point/origin/sum/
  self.x/Point::origin()/p.sum())이 AST 동일 + rustc 정합. enum held 프로브.

## v3 구현 기록 (2026-07-03)
- sig_typed_program: fn 아이템(파라미터/리턴 타입)/블록(`ex`·`let x:_=` 문 +
  tail)/call/println/if-블록 — factorial.rs가 통째로 typed px 트리가 됨.
- sig_typed_render_v3: byte-identical sig 재생성(전 프로그램).
- **rust_render: px 트리 → Rust 소스 재구성**(역방향 접합). 수용 기준은
  텍스트가 아니라 **AST 동일성** — ast-canonical(재구성) == ast-canonical
  (원본), rs-meta 자신이 판정 — + rustc 출력 정합(witness direction
  rust-reconstruction). rust-mirror-check 26/26 (factorial.rs, add3-with-let
  둘 다 3중 통과; struct 아이템은 held 프로브).

## v2 구현 기록 (2026-07-02)
- sig 핵심 expr 노드(int/var/bin/if)를 typed px 노드(rint/rvar/rbin/rif)로
  파싱(sig_typed_parse)·byte-identical 재생성(sig_typed_render).
- **tower join**: px로 쓴 번역기 `runtime/tower/rust_bridge.px`가 typed Rust
  노드를 P11 tower 인코딩으로 옮기고, px 자기해석기가 평가 —
  **rustc(native tier) == px self-interp** 를 같은 Rust 표현식에서 검증
  (witness direction rust-typed-projection). 3-기판 합치: rustc == rs-meta
  interp(rs-meta 자체 TV) == px tower.
- 프로브: `6 * 7`→42, `(1+2)*(3+4)`→21, `if 2<3 {10} else {20}`→10 전부 일치
  + call 노드는 typed core 밖 held(브래킷 트리 v1a가 전 커버리지 유지).
- **v3 (HELD)**: call/let/match/item 레벨 typed 커버리지, px에서 Rust AST
  재구성해 rs-meta emit과 접합(역방향).

## v1a 구현 기록 (2026-07-02)
- rs-meta에 `ast-canonical` CLI 추가됨(commit 003e7183; mirror-sig serializer의
  src/sig.rs 승격 — pnix 무관 범용, 안정성 근거 = stage3-mirror 3-레벨 증명).
- pnix-rs rust_mirror.rs: `sig_tree_to_px`(quote-aware bracket-tree 파서) /
  `px_to_sig_text`(byte-identical 재생성) / `rust_ast_roundtrip`(witness
  direction rust-ast-projection). 두 왕복 요구: ① 재생성 == 원 sig 텍스트
  ② px 임베드(px_print→px_parse→px_eval 왕복 — reified Rust AST가 진짜 px
  데이터로 생존).
- 검증: rust-mirror-check 13/13 (factorial.rs, mirror_probe.rs AST-axis
  lossless).
- held edge(기록): 비균형 브래킷 문자를 담은 Rust char 리터럴('(' 등)은 sig에
  raw로 나와 트리 파싱이 정직 거부.
- **v2 (HELD)**: sig 텍스트의 typed-kind px 인코딩(fn/struct/expr 노드를
  P11 tower 인코딩과 정렬) + px에서 Rust AST 재구성해 rs-meta emit과 접합.

## 목적
px attrset ⇄ Rust AST의 구조 projection: rs-meta가 파싱한 Rust 프로그램의 AST를
px 데이터로 reify하고, px 쪽에서 재구성해 되돌리는 왕복(+손실 어휘, witness).
pnix-hy의 hy_mirror가 Hy 폼을 pnix 데이터로 정렬하는 축의 Rust 대응물.

## 막힌 지점 (held 사유)
rs-meta bootstrap의 `ast` 명령은 rustc derive Debug 포맷을 출력한다 — 기계 파싱
대상이 아니고 안정성 보장이 없다. 구조 축에는 **안정 직렬화**가 필요하다.

## 필요한 것 (rs-meta 쪽, pnix 무관 범용 기능)
rs-meta에 `ast-canonical -c|-f` 명령: `proofs/mirror-sig.rs`의 canonical AST
serializer(이미 stage3-mirror-check에서 3-레벨 byte-identical이 증명된 그 직렬화)
를 CLI로 노출. 출력 스키마는 sig 문법 그대로(`fn name(params)->ret {...};`).
- rs-meta는 pnix를 모른다 — 이 기능은 "Rust AST의 안정 직렬화"라는 rs-meta 고유
  가치(mirror 증명 표면의 CLI화)로 제안한다. pnix 언급 없이.

## 이 lane 쪽 작업 (rs-meta 기능이 생긴 뒤)
1. `ast-canonical` 출력을 px 파서가 아니라 **전용 sig 파서**(rust_mirror.rs 내)로
   읽어 px attrset 인코딩으로 변환 (`{ kind = "fn"; name = ...; }` — P11 reify
   인코딩과 정렬).
2. px 쪽 재구성 → sig 텍스트 재생성 → rs-meta `ast-canonical` 재출력과
   byte-identical = lossless.
3. witness direction "rust-ast-projection", 손실 어휘 적용.

## 수용 기준
- corpus: rs-meta samples/mirror_probe.rs + factorial.rs 왕복 lossless.
- rust-mirror-check에 AST 축 항목 추가 (v0 값 축과 별도 카운트).
