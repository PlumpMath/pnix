# pnix-cljs PLANS

목적: 아직 확정 안 된 미래 설계 방향 — 방향 제시용 메모이지 착수가
결정된 작업이 아니다. 지금 당장 누가 집어서 바로 진행할 수 있는 작업은
[`TODO.md`](TODO.md)에 둔다. (구 `todo.md`의 "미래 아이디어" 절과 "실제
백로그" 절을 2026-08-20에 그대로 옮겨왔다 — 둘 다 착수 확정은 안 됐지만
방향은 있는 항목이라는 점에서 동일한 성격.)

## 미래 아이디어 — pnixMounts / unsafeGetAttrPos 통일 (아직 예정 없음, 2026-08-19)

지금은 만들지 않는다. 기본 언어 기능(5개 호스트가 실제로 똑같이 동작해야 하는
핵심 부분)이 production 수준으로 완전히 갖춰지기 전까지는, 여기 적힌 건 전부
방향 제시용 메모일 뿐 확정된 설계가 아니다. 나중에 필요해지면 아래 단서를
참고해서 5개 호스트를 통일시키고 어디에 응용할지 결정한다.

### unsafeGetAttrPos

- Nix 실제 스펙: 속성이 정의된 위치를 `{ file; line; column; }` 모양으로
  돌려준다.
- 2026-08-19 5개 호스트 감사 결과:
  - hy: `{file; line; column;}` — Nix 스펙과 일치하는 모양. 나중에 통일할
    때 이게 목표 모양일 가능성이 높음.
  - clj: `{start; end; span;}`(바이트 오프셋) — clj 자신의 todo.md에 이미
    "line/column 추적 인프라가 생길 때까지의 임시방편"이라고 적어둠.
  - clr: 항상 `null` — 위치 추적 자체를 아직 안 함.
  - **cljs (여기)**: `builtins` 레지스트리에 이름은 등록돼있는데(`invoke-builtin`
    의 `case`에 `:unsafeGetAttrPos` 분기가 없어서) 실제로 호출하면
    "not-callable" 에러 — 이름만 있고 구현이 없는 죽은 항목.
  - rs: 아예 등록 안 됨.
- 방향 아이디어(확정 아님): line/column 추적은 이 빌트인 하나만을 위한 게
  아니라 에러 메시지 품질 전반에 같이 쓸 수 있는 인프라다 — 파싱/평가 에러가
  지금은 대부분 바이트 오프셋(`:offset`)만 주는데, 실제 Nix처럼
  "파일:줄:컬럼"으로 보여주면 디버깅이 훨씬 편해진다. 이 하나만 따로
  만들기보다 에러 위치 표시 개선 작업과 묶는 게 나을 수 있다. cljs 자신은
  현재 "not-callable"로 죽어있는 등록만 빨리 정리(진짜 구현이든, 최소한
  `null` 스텁으로든)해도 다른 호스트들과 최소한 "에러는 안 남" 수준으로는
  맞출 수 있어 보인다 — 다만 그것도 지금 단계에서 서두를 필요는 없다.

### pnixMounts

- Nix 실제 빌트인 아님 — pnix-clj/pnix-clr 소스에 `:nix-builtin? false`,
  `:policy :non-faithful-extension-not-nix-coverage`로 명시돼있다. Nix 호환
  주장에서 의도적으로 제외된 pnix 자체 아이디어다. cljs 레지스트리에는
  이름조차 없다.
- 이름과 프로젝트 전체 설계 방향(순수 평가기는 기본적으로 실제 OS
  파일시스템/store에 손을 못 댐 — `storePath`도, 원래 `import`도 전부 이
  원칙 때문에 막혀있다가 2026-08-19에 `import`만 실제 파일 읽기로 확장됨)으로
  미루어 짐작하면(확정 아님, 순전히 추측): "순수 평가기에게 실제 OS
  파일시스템 대신 미리 정해둔 가상 경로 목록(mount)만 제한적으로 보여주는
  기능"일 가능성이 있다.
- pnix-clj의 `import`가 `*import-modules*`(경로 문자열 -> pnix 소스 텍스트로
  된 순수 인메모리 맵)로 정확히 이 패턴을 이미 증명해뒀다. cljs는 이미
  `import`가 상대/절대 경로 둘 다 실제 파일시스템으로 잘 동작하고 있으니
  (2026-08-19 교차검증 + 절대경로 렉서 수정으로 확인), 인메모리 mount
  계층은 그 위에 얹는 추가 옵션 정도로 생각하면 될 것 같다.
- 방향 아이디어(확정 아님): 나중에 필요해지면 `import`뿐 아니라
  `pathExists`/`readFile`/`readDir` 같은 다른 파일시스템 관련 빌트인들도
  똑같은 "인메모리 mount 맵" 패턴으로 확장하고, `pnixMounts`는 그 맵을
  읽기 전용으로 들여다보는 조회용 빌트인으로 만드는 게 자연스러워 보인다.
  정확한 시그니처/의미는 아직 미정 — 실제로 필요한 상황(재현 가능한 테스트,
  hermetic 빌드 등)이 생겼을 때 다시 설계해야 한다.

**중요**: 위 두 항목은 전부 방향 제시용 메모다. 5개 호스트를 실제로
통일시키는 작업은 기본 언어 기능이 production 수준으로 완전히 갖춰진 다음,
필요에 의해 결정한다.

## (해결됨) CAPABILITIES.md 자동 생성기 — 2026-08-20

이전엔 이 자리에 "clj/hy/rs 3개 호스트엔 있는데 이 호스트와 pnix-clr만
없다"는 실제 백로그 항목이 있었다. 2026-08-20에 pnix-cljs 쪽은 해결됨:
`src/pnix_cljs/capabilities.cljs`가 `evaluator/builtins-value`를 직접
introspect해서 `docs/CAPABILITIES.md`를 생성하고, `pnix-cljs.main`의
`capabilities`/`capabilities-check` 서브커맨드 + `bin/pnix-cljs-gate`에
게이트로 박혀 있다. 자세한 내용은 [`IMPLEMENTATION.md`](IMPLEMENTATION.md)
§1·§7과 [`CAPABILITIES.md`](CAPABILITIES.md) 자체 참고. (pnix-clr은 별도
트리라 이 항목과 무관 — 거기는 그쪽 문서에서 다룬다.)
