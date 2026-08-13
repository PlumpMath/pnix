# pnix-rs todo

- 2026-07-08 px rec attrset (워크트리 feat/px-rec, proposal 0006 runtime surface
  수요 발생): rec { a = 1; b = a + 1; } -- 파서에서 let <bindings> in { k = k; }로
  desugar해 기존 LetIn Rec 프레임(lazy 슬롯, 순서 무관) 재사용. 새 AST 변형 없음
  (mirror/ir/sig 무영향). 동적 키 rec은 held(Nix도 제한). nix 오라클 42 일치.
  rec 식별자 사용은 유지(뒤가 { 일 때만 특수). px-check +2 인라인 케이스(19->21).
  vendored corpus는 불변(cross-host 비교 보존). all_ready + substrate-check 유지. — rs-meta backed pnix runtime front-end

기준일: 2026-07-02 KST

재개 시 여기부터 읽고, `px-check` / `substrate-check`로 상태 확인 후 다음
슬라이스를 잇는다. 이 파일은 pnix-clj/pnix-hy의 todo와 같은 역할이다.

---

## 0. 정체성 / 아키텍처 (core decision)

```text
pnix-rs    = Rust bootstrap/front-end for the pnix runtime path (이 lane)
../rs-meta = Rust meta-circular stage15..N compiler/evaluator substrate (의존 대상)
pnix runtime (.px) = repo-owned runtime artifacts under runtime/
pnix mirror = pnix-side self-observation/evidence layer (추후 슬라이스)
```

- **pnix-rs는 rs-meta를 의존한다.** pnix-clj ↔ clj-meta("clj-meta backed pnix
  runtime"), pnix-hy ↔ hy-meta와 같은 관계의 Rust 형제 lane.
- **rs-meta는 pnix를 모른다.** pnix runtime/mirror/meta-circular 작업은 전부
  여기(pnix-rs)에서 한다. rs-meta에 pnix 코드를 넣지 않는다.
- 의존의 실증 형태(**substrate contract**): pnix-rs의 px 엔진 소스(`src/px.rs`)는
  **rs-meta가 평가하는 Rust subset 안에** 작성된다. `substrate-check`가
  rs-meta bootstrap으로 그 소스를 해석·실행하고, rustc 컴파일 결과 및 pnix-rs
  네이티브 바이너리 결과와 3-way 동등을 요구한다.

## 1. 헌법

1. **zero crates.io dependency.** std만. rs-meta 의존은 substrate binary 호출
   (`RS_META_BOOTSTRAP`, 기본 `/tmp/rs-meta-target/release/bootstrap`)로 실증.
   (라이브러리(path-crate) 의존 형태는 rs-meta lib target 추가와 함께 후속 검토.)
2. **px 의미는 pnix 표준을 따른다.** 특히 **let은 재귀 스코프**(형제/자기 참조
   가능, 순차 아님 — pnix-hy 감사 A4에서 확정된 의미). `seed_let_rec.px`가 회귀
   가드.
3. **정직.** 세 lane 공통: DONE=돌고 검증됨 / TODO / HELD. seed 범위 밖(-floats,
   strings, lists, `//`, selection, builtins, 문자열 interpolation)은 명시 미지원.
4. **두 번째 평가기 금지 원칙 존중** (pnix-hy 규칙): px 평가기는 이 lane에 하나.
   mirror/gate 중복 생성 금지.

## 2. 현재 위치 — S1+S2 substrate wiring + px runtime 기판 (2026-07-02, DONE)

```text
pnix-rs/
  Cargo.toml               # zero-dep, bin = pnix-rs
  src/px.rs                # seed .px lexer/parser/evaluator (rs-meta subset 안)
  src/main.rs              # CLI: px-eval / px-check / substrate-check
  harness/substrate_harness.rs  # rs-meta multi-file run 용 (probe 임베드)
  runtime/corpus/*.px      # vendored(c05,c09 from pnix-clj rust_grounded) + seed
  todo.md
```

px runtime 기판 범위 (전부 실측 green):
- int/bool, `+ - * /`, `== != < <= > >=`, `if/then/else`, `#` 주석
- 문자열 + `${...}` interpolation, 리스트 `[ a b ]` + `++`,
  attrset `{ k = v; }` + `//` merge + `.name` selection
- 람다 `param: body`, juxtaposition application `f x y`(좌결합, Nix 우선순위 체인)
- **재귀 let** (형제·자기 참조 — pnix 의미; `seed_let_rec.px` 가드)
- builtins 15종: toString/stringLength/concatStringsSep/substring/length/map/
  filter/foldl'/attrNames/hasAttr/sort/head/tail/elemAt/elem
- canonical print: 키 정렬 attrset, `[ a b ]`, 따옴표 문자열

검증 (전부 PASS):
- `px-check` 9/9 — invariance corpus c02~c05, c07~c09 (pnix-clj
  `oracles.edn`의 rust-grounded 의미값과 전부 일치) + seed 2종.
- `substrate-check` 1/1 — **rs-meta interp == rs-meta rustc == pnix-rs native**,
  corpus 7종 transcript 3-way 동등. 확장 중 `String::clear` subset 거부를
  rs-meta가 정확히 잡아냄(= substrate contract가 실제로 작동).

## 3. 검증 명령

```sh
cd ~/pnix-rs/pnix-rs
export CARGO_TARGET_DIR=/tmp/pnix-rs-target
cargo build --release
P=/tmp/pnix-rs-target/release/pnix-rs

$P px-check
RS_META_BOOTSTRAP=/tmp/rs-meta-target/release/bootstrap $P substrate-check
$P px-eval -c 'let a = 1; b = a + 2; in a + b'
$P px-eval -f runtime/corpus/c05_recurse.px
```

(substrate-check 전에 ../rs-meta를 release로 빌드해 둘 것:
`cd ../rs-meta && CARGO_TARGET_DIR=/tmp/rs-meta-target cargo build --release`)

## 4. 로드맵 — pnix-hy의 길을 Rust 방식으로 (2026-07-02 확정)

### 4.0 어느 길인가 (박아두는 선언)

**걷지 않는 길 (~/clj-msv, ~/pnix-old):**
- AI 에이전트 / coding-agent 런타임 ❌
- task routing / plan synthesis / autonomous 실행 ❌
- MSV / gate-graph 실험 ❌
- corpus 표면(문장처리) 갈기 자체가 목적이 되는 것 ❌

**걷는 길 (~/pnix-hy의 길, Rust 방식으로):**
> pnix-rs는 **human-operated meta-circular language projection lab**이다.
> 연구 대상은 언어 표현력 projection이다:
> ```text
> Rust/rs-meta ecosystem 표현력
> <-> rs-meta stage15..N compiler/evaluator evidence
> <-> rs-meta AST/emit으로 도달 가능한 Rust 표면
> <-> .px로 쓰인 pnix runtime 표현
> <-> pnix-side mirror 관찰
> ```
> 모든 기능은 "Rust↔pnix projection과 mirror evidence를 개선하는가"로만
> 판단한다. 에이전트를 굴리는가/작업을 라우팅하는가로 판단하지 않는다.

**호스트 언어 경계 (2026-07-02 명시):** pnix-hy는 **길의 모범(구조·수준)일 뿐**이다.
이 lane은 **Python도 Hy도 다루지 않는다** — 호스트는 오직 Rust/rs-meta,
projection은 오직 **Rust↔px**. pnix_hy 모듈명이 §4.1에 나오는 것은 대응
관계(무엇을 어떤 순서로 어떤 수준으로)의 참조지, Python/Hy 코드를 읽어와
쓰거나 이식한다는 뜻이 아니다. pnix-hy/pnix-clj와의 접점은 P13 cross-host
하나뿐이며, 그것도 .px 결과물/witness의 **비교**이지 그쪽 호스트를 만지는 게
아니다.

### 4.1 사다리 (pnix-hy가 걸은 순서 → pnix-rs의 Rust 판)

pnix-hy 모듈 → pnix-rs 파일 대응과 각 단계의 증명 커맨드. 각 단계는 pnix-hy의
해당 모듈이 실증한 형태(문헌 포함)를 따르되 구현은 이 lane 고유(Rust, zero-dep,
substrate-checked).

### 4.1.0 ⚙️ 코덱스 작업 규칙 (모든 항목 공통 — 필독)

1. **환경**: 작업 디렉터리 = `~/pnix-rs/pnix-rs`. 빌드:
   `export CARGO_TARGET_DIR=/tmp/pnix-rs-target && cargo build --release`,
   바이너리 `P=/tmp/pnix-rs-target/release/pnix-rs`. substrate-check 전에
   `cd ../rs-meta && CARGO_TARGET_DIR=/tmp/rs-meta-target cargo build --release`.
2. **매 슬라이스 후 필수 green** (하나라도 깨지면 커밋 금지):
   `px-check` 9/9 · `mirror-check` 9/9 · `stage-check` 9/9 · `ir-check` 11/11 ·
   `gate-check` 15/15 · `interop-check` 4/4 · `substrate-check` 1/1 (+ 새로 추가한 자기 check).
3. **px.rs 수정 시 rs-meta subset 준수** — substrate-check가 게이트다. 실패하면
   stderr의 `typeck: ...` 라인이 정확한 위반 지점을 알려준다. 알려진 함정:
   `String::clear` ❌(빈 String 재대입로), `{:?}`에 `Option<&T>` ❌(match로 풀어
   `{:?}`는 `&T`에만), 문자열/char `<` ❌(`px_str_lt`류 byte 비교), `char as i64`
   ❌(`digits.parse::<i64>()`), `Vec::sort` ❌(selection sort 손구현),
   `use std::...`는 파일 상단에만.
4. **금지**: 두 번째 평가기/mirror/gate 생성 ❌ (모든 평가는 `px.rs`의 sacred
   runtime 경유). rs-meta에 pnix 코드 추가 ❌ (rs-meta 수정이 필요하면 pnix 무관
   범용 기능으로 rs-meta todo에 제안만). Python/Hy 코드 접촉 ❌. crates.io 의존
   추가 ❌. 의도적 placeholder를 미구현으로 재해석 ❌.
5. **스키마 동결**: witness 13필드(gate.rs `WITNESS_FIELDS`) 순서·이름 변경 ❌.
   roundtrip 상태 어휘(lossless/lossy-ok/held/rejected) 변경 ❌. 기존 receipt
   스키마(`pnix-rs.*.v0`)는 필드 추가 시 v1로 올리고 마이그레이션 명시.
6. **커밋 단위**: P별 1커밋(사전 리팩토링이 크면 분리 가능). 메시지에 P번호
   명시. 완료 시 이 todo의 해당 항목을 [x]로 바꾸고 증거(체크 카운트) 기록.
7. **정직**: 각 P의 "명시 미주장" 블록을 구현으로 넘어서지 말 것 — 넘어서고
   싶으면 todo에 새 항목을 먼저 만든다.

- [x] **P0 runtime 기판** (`pnix_runtime.py` → `src/px.rs`): px 단일 평가기(sacred,
  두 번째 평가기 금지). S1+S2로 corpus c02~c05,c07~c09가 oracles.edn 의미값과
  일치, substrate-check 3-way 동등. laziness(thunk memo)·mirror events는 P1/P2
  수요 시 이 파일에서만 확장. (2026-07-02)
- [x] **P1 mirror** (`pnix_mirror.py` → `src/mirror.rs`): **singleton mirror_run**
  구현 — 단일 canonical 진입점 `mirror_run(source)`이 모든 facet 방출:
  schema(`pnix-rs.mirror.v0`)/source_fnv/tokens/value/value_fnv/emit/emit_fnv/
  reparse_ok/revalue_match/emit_fixed_point/status(+error). roundtrip 상태 어휘
  `lossless / lossy-ok / held / rejected` 고정 — held는 opaque leaf(lambda/
  builtin 포함 값), rejected는 parse/eval/reparse 실패 또는 값 드리프트.
  px emitter `px_emit`(+`px_tokens`/`px_value_has_opaque`)는 기판(px.rs)에 —
  **emit 고정점**(emit(reparse(emit))==emit)이 mirror 불변. 해시는 FNV-64 명시
  (SHA-256은 P3 항목). 증명(전부 첫 실행 PASS, 2026-07-02):
  - `mirror-check` 9/9 — corpus 전체 **lossless** (값 보존 + emit 고정점).
  - substrate-check에 `mirror_c05` 프로브 추가 — **emitter 자체가 rs-meta 해석
    하에서 돌아** emit→reparse→eval 왕복이 rustc·native와 3-way 동등.
  - CLI `mirror -c|-f`(facet receipt), `mirror-check`.
- [x] **P2 pnix runtime stage ladder** (`stage.py` → `src/stage.rs`): host(rs-meta)
  stage와 구분되는 **pnix 런타임 자체의 안정성 사다리** 구현(schema
  `pnix-rs.stage.v0`): `px-stage1` direct eval / `px-stage2` normalized-AST
  eval(`px_normalize` — 인접 Lit 병합, attrset·유일이름 let 바인딩 정렬; 재귀
  let 의미 보존, 중복 이름 프레임은 정직하게 순서 유지) / `px-stage3`
  content-addressed store eval(정규화 emission을 content key로 저장→인출→평가) /
  `px-stage4` AST roundtrip 무결성(emit 고정점+값 일치) / `px-stage5`
  deterministic replay(신선한 재실행이 모든 hash 재현) / **closure**(전 단계
  동일 값 hash). px_normalize는 기판(px.rs, substrate typeck 포함; 런타임
  substrate 프로브는 P3 IR 프로브와 함께). 증명(첫 실행 PASS, 2026-07-02):
  `stage-check` 9/9 closure. CLI `stage -c|-f`(사다리 receipt), `stage-check`.
- [x] **P3 IR** (`ir.py` → `src/ir.rs`): canonical IR 레이어 구현(schema
  `pnix-rs.ir.v0`). IR = normalized AST — **직접 평가 가능**(px_eval이 그대로
  실행, 값 동등) + **canonical 고정점**(normalize(reparse(ir_text))의 emission
  == ir_text) + **content-addressed**(`ir_sha256` = in-house SHA-256, zero-dep,
  `src/sha256.rs`; ir-check가 FIPS 180-4 벡터 self-test를 먼저 통과시킴).
  원칙 receipt에 고정: "IR-is-canonical; host artifacts are cache" (rs-meta
  헌법과 동일 선). **identity sharing 실증**: 바인딩 순서만 다른 프로그램들이
  같은 ir_sha256 공유(이름은 메타데이터 — P9 Unison 모델의 전조).
  증명(첫 실행 PASS, 2026-07-02): `ir-check` 11/11 (sha 벡터 + corpus 9 +
  identity sharing). CLI `ir -c|-f`, `ir-check`.
- [x] **P4 gate/witness** (`gate.py` → `src/gate.rs`): 구현(schema
  `pnix-rs.gate-check.v0` / `pnix-rs.witness.v0`).
  - static purity walk: `builtins.<name>` 사용 수집; unknown builtin과
    **builtins가 값으로 탈출**하는 경우는 uncertain → **fail-closed**.
  - effect-class 어휘 선언(file-read/file-write/host-call/import/network) +
    effect_of 표 — seed builtin은 전부 pure이므로 표는 비어 있음을 **명시**
    (미래 host builtin의 필수 진입 경계; P5 interop과 짝).
  - capability admission `gate_check(source, granted)`: denials 없음 ∧
    uncertain 없음일 때만 승인.
  - **witness**: 13필드 공유 스키마 그대로(direction…loss), SHA-256 content
    hash(in/out/env), eval·mirror·ir 3방향 빌더, 결정성 검증.
  증명(첫 실행 PASS, 2026-07-02): `gate-check` 15/15(어휘 검증 포함) — corpus 9 pure 승인
  (builtin 사용 집계 포함), fail-closed 2종, witness 3방향 결정성+스키마.
  CLI `gate -c|-f`, `gate-check`, `witness -c|-f`.
- [x] **P5 interop 경계** (`interop.py` → `src/interop.rs`): host-call의 유일한
  통로. 현재 이 lane의 host 접촉은 정확히 두 가지 — ① rs-meta bootstrap 서브프로세스
  (substrate-check) ② corpus 파일 읽기(fs::read_to_string). 이 둘을 interop로 모은다.
  - [x] `src/interop.rs` 생성:
    - `pub fn check_capability(effect: &str, granted: &[String]) -> Result<(), String>`
      — effect가 granted에 없으면 `Err("capability denied: {effect}")`.
    - `pub fn host_run_bootstrap(bootstrap: &str, mode: &str, files: &[&str],
      granted: &[String]) -> Result<String, String>` — `check_capability("host-call")`
      통과 후에만 `Command` 실행(현 main.rs `run_bootstrap` 본문 이동). 성공 시
      stdout 반환.
    - `pub fn host_read_file(path: &str, granted: &[String]) -> Result<String, String>`
      — `check_capability("file-read")` 후 read_to_string.
    - `pub fn host_call_witness(mode: &str, in_desc: &str, out: &str,
      granted: &[String]) -> gate::Witness` — direction "host-call",
      source_lang "px-lane", target_lang "rust-substrate", effect_class
      "host-call", capability_required "host-call", in/out/env sha256.
  - [x] main.rs 교체: `run_bootstrap` 삭제 → interop 경유. 각 cmd가 자기 목적에
    필요한 최소 grant를 **스스로 선언**해 넘긴다(px-check/mirror-check 등 corpus
    읽기 = `["file-read"]`, substrate-check = `["file-read","host-call"]`).
    grant는 CLI 사용자 플래그가 아니라 명령의 선언된 정책이다(레코드에 남음).
  - [x] 불변 문서화(코드 주석 + §5): PxVal에는 host 객체 variant가 없다 —
    호스트 결과는 항상 문자열/데이터로 경계를 건넌다(OpaqueNative가 필요해지는
    순간이 오면 새 variant가 아니라 held 처리 후 proposals로).
  - [x] `interop-check` (CLI + main.rs): ① grant 없이 host_run_bootstrap →
    "capability denied" ② grant 없이 host_read_file → denied ③ 정당 grant로
    substrate 스모크 1회 성공 + host-call witness 생성(13필드) ④ witness 결정성
    (같은 입력 2회 → 동일 렌더). 기대: 4/4.
  - 명시 미주장: 프로세스 샌드박싱/파일시스템 격리(OS 수준)는 범위 밖 —
    capability는 lane 내부의 admission 규율이다.
  - 완료(2026-07-02): interop-check 4/4. main.rs의 모든 fs 읽기/서브프로세스가
    interop 경유로 이관(run_bootstrap 삭제, load_source -f 포함). gate-check에
    effect 어휘 검증 추가(15/15). 전 게이트 green 유지.
- [x] **P6 projection v0 — 값 축** (`hy_mirror.py` → `src/rust_mirror.rs`):
  이 lane 고유의 projection **Rust ↔ px**를 값(value) 축부터. px canonical 값을
  Rust 프로그램으로 사영해 rs-meta substrate에서 실행하고, 돌아온 출력이 px
  canonical과 일치하는 왕복(hy_mirror의 값 정렬 축 대응).
  - [x] `src/rust_mirror.rs` 생성:
    - `pub fn px_value_to_rust_expr(v: &px::PxVal) -> Result<String, String>` —
      Int→`{n}i64`, Bool→`true/false`, Str→이스케이프된 `&str` 리터럴,
      List→`vec![...]`(원소 재귀; 혼합 타입 리스트는 v0에서
      `Err("held: heterogeneous list")`), Attrs→**키 정렬된** `vec![("k", ...)]`
      pairs. Closure/Builtin→`Err("held: opaque")`.
    - `pub fn px_value_to_rust_print_program(v: &px::PxVal) -> Result<String, String>`
      — 위 표현을 **px canonical 문법으로 다시 출력하는** `fn main` Rust 프로그램
      생성(즉 Rust 쪽이 px_print를 재연): Int는 `println!("{}", n)`, Str은
      `println!("\"{}\"", s)` 식, List/Attrs는 push_str 조립. 생성된 Rust는
      **rs-meta evaluated subset 안**이어야 한다(그래야 interp/rustc 양쪽 실행).
    - `pub fn rust_value_roundtrip(px_source: &str, granted: &[String])
      -> Result<(gate::Witness, &'static str), String>` — px 평가 → 값 →
      print-program 생성 → `interop::host_run_bootstrap`으로 rs-meta `run -c`와
      `native-run -c` 실행 → 두 stdout과 px canonical 3-way 비교 →
      상태(lossless/held/rejected) + witness(direction "rust-projection",
      target_lang "rust", loss_status=상태).
  - [x] `rust-mirror-check`: corpus 9종 — 값에 opaque 없는 것 전부
    lossless(3-way), opaque 값 프로브(`x: x`) → held. 기대: 10/10.
  - [x] CLI `rust-mirror -c|-f` (witness + 상태 출력).
  - [x] **P6 v1 (held — proposals로)**: Rust AST 구조 축(px attrset로 Rust AST
    reify ↔ 재구성). rs-meta에 안정 직렬화 `ast-canonical` 명령이 필요 —
    rs-meta는 pnix를 모르므로 **pnix 무관 범용 기능**으로 rs-meta todo에 제안
    (proofs/mirror-sig.rs의 serializer를 CLI로 노출하는 형태). 그 전까지 v1은
    held, 이 항목이 그 경계 기록이다. `docs/proposals/0001-rust-ast-projection.md`
    작성으로 시작.
  - [x] **P6 v1a (sig-tree 축) DONE (2026-07-02)**: rs-meta `ast-canonical`
    (범용, commit 003e7183) + rust_ast_roundtrip — sig 브래킷 트리를 px 값으로
    reify, ① byte-identical 재생성 ② px 임베드 왕복. rust-mirror-check 13/13
    (factorial/mirror_probe AST-axis lossless). typed-kind 인코딩은 v2 held
    (proposals/0001).
  - 명시 미주장: Rust 코드 생성이 곧 컴파일러라는 주장 ❌ — v0은 값 사영이다.
  - 완료(2026-07-02): rust-mirror-check 10/10 (corpus 9 lossless 3-way + opaque
    held). 구현 노트: 이질 컨테이너의 hetero-vec 타이핑 문제는 String-표현식
    합성으로 회피 — leaf는 네이티브 Rust 리터럴(i64/bool/&str), 구조는 Rust
    코드가 재조립(생성 Rust는 rs-meta subset 안, interp/rustc 양쪽 실행).
    v1(AST 축)은 docs/proposals/0001-rust-ast-projection.md로 held 경계 기록.
- [x] **P7 check 집계 + capabilities** (`cli.py --check`/`capabilities.py`):
  - [x] 리팩토링(선행): 각 `cmd_*_check`의 본문을 `fn *_report() -> Report`로
    분리(Report { name: &'static str, passed: usize, failed: usize,
    lines: Vec<String> } — rs-meta check.rs의 Report와 같은 모양). cmd_*는
    report를 출력하고 ExitCode 반환하는 얇은 껍질로.
  - [x] `pnix-rs check`: 등록된 모든 report 실행(px/mirror/stage/ir/gate/
    interop/rust-mirror + 이후 P들) → 각 `name: n/n ready` 나열 →
    `all_ready: true|false` (pnix-hy `--check` 형식). substrate-check 포함:
    RS_META_BOOTSTRAP(기본 경로 포함)에 바이너리가 없으면 **FAIL**(명시 메시지
    "substrate binary not found — build ../rs-meta first"; 조용한 skip 금지).
  - [x] receipt: `proof/check-receipt.txt` — 각 report 이름/카운트/ready +
    all_ready + receipt 자신을 제외한 본문의 sha256. proof/는 .gitignore에
    넣지 **않는다**(receipt는 커밋 대상; pnix-clj proof/ 관례).
  - [x] `pnix-rs capabilities` → `docs/CAPABILITIES.md` 생성: ① CLI 명령 표
    (이름/목적/schema/증명 커맨드) ② 모듈 표(파일→역할) ③ px 표면 목록(지원/
    미지원) ④ 스키마 목록(pnix-rs.*.v0). 소스는 하드코딩 함수(자기 기술) —
    이 함수가 곧 능력 인덱스의 단일 진실.
  - [x] `capabilities-check`: 생성 결과와 디스크의 docs/CAPABILITIES.md가
    byte-identical(docs_drift 게이트). 불일치 시 FAIL + "run `pnix-rs
    capabilities > docs/CAPABILITIES.md`" 안내.
  - 기대: check가 substrate 포함 전 report green으로 `all_ready: true`.
  - 완료(2026-07-02): 구현 노트 — in-process Report 리팩토링 대신 **clean-process
    self-replay**(interop::host_run_self로 자기 바이너리 서브프로세스, pnix-hy
    stage9 철학)로 집계: report별 프로세스 격리가 공짜로 따라옴. receipt
    schema pnix-rs.check-receipt.v0(본문 sha256 포함). interop에
    host_run_self/host_write_file(file-write) 게이트 추가.
    실측: check 9 reports all_ready true(17s), capabilities-check drift 게이트
    green.
- [x] **P8 specialize** (`src/specialize.rs`; 평가는 px.rs 재사용):
  px 부분평가 — 정적 부분을 fold하고 나머지는 **재파싱 가능한 residual px**로.
  - [x] `pub struct SpecializeRecord { residual: String, fully_static:
    Option<String> /* canonical 값 */, gaps: Vec<String>, witness: gate::Witness }`
  - [x] `pub fn specialize(source: &str, static_bindings: &[(String, px::PxVal)])
    -> Result<SpecializeRecord, String>`:
    - 상수 폴딩: Binary/If/Select/Apply의 인자가 전부 static 값이면 px_eval로
      접기(sacred runtime 재사용 — 두 번째 평가기 금지).
    - **재귀 let 규칙(A4 그대로)**: ① let의 바인딩 이름 집합 N 먼저 수집
      ② let 본문/바인딩 안에서 N에 속한 이름은 바깥 static env로 절대 해석 금지
      (형제 결과로만) ③ 고정점 반복: "참조하는 N-이름이 전부 이미 fold됨"인
      바인딩만 fold ④ 하나라도 fold 못 하면 **let 전체를 gap
      (`let-recursive-not-static`)으로 기록하고 dynamic residual로**
      (부분 fold로 순차 의미의 let을 방출하지 말 것 — 그 자체가 의미 불일치).
      건전성 우선, 특화력 손실 허용.
    - residual은 반드시 `px_parse` 통과(재파싱 게이트) + 자유변수는
      static_bindings에 없는 이름만.
  - [x] `specialize-check`: ① `let x = 5; in let y = x + 1; x = 10; in y` →
    fully_static "11"(안쪽 재귀 let에서 x=10이 이김) ② `let b = a + 1; a = 2;
    in b` → "3" ③ dynamic 형제(예: static_bindings 없이 자유변수 d 포함
    `let b = a + d; a = 2; in b`) → gaps 비어있지 않음 + residual 재파싱 OK
    ④ corpus 3종(c05/c09/seed_arith)을 static_bindings 없이 → fully_static ==
    px_run 결과(전부 닫힌 식이므로). 기대: 6/6 이상.
  - CLI `specialize -c|-f`.
  - 완료(2026-07-02): specialize-check 7/7. 구현 노트: fold 전략은 "폐쇄
    부분식이면 sacred runtime(px_eval)으로 통째 평가" — fold된 의미가 곧 런타임
    의미(건전성 by construction). 재귀 let은 폐쇄면 통째 fold, 아니면 A4대로
    구조 유지 + let-recursive-not-static gap(부분 fold 방출 없음). opaque 결과는
    gap 없이 원식 유지(임베드 불가는 손실이 아님). check aggregate 10 reports
    all_ready true.
- [x] **P9 incremental** (`incremental.py` → `src/incremental.rs`):
  - [x] 정의 단위 해시(Unison 모델): top-level `let`의 각 바인딩에
    **dependency-substituted hash** — 바인딩 식의 자유변수 중 형제 이름을
    "그 형제의 해시"로 치환한 canonical 텍스트(px_emit(px_normalize(...))에
    치환 적용)의 sha256. 이름은 메타데이터: 형제를 알파-리네임해도 참조하는
    쪽 해시 불변.
    - 상호 재귀(순환)는 v0에서 SCC 그룹 단위: 그룹 멤버의 canonical 텍스트를
      이름 정렬로 결합해 그룹 해시 하나, 멤버는 `group_hash + ":" + name`.
      (SCC 검출: 각 바인딩의 형제 참조 그래프에서 단순 DFS — corpus 규모라
      O(n^2)면 충분.)
  - [x] realisation record(Nix-CA 모델): `proof/realisations.tsv`
    (`ir_sha256\tvalue_sha256\twitness_out_hash` 행) append/조회 —
    `incremental_eval(source)`: ir_sha256 계산 → tsv에 있으면 **평가 생략**
    (early cutoff, value_sha256 반환 + cutoff=true), 없으면 평가 후 기록.
    파일 IO는 P5 interop(file-read/file-write grant) 경유.
  - [x] `incremental-check`: ① 알파-리네임 불변(형제 d→e 리네임 시 참조자
    해시 동일) ② 의미 변경(5→6)은 해시 변경 ③ 같은 소스 2회
    incremental_eval → 두 번째가 cutoff=true & 같은 value_sha256 ④ SCC 프로브
    (c05의 go/fib는 자기재귀 — 자기 SCC 그룹 해시 동작). 기대: 4/4 이상.
  - CLI `incremental -c|-f`, `incremental-check`.
  - 완료(2026-07-02): incremental-check 5/5. 스펙 이탈 기록: realisation store는
    proof/가 아니라 **work/realisations*.tsv**(캐시는 receipt가 아니므로
    gitignore 대상; check는 scratch store로 자기완결). 추가 실증: 바인딩 순서
    변형이 같은 realisation 히트(IR 정체성 기반 cutoff — P3와 접합). SCC 내부
    이름이 그룹 텍스트에 포함되는 v0 경계는 모듈 doc에 명시.
- [x] **P10 compartment** (`compartment.py` → `src/compartment.rs`): SES 형태
  격리 부기 — **두 번째 VM 아님**, 모든 평가는 px_eval 경유.
  - [x] `pub struct Compartment { env: Vec<px::PxFrame> /* 지속, REPL식 누적 */,
    modules: Vec<(String, String, bool)> /* name, px source, materialized */ }`
    - `new()` — 빈 env(builtins는 px_lookup 폴백으로 자동 공유 = shared frozen
      primordials).
    - `define(&mut self, name, px_source) -> Result<..>` — 평가 후 Bind frame
      push(누적 바인딩).
    - `register_module(&mut self, name, px_source)` — 등록만. 참조 시점에
      lazy materialize: `eval(&mut self, source)`가 unbound-variable 에러를
      만나면(또는 사전 스캔으로) 등록 모듈이면 평가해 Bind 후 재시도 — v0은
      **사전 스캔**(px 소스의 자유변수 후보를 걷어 등록 모듈명과 교집합이면
      먼저 materialize)로 단순하게.
    - `eval(&mut self, source) -> Result<String, String>` — self.env 위에서
      px_eval, canonical print 반환.
  - [x] `compartment-check`: ① A.define("x",…10) 후 B에서 x → unbound(격리)
    ② 두 compartment 같은 소스 → 같은 값(공유 intrinsics 순수성) ③ A의 모듈
    lazy materialize 1회만(재참조 시 재평가 없음 — materialized 플래그)
    ④ A에서 builtins 정상 동작(폴백 공유). 기대: 4/4.
  - CLI `compartment-check`.
  - 완료(2026-07-02): compartment-check 4/4 (격리/공유 intrinsics/lazy 1회
    materialize(count 관측)/REPL식 누적). 사전 스캔은 specialize의
    px_free_vars 재사용 — 새 기계 없음. check aggregate 12 reports all_ready.
- [x] **P11 tower milestone-1** (`tower.py` → `src/tower.rs` +
  `runtime/tower/*.px`): 문헌 기반(Amin&Rompf POPL'18, Jones/Gomard/Sestoft,
  3-Lisp), pnix-hy tower와 동일한 honest milestone 분할.
  - [x] **reify/reflect (Rust 쪽)**: `pub fn reify(e: &px::PxExpr) -> px::PxVal`
    — AST를 px attrset 인코딩으로: `{ kind = "int"; value = 5; }`,
    `{ kind = "lambda"; param = "x"; body = {...}; }`,
    `{ kind = "apply"; func = {...}; arg = {...}; }`,
    `{ kind = "var"; name = "x"; }`, `{ kind = "if"; ... }`,
    `{ kind = "binary"; op = "+"; lhs; rhs; }`,
    `{ kind = "let"; bindings = [ { name; value; } ... ]; body; }` (v0 인코딩
    표면은 int/bool/var/lambda/apply/if/binary/let — 문자열/리스트/attrs 인코딩은
    v1). `pub fn reflect(v: &px::PxVal) -> Result<px::PxExpr, String>` 역방향.
  - [x] **px로 쓴 self-interpreter** `runtime/tower/self_interp.px`:
    위 인코딩을 받는 px 람다 — `eval = env: node: if node.kind == "int" then
    node.value else if ...` 식. env는 `[ { name; value; } ... ]` 리스트로
    표현하고 조회 재귀 람다 포함. **주의**: 인코딩된 let의 재귀 의미는 v0에서
    비재귀(순차) let만 지원으로 **명시 축소**하고 gap 기록 — 재귀 let 인코딩은
    milestone-2 (self_interp에서 고정점 컴비네이터 필요).
  - [x] **acceptance harness** (`tower-check`): ① reify→reflect 왕복 == 원 AST
    (px_emit 비교) — corpus 부분집합(산술/lambda/apply/if) ② self_interp.px로
    인코딩 프로그램 3~5개(산술, 커링 apply, if, 비재귀 let) 평가 == 네이티브
    px_eval 값 (S=L 전제 실증: px가 자기 인코딩을 평가) ③ 인코딩 값의
    ir_sha256 결정성. 기대: 5/5 이상.
  - **명시 미주장**: 실제 cogen, full S=L(전 표면), stage-polymorphic 전체,
    재귀 let 인코딩 — milestone-2+ 로 held.
  - 완료 m1(2026-07-02): tower-check 11/11 — roundtrip 5종 + self-interp==
    native 5종 + 인코딩 결정성. 인코딩 임베드는 px_print 재사용(P1 성질 활용).
  - [x] **milestone-2 (2026-07-02): 재귀 let 인코딩** — self_interp.px의 env를
    bind 엔트리 + **재귀 프레임**({ rec = true; bindings; })으로 확장. 재귀
    프레임 히트 시 바인딩 노드를 "그 프레임에서 시작하는 env"로 평가(call-by-
    name 재귀 스코프), scanrev(tail-first)로 **뒤 바인딩이 앞을 shadow**(A4).
    tower-check 16/16 — 기존 m1 11종 + 재귀 5종(shadow 2/sibling 3/nested 11/
    fib 12=144/go 200+fib 12=20244) 전부 self-interp==native.
    **성능 경계(기록)**: 클로저 env가 인코딩된 프로그램(대형 attrset)을 담아
    호출마다 deep-clone — 트리 재귀 비용 = 인코딩 크기 × 호출 수. 프로브는
    소형 유지; persistent env 공유는 milestone-3 후보.
  - [x] **milestone-3a (2026-07-02): persistent 값/본문 공유** — 성능 경계 해소.
    rs-meta Val 패턴 그대로: PxVal::List/Attrs 페이로드 Rc화(px_list/px_attrs
    canonical 생성자), **PxExpr::Lambda body Rc화**(클로저가 본문을 공유 —
    call-by-name 재귀 lookup의 클로저 재생성이 Rc 범프로), PxFrame::Rec Rc화
    (self_interp 전체 AST가 프레임 클론마다 복사되던 것 제거). 결과: 원 스케일
    재귀 프로브(fib 20=6765, go 500+fib 20=132015)가 **5분+ 타임아웃 → 19.2s**
    (tower-check 16/16). substrate-check 통과(rs-meta 자신의 Val 패턴이라
    subset 그대로). check aggregate 27.7s all_ready.
  - [x] **milestone-3b (2026-07-02): str/list/attrs/select 인코딩** — reify/
    reflect와 self_interp를 전 표면으로 확장(str 보간 parts, list, attrs,
    select, `++`/`//` 연산). 동적 attrset 구성/선택을 위해 **Nix 표준 builtins
    3종 추가**(listToAttrs first-wins/getAttr/isAttrs — pnix-hy 런타임 의미와
    대조 후 동형 구현; 수요 기반 확장 규칙 충족, seed_list_to_attrs.px corpus
    가드). 인코딩된 프로그램에서 `builtins` 도달 + **비클로저 값 직접 적용
    폴스루**로 1차 builtins 호출 인코딩 지원. tower-check 23/23 —
    **원본 corpus c02(보간+builtins)/c05(재귀+attrs body)/c09가 통째로 인코딩
    되어 self-interp == native** + 인라인 4종 + 재귀/roundtrip 전부.
  - [x] **milestone-4 (2026-07-02): 게스트 클로저 → 고차 builtins 브리지** —
    px 런타임 무변경, 순수 게스트 코드로 해결(__functor식 px 의미 확장은
    pnix-hy 런타임에 없음을 확인하고 기각): `gapply` 헬퍼(클로저 attrset이면
    인코딩 apply, 아니면 직접 적용) + `gbuiltins = builtins // { map filter
    sort foldl' = 게스트 래퍼 }` — 래퍼가 host 클로저로 gapply를 감싸 게스트
    클로저를 host 고차 builtins에 흘림. 인코딩 프로그램의 var "builtins"는
    gbuiltins를 봄(관측 edge 문서화: builtins 자체를 print하면 래퍼 노출).
    tower-check 29/29 (16s) — **non-held corpus 원본 7종 전부**(c02~c05,
    c07~c09) 인코딩 실행 == native + 고차 인라인 2종(map 제곱, 역순 sort).
  - [x] **milestone-5 (2026-07-02): px로 쓴 specializer + cogen 수용 기준** —
    pnix-hy MIX_IN_PNIX 형태를 이 lane에 적응(`runtime/tower/mix.px`): 우리
    kind-인코딩, **strict 런타임 적응**(lazy env 매듭 대신 A4-보수 let 규칙 —
    이름 마스킹 + residual 보존; 호스트 specialize와 동일 건전성 선),
    `|| !` 없는 우리 연산 집합. senv = 이름→folded 노드 attrset(listToAttrs/
    getAttr/hasAttr — m3b builtins 재사용). 타입 판별용 Nix 표준 builtins 3종
    추가(isInt/isBool/isString — pnix-hy 런타임 존재 확인, seed_type_tests.px
    가드, builtins 21종). tower.rs: mix_in_px(residual node/folded/reflected
    source), cogen_acceptance(자기생성 = IR 해시 동등 + witness — pnix-hy
    self_generation_witness 대응, **기준 머신리이지 실제 cogen 아님 명시**).
    tower-check 35/35 — 닫힌 fold(42)/spec-time 베타(49)/static fold(35)/
    동적 residual `x * 5` + **mix 정확성 방정식**/A4 let residual(12)/
    cogen 기준 승인·거부. spec-time 비종결(무한 static 재귀)은 고전적 mix
    노출로 문서화(프로브 유계).
  - [x] **milestone-6a (2026-07-02): 1차 Futamura 사영** — pnix-hy
    futamura_ladder의 1차 사영을 Rust-lane 방식으로. mix.px v2:
    ① 정적 데이터 노드에 단일-lit 문자열·const attrs(cattrs) 추가, ② cattrs
    select 폴딩(정적 if가 죽은 가지를 mix하지 않아 결측 필드 select 도달 불가),
    ③ 정적 문자열 ==/!= 폴딩, ④ **lazy knot 없는 재귀 클로저**: 전원-람다
    let → recclo(param/body/binds/outer), apply마다 재귀 프레임 재구성
    (strict식 knot). 비-람다 let은 A4-보수 유지(m5 하위호환).
    결과: px로 쓴 객체언어 인터프리터(num/arg/add/mul)를 고정 prog에 특화 →
    **interpreter-free residual `(input * 3) + 4`** + 정확성 방정식(19) +
    전정적 붕괴. tower-check 39/39. 디버깅 기록: 인코딩된 str 리터럴 arm
    부재 → cond 미폴딩 → 죽은 가지의 결측 select 평가 — 손 인코딩 최소
    케이스로 분리 후 str arm 추가.
  - [x] **milestone-6b (2026-07-02): mix 자기-언어 커버리지** — pnix-hy
    POLY_MIX의 bapply 층을 우리 인코딩으로: const 리스트(clist)·residual
    list/attrs 노드·`builtins` 값 도달(gbuiltins→bfn 누적)·builtins 폴딩
    테이블(length/head/tail/map/filter/listToAttrs/getAttr/hasAttr/attrNames/
    타입테스트 5종/toString + `++`/`//` 폴딩). encode/decode로 정적 계산은
    실제 builtins에 위임(새 기계 없음). 판별 불가면 bapp/bfn residual —
    건전성 우선. builtins.isList 추가(Nix 표준, pnix-hy 대조, corpus 가드).
    reflect 확장(clist/cattrs/bapp/bfn/gbuiltins). tower-check 44/44 —
    정적 폴딩(42)·map 정적/동적(정확성 방정식)·filter·부분-동적 attrs select.
  - [x] **2차 사영 실험 (기록)**: `pnix-rs second-projection-experiment` —
    mix(mix_enc, ast=인터프리터 인코딩 static, senv dynamic) 시도. 결과:
    **1GB 스택으로도 63s 후 오버플로** — memo 없는 monovariant 자기적용의
    지수 unfold(recclo 참조마다 본문 재전개). pnix-hy가 POLY closure
    conversion + memo(st.specs)로 넘은 바로 그 벽의 실측. 실험 커맨드는
    게이트 밖 아티팩트로 유지.
  - [x] **milestone-6c (2026-07-02): polyvariant specializer** —
    `runtime/tower/poly_mix.px`: 전 단계 st = { specs; ctr } 스레딩, 동적
    인자 클로저 적용은 spec point로(키 적중 시 apply(var __sN, a)로 접힘);
    **pending 씨딩**으로 재귀 자기적용이 memo에 적중(m6b의 지수 벽 해법).
    etaBody(커링 residual), 재귀 let으로 spec 조립(추가 기계 불요).
    전제 확장 2건: px 깊은 `==`(Nix 의미, seed_deep_eq 가드) + **lid 라벨**
    (pnix-hy M8 — reify가 람다마다 결정적 pre-order 정수 라벨, poly 키가
    O(1); 다른 소비자는 무시). tower-check 46/46 — 패리티(`x*(2+3)`→`x*5`,
    spec 0) + **1차 사영이 spec 5개로 구조화**되어 residual 평가 19.
  - [x] **2차 사영 실험 2R (기록)**: poly 경유 mix(mono-mix, ast=interp 인코딩)
    — 구조 비교 병목은 lid로 제거했으나 **18분+ 미종결로 중단**. 다음 병목
    특정: ① strict recclo가 **apply마다 recFrame 재구성**(mono-mix 상위
    바인딩 ~40개 병합+listToAttrs — pnix-hy는 lazy knot으로 let당 1회),
    ② sig의 data-verbatim 깊은 비교(interp 인코딩 cattrs 조각). pnix-hy도
    같은 급 작업을 "host-tree-walker perf-bound"로 문서화 — 동일한 정직 기록.
  - [x] **sig 데이터 요약 (2026-07-02)**: sig의 data-verbatim을
    `builtins.toJSON (decode n)` 문자열로 — spec key 동등이 문자열 비교로.
    패리티 유지. 실험 3R: **여전히 7분+ 미종결** — 진단 정밀화: sig 자체가
    dynamic apply마다 decode+toJSON을 재계산(memo 조회보다 먼저!) + recFrame
    재구성과 곱셈. 병목은 비교가 아니라 **frame/sig의 per-apply 재계산**.
  - [x] **milestone-6d (2026-07-03): frame 정체성 재설계** — st에 frames
    gid 레지스트리: 전원-람다 let이 frame을 **1회** 구축·등록, recclo는
    binds/outer 복사 대신 **gid만** 참조(strict knot을 레지스트리 간접참조로
    폐쇄 — pnix-hy lazy env2의 strict 등가물). recclo spec 키 = { lid; gid }
    (frame 정체성이 env 시그니처를 대체 — hot path의 sig/recFrame 재계산
    소멸). 일반 클로저는 sig 키 유지(건전성). 패리티 46/46 유지.
  - [x] **2차 사영 실험 4R — 최종 판정 (기록)**: 재설계 후에도 **1h40m(CPU
    99분) 미종결로 중단**. 최종 진단: 알고리즘 벽(지수→memo 해소, 재계산→gid
    해소)이 아니라 **기판 상수** — px attrset 연산이 선형 스캔 Vec(진입
    ~40개 env에 var lookup 수백만 회)이며 pnix-hy가 완주한 것은 CPython
    dict(O(1) 해시)이기 때문. pnix-hy 문서화("host-tree-walker perf-bound")와
    동일 결론 + 원인 한 층 더 정밀. 사다리 의미론은 전부 증명됨(1차 사영
    spec 구조화 포함); 2차 사영 완주는 **기판 성능 작업**(px attrs 해시/정렬
    조회 — sacred runtime 변경이므로 proposals 절차)으로 승계.
  - [x] **milestone-6e (2026-07-03): 정렬 attrs + 이진 탐색** —
    proposals/0002 절차로 sacred runtime 변경: PxVal::Attrs를 이름-정렬
    불변식으로(유일 생성자 px_attrs가 수립, sort builtin의 subset-검증
    remove(min) 패턴), 조회 O(log n)·`//` 정렬 병합 O(n+m)·`==` zip O(n).
    관측 표면 불변(print/attrNames/toJSON은 원래 정렬 출력). 구현 중 게이트
    2회 적중: first-wins가 정렬-전 누적기에 이진 탐색(px-check 검출),
    Vec::swap subset 밖(substrate-check 검출). 전 게이트 green.
  - [x] **2차 사영 실험 5R/6R — 계측 필요 판정 (기록)**: 5R(정렬 attrs) 20분
    미종결, 6R(pnix-hy처럼 **축소 객체** — m6a-era mix_core.px 164줄, git
    이력에서 추출) 20분 미종결. 여섯 라운드로 식별 레버 소진(지수→memo,
    비교→lid, sig→toJSON, frame→gid, 조회→이진, 객체→축소). 1차 사영(~2s,
    spec 5)과의 격차가 초선형 — **spec 폭증 가설**이 유력하나 미관측.
    추가 최적화는 계측 선행: st.specs 증가 곡선/pmix 스텝 카운터 노출 후
    스케일 측정. (호스트 상수의 최심층 후보 = call-by-name 재평가 —
    px runtime held 표면 'thunk-memo laziness'와 접점.)
  - [x] **milestone-6f (2026-07-03): 2차 Futamura 사영 완주** — 계측이
    진범을 잡았다: fuel 스레딩(순수 px 계측 — st.fuel 소진 시 즉시 unwind로
    부분 상태 관측)으로 fuel=1이 14ms, fuel=100이 34초임을 측정 → 스텝당
    ~340ms의 정체는 **call-by-name 재귀 let의 지수 재평가**(`let l = f x;
    r = g l.st; in ...l.n...` 체인에서 참조마다 f 재실행). 1R~6R 전체
    미종결의 최상위 원인이었고 lid/gid/정렬은 그 아래 실재하던 상수 개선.
    **proposal 0003**: held 표면 thunk-memo laziness 개방 — PxFrame::Rec에
    바인딩별 memo(순수 언어라 name↔need 관측 동등; Rc<RefCell> 패턴은
    rs-meta interp 자기 증명). 효과: tower-check 82s→14s, **2차 사영
    1h40m+ → ~0.1s**. 마지막 조각 = **closure conversion at extraction**
    (finalize/finSpecs — 잔존 클로저 값을 eta-전개로 람다 구문화, pnix-hy
    M8 대응) 후 export를 run(pmix+finalize)으로.
    **게이트**: tower-check 47/47 — "compiler (22 specs) applied to prog ==
    direct mix" (컴파일러 정합 수용 기준). check aggregate all_ready.
  - [x] **3차 사영 시도 — 스케일 판정 (2026-07-03, 기록)**: cogen = poly의
    자기적용(ast/senv/st 전부 dynamic — pnix-hy build_cogen 대응) 실험.
    무계측 실행 6h17m 무출력 → 중단 후 fuel 곡선: fuel 10k에서 specs 170/
    2.2s, 40k에서 specs 687/25s — **spec 생성이 수렴 기미 없이 선형 증가 +
    시간 초선형**. 진단: 2차와 달리 자기적용에서는 closure spec 키의
    sig(senv) 변형이 대량 발생하는 **polyvariance 폭발** — 해법은 spec 키
    통합(coarsening)/모노바리언트 강등 휴리스틱 등 부분평가 연구 지평.
    실험 커맨드(third-projection-experiment, fuel 곡선 내장) 게이트 밖 유지.
    m5 자기생성 수용 기준은 판정자로 계속 대기.
  - [x] **m7 coarsening 시도 — spec 키 자유변수 제한 (2026-07-03)**: closure
    spec 키의 sig(frame)를 **본문의 자유변수로 제한**(fvNode가 인코딩 노드의
    자유변수 계산, lid별 캐시; sigOf가 fv 이름에만 국한). 건전성: 본문이
    참조 못 하는 env 엔트리는 특화에 영향 없음 → key coarsening이 의미 보존.
    parity 유지(tower-check 47/47, 2차 사영 그대로). **3차 사영 fuel 곡선
    재측정: 10k specs 170→163, 40k 687→655 (~5% 감소, 노이즈 아님)** —
    건전한 개선이나 **폴리바리언스의 레버는 아님**. 판정: 자기적용의 spec
    폭발은 env-노이즈가 아니라 **의미적**(각 재귀 특화점이 실제로 다름) —
    부분평가의 known-hard(폴리바리언트 specializer의 자기적용). fv-제한은
    lane에 남김(무관 env 있는 케이스엔 실질 이득, 건전).
  - [x] **m8 BTA 분석 facet (2026-07-03)**: pnix-hy처럼 BTA를 온라인
    specializer 수정이 아닌 **오프라인 예측 분석기**로 구현(src/bta.rs, Rust
    host-side — specialize.rs 자유변수 분석의 연장). monovariant, 보수적
    (미지 = Dynamic), static/dynamic 분류 + if-조건 binding-time 수집.
    **핵심 = mix.px와의 교차검증**: static if-조건 ⟺ mix 폴딩, dynamic if-조건
    ⟺ mix residualize. bta-check 6/6, aggregate 16 reports 편입.
    **정직한 발견**: BTA는 mix 폴딩의 **상한(upper bound)** — `let b = 2<3;
    in if b ...`에서 BTA는 Static 예측하나 mix의 A4-보수 let 규칙은
    residualize(폴딩 방향만 건전: mix 폴딩 ⟹ BTA Static, 역은 성립 안 함).
    이 gap을 6번째 검사로 명시 게이트화. capabilities_doc 사실 갱신(floats/
    toJSON/laziness 지원 반영, builtins 23종).
  - [x] **m9 Jones-optimality 게이트 (2026-07-03, deep-research 근거)**: "specializer가
    해석 계층을 실제 제거했는가"의 측정 가능·falsifiable 게이트. 조작적 정의
    (Glück; JGS strict form): 인터프리터에 **안 쓰는 dispatch 분기(sub/neg)를
    추가(bloat)**해도 residual이 AST 동일하면, residual이 인터프리터가 아니라
    프로그램에만 의존 ⟹ 해석 계층 완전 제거. jones-check 4/4: bloat 불변
    (`(input*3)+4`)·인터프리터-free·프로그램별 상이·정확성. deep-research
    finding [5] 확증: **fv-제한 등 subject BTA는 Jones-optimality를 못 올리는
    강도 천장** — 그러니 다음은 "더 coarsen"이 아니라 이 게이트로 tower 강도를
    falsifiable하게 박는 것. aggregate 17 reports.
  - 잔여 held(m10+, deep-research 로드맵): ① **손으로 쓴 cogen**(Leuschel —
    3차 사영을 자기적용 없이 우회, offline BTA의 얇은 확장; 종결성 = local+
    global 2 의무 via size-change + mgg generalization) ② **잘-타입된 residual
    게이트**(Brown&Palsberg POPL'18 — px→Rust residual이 rs-meta typeck로 구성상
    타입-정합; Rust 정적 강점, 동적 Lisp 불가) ③ full S=L. docs/research/
    2026-07-03-metacircular-frontier.md 참조.
- [x] **P12 action** (`action.py` → `src/action.rs`): 새 기계 없이 기존 표면을
  하나의 verdict로 — 평가기/mirror/gate/백업 아무것도 새로 소유하지 않는다.
  - [x] `pub struct ActionVerdict { schema: "pnix-rs.action.v0", gate_allowed:
    bool, mirror_status: String, ir_sha256: String, witness: gate::Witness,
    allowed: bool /* = gate_allowed && mirror_status=="lossless" */ }`
  - [x] `pub fn action_check(px_source: &str, granted: &[String])
    -> Result<ActionVerdict, String>` — gate::gate_check + mirror::mirror_run +
    ir::ir_of + gate::eval_witness 조합만.
  - [x] `action-check`: ① corpus 프로브 allowed ② uncertain 프로브(builtins
    탈출) → allowed=false(gate에서) ③ opaque 값 프로브 → allowed=false
    (mirror held에서) ④ verdict 렌더 결정성. 기대: 4/4.
  - CLI `action -c|-f`, `action-check`.
  - 완료(2026-07-02): action-check 4/4 (승인/gate-거부/mirror-held 거부/결정성).
- [x] **P13 cross-host** (export 우선, 비교는 파일 대 파일):
  - [x] `pnix-rs export-oracles` → `proof/oracles-rs.tsv`: corpus별
    `name\tvalue_canonical\tvalue_sha256\tir_sha256` + 헤더에 schema/생성
    커맨드. provenance 주석: 의미값은 pnix-clj `oracles.edn`(rust-grounded,
    ~/pnix-old f5ce48f 캡처)과 S2에서 수동 대조됨.
  - [x] `cross-host-check`: ① export 재생성이 디스크 파일과 byte-identical
    (drift 게이트) ② witness 13필드 스키마 가드(WITNESS_FIELDS 상수와 렌더
    일치) ③ corpus expected(px_corpus)가 export의 value_canonical과 일치.
    기대: 3/3.
  - [ ] **held(경계 기록)**: pnix-clj/pnix-hy 쪽에 같은 TSV export가 생기기
    전까지 파일 대 파일 자동 비교는 held. EDN/Python 파싱으로 우회하지 말 것
    (Python/Hy 불가촉 + zero-dep). 자매 lane에 제안할 TSV 스키마가 위 포맷.
  - CLI `export-oracles`, `cross-host-check`.
  - 완료(2026-07-02): cross-host-check 3/3 (export drift 게이트 / witness 13필드
    렌더 순서 동결 검증 / corpus 기대치-export 일치). 자매 lane TSV 대 TSV 자동
    비교는 held 유지(위 항목).
- [x] runtime 확장(2026-07-02, 수요=P13 corpus 완성): **toJSON(c06) + 동적
  attrset 키(c10) 개방** — toJSON은 pnix-hy json.dumps(sort_keys+compact) 의미
  동형 in-house 직렬화, 동적 키는 파서 desugar→listToAttrs(m3b)로 AST 무변경
  (동적 중복 first-wins — Nix eval-error, divergence 기록). oracle 대조 일치
  (c10 total=240; 처음 본 210은 c03 값 오독). substrate harness에 c06/c10
  내장 — rs-meta interp==rustc==native 3-way 증명. invariance corpus **9/10
  native**, tower ORIGINAL 인코딩 9종(37/37).
  **c01(floats) 2026-07-02 완성 — cross-lane 순서 실증**: rs-meta가 먼저 f64
  subset 슬라이스를 얻고(lexer/ast/typeck/interp/emit/sig, corpus 310, 전
  self-host 체인 green), 그 위에서 px.rs Float(f64)를 구현(float×float 연산,
  {:?} 캐논 출력 = pnix-hy repr 동형, int↔float 승격 없음 — divergence 기록).
  c01 == oracle 정확 일치. substrate harness에 c01 내장(f64 px 코드가 rs-meta
  interp==rustc==native 3-way 증명). **invariance corpus 10/10 native.**
  string `+`/bool 연산/`rec`/`with`/laziness는 여전히 수요 대기.

### 4.2 문화 (pnix-hy에서 그대로 가져오는 규칙)

- [x] **SCOPE_LOCK.md** 수립(저장소 루트 `~/pnix-rs/pnix-rs/SCOPE_LOCK.md`):
  ① source-of-truth 절(main 기준) ② scope-relative 완성 문구 강제("complete
  w.r.t. the stated Rust↔pnix projection scope"만 허용; "전체 완성" 문구 금지)
  ③ 대원칙: 의도적 placeholder(각 P의 "명시 미주장"/held 블록)를 미구현으로
  재해석해 구현 금지 ④ 새 기능은 `docs/proposals/NNNN-*.md`로 시작(P6 v1이
  0001) ⑤ 걷지 않는 길 블록(§4.0) 참조 링크.
- [x] `docs/proposals/` 생성 + `0001-rust-ast-projection.md`(P6 v1: rs-meta
  ast-canonical 필요성/스키마/경계 — rs-meta는 pnix 무관 유지).
- [x] `docs/CAPABILITIES.md` 능력 인덱스 — P7에서 생성 자동화 + drift 게이트. (2026-07-02)
- [x] 감사 문화: adversarial 재검증 1회 완료 — **발견 4건 전부 수정**
  (F1 중복 let shadow 의미 버그(A4 위반, HIGH) / F2 중복 attrset 키 수용 /
  F3 interop 밖 fs 4곳 / F4 incremental 중복 이름 무방비).
  `docs/audits/2026-07-02-ladder-closure-audit.md`. 수정 후 15 reports
  all_ready 재확인. (2026-07-02)
- 이미 적용 중: 두 번째 평가기/mirror/gate 금지, 정직 경계(§5), witness 스키마
  무단 변경 금지(P13), zero-dep, Python/Hy 불가촉.

## 5. 정직 경계

- runtime 기판 미지원(정직 거부): floats(c01), `builtins.toJSON`(c06), 중첩
  attrset-키 interpolation(c10), string `+`, bool `&& || !`, `?`, `rec`, `with`,
  paths. **mirror/gate 유기체가 수요를 만들 때만 확장.**
- `builtins.sort`는 selection sort(비안정) — Nix는 안정 정렬. corpus 값이 전부
  distinct라 현재 관측 불가; 안정성 요구가 생기면 명시 수정.
- canonical print 형식(`{ k = v; }` 정렬)은 이 lane의 잠정 canonical이며,
  cross-host(pnix-clj/pnix-hy) canonical과의 정합은 M7에서 확정.
- substrate-check는 px 엔진의 rs-meta-해석 동등 증거이지, px 의미론의 형식증명이
  아니다.

## 6. 참고

- `~/pnix-clj/pnix-clj/todo.md` — 자매 lane 아키텍처("clj-meta backed"), 금지
  규칙(두 번째 평가기 금지 등).
- `~/pnix-clj/pnix-clj/resources/pnix_clj/rust_grounded/invariance_corpus/` —
  cross-host .px corpus 원본 (c05/c09 vendored, 나머지는 로드맵).
- `~/pnix-hy/pnix-hy/todo.md` — px 의미 감사(A4 재귀 let 등), SCOPE LOCK 문화.
- `../rs-meta/todo.md` — substrate의 stage ladder / evaluated subset 현황.

## Host-language import of pnix product library (user intent, 2026-08-13)

**Canonical doctrine:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md)  
**HM mirror:** `~/dot-nix/dev/PNIX-HOSTS.md`

Context from home-manager (`dot-nix`) integration:

- `pnix-<host>-pnix` = pnix-language surface (REPL/eval of `.px`) on this host.
- `pnix-<host>-<lang>` = host-language interpreter/compiler used for day-to-day
  host development.
- Libraries produced by the **pnix product half** of this host are **host-
  language libraries**: they must load in *this* host language. They are **not**
  assumed to be portable common bytecode for other hosts.
- A future **common portable `.px` library** track (historical pnix-meta style)
  is deferred; do not block host-local import work on that.

dot-nix can only set PATH/env (classpath, PYTHONPATH, link paths, NODE_PATH,
DLL HintPath). Anything that requires a real packaging format is product work
below.


### rs — status (2026-08-14)

**Landed:**

1. Dual-axis docs: `HOST_DEV_ENV.md`, host `CLAUDE.md` / `README.md`.
2. Flake `packages.pnix-rs-library` + C header `pnix_rs.h`.
3. Host-main env: `PNIX_RS_LIB_DIR` / `PNIX_RS_INCLUDE_DIR` / `PNIX_RS_RUNTIME`
   (HM `pnix-rs-rs` wraps cargo/rustc).
4. Host-language `.px` import: `pnix_rs::eval_file` + C `pnix_rs_eval`.

**Still open:**

1. ~~Cargo cookbook~~ → `docs/CARGO_HOST_IMPORT.md` + flake `pnix-rs-refs`.
2. Optional: crates.io / git package (`publish = false` today).
3. Stable C ABI versioning for the cdylib (semver + header).

## Post host-env plan (2026-08-14) — plan only

Host dual-axis + library import for this host is **closed** for day-to-day.
Optional P2/P3 and residual product work: monorepo `HOST_ENV_P2_P3.md`.
Do not reopen host-env packaging as a primary gate unless env contracts break.
