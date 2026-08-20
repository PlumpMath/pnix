# pnix-rs CAPABILITIES — 능력 인덱스 (중복개발 방지 조회)

> 생성: `pnix-rs capabilities > docs/CAPABILITIES.md` — 손 편집 금지.
> drift 게이트: `pnix-rs capabilities-check`.

## CLI 명령

| 명령 | 목적 | schema |
|---|---|---|
| px-eval -c\|-f | .px 평가 → canonical 출력 | - |
| px-check | corpus가 기대 canonical과 일치 | - |
| mirror -c\|-f / mirror-check | singleton mirror facet + roundtrip 어휘 | pnix-rs.mirror.v0 |
| stage -c\|-f / stage-check | px-stage1..5 + closure | pnix-rs.stage.v0 |
| ir -c\|-f / ir-check | canonical IR + ir_sha256 + identity sharing | pnix-rs.ir.v0 |
| gate -c\|-f / gate-check | purity/effect-class admission | pnix-rs.gate-check.v0 |
| witness -c\|-f | eval witness (13필드 공유 스키마) | pnix-rs.witness.v0 |
| interop-check | host-call/file 경계 fail-closed + witness | pnix-rs.witness.v0 |
| rust-mirror -c\|-f / rust-mirror-check | 값 축: px값→Rust 3-way / AST 축: sig-tree(v1a)/typed(v2)/program 역재구성(v3)/struct·impl(v4) | pnix-rs.rust-mirror.v0 |
| substrate-check | rs-meta interp == rustc == native 3-way | - |
| check | all_ready 집계(clean-process replay) + receipt | pnix-rs.check-receipt.v0 |
| capabilities / capabilities-check | 이 문서 생성 / drift 게이트 | - |

## 모듈

| 파일 | 역할 |
|---|---|
| src/px.rs | sacred px runtime: lexer/parser/eval/print/emit/normalize (rs-meta subset 안) |
| src/mirror.rs | singleton mirror_run + roundtrip 어휘 |
| src/stage.rs | pnix runtime stage ladder |
| src/ir.rs | canonical IR (직접평가가능, content-addressed) |
| src/sha256.rs | in-house SHA-256 (FIPS self-test) |
| src/gate.rs | purity/capability gate + 13필드 witness |
| src/interop.rs | host-call/file 유일 통로 (capability 게이트) |
| src/rust_mirror.rs | Rust↔px projection v0 (값 축) |

## px 표면 (지원)

checked int/float(혼합 승격·Nix 반올림)/bool/string(+`${}` 보간·raw bytes)/list(+`++`)/
attrset(+`//`, `.name`, `?`, 깊은 identity-aware `==`)/재귀 let·rec(call-by-need)/
lambda+juxtaposition/if-then-else/with/string `+`/bool `&& || !`/산술·비교/`#` 주석/
MD5·SHA1·SHA256·SHA512 `hashString`/Nix 호환 source-float grammar.

등록된 public builtins 192종(함수 + 값 상수 + 재귀 `builtins` 필드; presence inventory):
toString stringLength concatStringsSep substring length map filter all any isFunction isNull isFloat typeOf baseNameOf dirOf abort foldl genList foldl' attrNames hasAttr sort head tail elemAt elem listToAttrs removeAttrs replaceStrings getAttr isAttrs isInt isBool isString isList toJSON fromJSON throw deepSeq addErrorContext hashString concatMap concatLists match split sin cos tan sqrt exp ln log abs ceil floor pow max min mod functionArgs add sub mul div lessThan bitAnd bitOr bitXor attrValues mapAttrs catAttrs intersectAttrs zipAttrsWith groupBy partition seq splitVersion compareVersions break parseDrvName toPath unsafeDiscardOutputDependency unsafeDiscardStringContext hasContext getContext appendContext derivation derivationStrict tryEval isPath trace toXML toFile readFile readDir pathExists fetchurl fetchTarball fetchGit last init flatten foldr getAttrFromPath hasAttrByPath attrByPath getAttrFromPathOr filterAttrs filterAttrsRecursive mapAttrsRecursive concatMapStringsSep removePrefix removeSuffix hasPrefix hasSuffix splitString toLower toUpper boolToString implies optional optionals optionalAttrs when id const flip pipe fix range sum product recursiveUpdate updateManyAttrs getName getVersion unique intersectLists subtractLists zipLists zipListsWith warn assertMsg cons append drop take find findFirst reverseList replicate zip zipAttrs keys values mapAttrsToList merge genAttrs foldlAttrs genericClosure nameValuePair concatStrings concatMapStrings stringToCharacters hasInfix optionalString imap0 imap1 toInt placeholder storePath getEnv and or not eq lt le gt ge neg get set atan2 mapAttrs' true false null langVersion nixVersion storeDir builtins

presence는 호출 parity 주장이 아니다(max/min은 구현됨 — 이전 텍스트가 부정확했음;
functionArgs는 2026-08-19 pattern-lambda desugar-shape 인식으로 구현됨). 2026-08-20:
이전까지 fail-closed HELD였던 10개 확장 수학 빌트인 `sin cos tan sqrt exp ln log abs
pow mod`도 실구현됨 — 다른 4개 호스트(clj/clr/cljs/hy)가 이미 갖고 있던 4/5 합의
사례였고, 순수 산술(Newton's method/Taylor 급수)로 구현했다: rs-meta의 인터프리트
Rust 부분집합은 f64 메서드 디스패치가 없어서 `.sin()`/`.sqrt()`/`.exp()`/`.ln()` 같은
표준 라이브러리 호출을 못 쓰기 때문(`px_bit_op`의 bit-by-bit 구현과 같은 이유). 같은
변경에서 `atan2`(오라클: pnix-hy)와 `mapAttrs'`(오라클: pnix-clj)도 신규 추가됨.

## px 표면 (명시 미지원 — held)

path literal/string-context/store 값 모델, URI literal, 중첩 동적 attr 경로,
POSIX ERE 전체 정합, JSON float exponent canonicalization,
비유한 float의 source-roundtrip 출력, Nix 전체 builtin 표면과 hash context 규칙

## 스키마

pnix-rs.mirror.v0 · pnix-rs.stage.v0 · pnix-rs.ir.v0 · pnix-rs.gate-check.v0 ·
pnix-rs.witness.v0(13필드 동결) · pnix-rs.rust-mirror.v0 · pnix-rs.check-receipt.v0

## 어휘 (동결)

roundtrip: lossless | lossy-ok | held | rejected
effects: file-read | file-write | host-call | import | network


## 게이트 레지스트리 — 이미 구현됨 (각 게이트가 증명하는 것)

> 중복개발 방지: 새 기능 전에 이 표와 `docs/proposals/`를 grep.
> 상태 = DONE (모두 `pnix-rs check` all_ready 집계에 포함).

| 게이트 | 증명 | 상태 |
|---|---|---|
| px-check | seed corpus가 기대 canonical로 평가 (부동/toJSON/동적키/깊은== 포함) | DONE |
| mirror-check | corpus mirror lossless (emit 고정점 + 값 일치) | DONE |
| stage-check | px-stage1..5 + closure 런타임 사다리 닫힘 | DONE |
| ir-check | sha256 벡터 + IR 증명 + identity sharing (바인딩 순서 무관) | DONE |
| gate-check | corpus 순수 admission; 미지 builtin fail-closed; witness | DONE |
| interop-check | host-call 경계: grant 없이 거부 + witness | DONE |
| rust-mirror-check | 값 축 px→Rust 3-way + AST 축 v1a~v7(mirror_probe 전량 + 제네릭 fn 왕복, AST 동일+rustc 정합) | DONE |
| specialize-check | A4-건전 부분평가: 폐쇄식 fold, 동적 let held | DONE |
| incremental-check | 알파 불변 + SCC + realisation 컷오프 + demand-driven 변경 전파(salsa/adapton 최소 재계산) | DONE |
| compartment-check | SES식 격리: 자기 env/모듈, intrinsic 공유 | DONE |
| tower-check | reify/reflect + px 자기해석기 == 네이티브 + 1·2차 Futamura 사영 | DONE |
| bta-check | 오프라인 BTA static/dynamic + mix 교차검증(폴딩 상한) | DONE |
| jones-check | Jones-optimality: 인터프리터 bloat에도 residual 불변(해석 계층 제거) | DONE |
| welltyped-check | px→Rust residual이 rs-meta 플로어 typeck로 well-typed (구성상 타입-정합; Rust 정적 강점) | DONE |
| certify-check | proof-carrying residual: 특화 residual이 소스와 입력 배터리 전체 동등(재검증 인증서, 증명기 없이) | DONE |
| cogen-check | 손으로 쓴 cogen(generating extension) — 어떤 객체 프로그램이든 컴파일된 residual 생성 == 해석 (자기적용 없이) | DONE |
| attest-check | typed attestation(in-toto/SLSA식) — witness에 predicate 타입 + subject; 불일치 predicate 거부 | DONE |
| reflect-tower-check | finite reflective tower(3-Lisp): reify/reflect가 인코딩을 다시 인코딩해도 2-레벨 coherent + 메타-레벨 의미 투명 | DONE |
| verifying-cache-check | verifying cache: 캐시 히트 시 재검증(재실행 대조) — 오염된 realisation 감지(이빨) | DONE |
| phase-check | phase separation: 특화 residual의 자유변수 = 정확히 동적 변수(정적 완전 소진·동적 완전 보존) | DONE |
| assumption-check | assumed specialization: residual의 정적 가정이 유효할 때만 재사용, 가정 변하면 stale 감지→재특화 | DONE |
| ir-diff-check | ir-diff: canonical IR 의미 diff — reorder는 동일(meaning-preserving), 의미 변경은 국소화 | DONE |
| attenuate-check | capability attenuation(SES): grant→감쇠(엄격히 약화)→회수; 감쇠는 되돌릴 수 없음(재확대 불가) | DONE |
| explain-check | unified explain: 한 호출로 value+purity+effects+ir+mirror+witness 통합; 개별 facet과 정합 | DONE |
| engine-verdict-check | peer-engine adapter: rs-meta Rust TV -> 공통 .px engine verdict envelope(pnix.engine.verdict.v0); verdict가 유효 px + TV->status 매핑 정합 | DONE |
| engine-artifact-check | native artifact receipt를 .px 봉투(pnix.engine.artifact.v0)로 export; 재현 가능 + 유효 px (stage8-repro per-program) | DONE |
| engine-request-check | 요청/응답 프로토콜: .px request 봉투(pnix.engine.request.v0)를 phase로 디스패치→verdict/artifact/profile 응답 | DONE |
| engine-attestation-check | 엔진 신뢰 증명(pnix.engine.attestation.v0): interp==rustc TV 커버리지(positive+negative corpus) + substrate 3-way | DONE |
| engine-verify-check | 검증 가능 verdict: witness_id를 증거 필드에서 재계산해 일치 확인(변조 감지); 신뢰 없이 검증 | DONE |
| engine-batch-check | 배치 오케스트레이션: Rust 소스 리스트 -> verdict 매니페스트(pnix.engine.batch.v0) + accepted/held/rejected 카운트 | DONE |
| action-check | 단일 verdict = gate+mirror+ir+witness (admitted/refused/결정성) | DONE |
| cross-host-check | oracles TSV export drift 게이트 + witness 스키마 동결 | DONE |
| substrate-check | rs-meta interp == rustc == native 3-way (rs-meta 의존 증명) | DONE |
| capabilities-check | 이 생성 인덱스가 커밋된 문서와 일치 (docs drift + 레지스트리) | DONE |
| registry-check | 레지스트리 <-> 실제 게이트/proposal 정합 (누락·dangling 방지) | DONE |

## 로드맵 — 새로 구현할 것 (held, 순위·근거·모듈·proposal)

> 근거: docs/research/2026-07-03-metacircular-frontier.md (deep-research, 6 findings high-confidence).
> 각 항목은 proposal로 등록됨(중복·누락 방지). external = 외부 lane 대기.

| # | 능력 | 성격 | 모듈 | proposal |
|---|---|---|---|---|
| 1 | full 3차 사영 — feature-rich specialiser 자기적용 (bounded cogen은 DONE; full은 연구 지평) | 연구 프론티어(Leuschel) | tower/bta | 0004 |
| 3 | P6 v8 — 제네릭 struct/impl<T> + 트레이트 solving projection | 기계적 확장 | rust_mirror | 0001 |
| 4 | runtime 표면 tail — path/context/store 값 · URI literal · 중첩 동적 attr 경로 · JSON float 표기 · regex 정합 | 기계적 확장 | px | 0006 |
| 5 | full S=L (전 표면 poly) + stage-polymorphic — 연구 지평 | 연구 프론티어 | tower | 0007 |
| 6 | research open: step-level bisimulation · N-레벨 collapsing tower [incremental·proof-carrying residual·finite reflective tower는 DONE] | 후속 리서치 | check/tower | 0007 |

## proposals (등록된 설계/경계)

0001 rust-ast-projection(v1a~v7 DONE, v8 held) · 0002 px-attrs-sorted-lookup(DONE) ·
0003 px-call-by-need(DONE) · 0004 hand-written-cogen(held) ·
0005 well-typed-residual-gate(held) · 0006 runtime-surface-on-demand(held) ·
0007 research-frontier-index(open: TV/certified-compilation, reflective towers,
content-addressed incremental)
